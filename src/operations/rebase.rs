//! Agent heartbeat: update clones from their pristine's latest state.
//! Strategy: fast-forward if possible, otherwise attempt merge via subprocess in a copy.

use log::{debug, info, warn};
use std::process::Command;

use crate::config::Config;
use crate::error::Result;
use crate::metadata::Metadata;

/// Heartbeat update: for each clone of a repo, fetch from pristine and update.
/// Fast-forwards where possible. For diverged clones, attempts merge in a copy.
/// Sets `upstream_conflicts` flag on clones that can't be cleanly merged.
pub fn heartbeat_update_clones(repo_name: &str, config: &Config) -> Result<()> {
    let pristine_path = config.pristines_dir.join(repo_name);
    if !pristine_path.exists() {
        debug!("heartbeat: skipping '{}' (no pristine)", repo_name);
        return Ok(());
    }

    let mut metadata = Metadata::load(repo_name, config)?;
    let mut changed = false;

    for clone_entry in &mut metadata.clones {
        if !clone_entry.path.exists() {
            debug!(
                "heartbeat: clone '{}' path missing, skipping",
                clone_entry.name
            );
            continue;
        }

        // Determine remote name: "pristine" for new clones, "origin" for legacy
        let clone_path_str = clone_entry.path.to_string_lossy().to_string();
        let remote_name = {
            let check = Command::new("git")
                .args(["-C", &clone_path_str, "remote"])
                .output();
            match check {
                Ok(output) if output.status.success() => {
                    let remotes = String::from_utf8_lossy(&output.stdout);
                    if remotes.lines().any(|l| l.trim() == "pristine") {
                        "pristine"
                    } else {
                        "origin"
                    }
                }
                _ => "origin",
            }
        };

        // Fetch from pristine remote (local, fast)
        let fetch_result = Command::new("git")
            .args(["-C", &clone_path_str, "fetch", remote_name])
            .output();

        match fetch_result {
            Ok(output) if !output.status.success() => {
                let stderr = String::from_utf8_lossy(&output.stderr);
                warn!(
                    "heartbeat: fetch failed for clone '{}': {}",
                    clone_entry.name,
                    stderr.trim()
                );
                continue;
            }
            Err(e) => {
                warn!(
                    "heartbeat: fetch failed for clone '{}': {}",
                    clone_entry.name, e
                );
                continue;
            }
            _ => {}
        }

        // Get current branch name
        let branch_output = Command::new("git")
            .args(["-C", &clone_path_str, "rev-parse", "--abbrev-ref", "HEAD"])
            .output();

        let branch_name = match branch_output {
            Ok(output) if output.status.success() => {
                String::from_utf8_lossy(&output.stdout).trim().to_string()
            }
            _ => continue,
        };

        if branch_name == "HEAD" {
            // Detached HEAD, skip
            continue;
        }

        let upstream_ref = format!("{}/{}", remote_name, branch_name);

        // Check if behind using explicit ref instead of @{upstream}
        let status_output = Command::new("git")
            .args([
                "-C",
                &clone_path_str,
                "rev-list",
                "--left-right",
                "--count",
                &format!("HEAD...{}", upstream_ref),
            ])
            .output();

        let (ahead, behind) = match status_output {
            Ok(output) if output.status.success() => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let parts: Vec<&str> = stdout.trim().split('\t').collect();
                if parts.len() == 2 {
                    let a = parts[0].parse::<usize>().unwrap_or(0);
                    let b = parts[1].parse::<usize>().unwrap_or(0);
                    (a, b)
                } else {
                    continue;
                }
            }
            _ => continue,
        };

        if behind == 0 {
            debug!("heartbeat: clone '{}' is up-to-date", clone_entry.name);
            continue;
        }

        if ahead == 0 {
            // Can fast-forward
            debug!("heartbeat: fast-forwarding clone '{}'", clone_entry.name);
            let ff = Command::new("git")
                .args(["-C", &clone_path_str, "merge", "--ff-only", &upstream_ref])
                .output();

            match ff {
                Ok(output) if output.status.success() => {
                    info!("heartbeat: fast-forwarded clone '{}'", clone_entry.name);
                    clone_entry.upstream_conflicts = false;
                    changed = true;
                }
                _ => {
                    warn!(
                        "heartbeat: fast-forward failed for clone '{}'",
                        clone_entry.name
                    );
                }
            }
        } else {
            // Diverged — attempt merge in a temporary copy
            debug!(
                "heartbeat: clone '{}' has diverged (ahead={}, behind={}), attempting merge",
                clone_entry.name, ahead, behind
            );

            let tmp_path = clone_entry.path.with_extension("merge-tmp");
            if tmp_path.exists() {
                let _ = std::fs::remove_dir_all(&tmp_path);
            }

            // Copy clone to tmp
            let cp_result = Command::new("cp")
                .args(["-a", &clone_path_str, &tmp_path.to_string_lossy()])
                .status();

            if cp_result.is_err() || !cp_result.unwrap().success() {
                warn!(
                    "heartbeat: failed to copy clone '{}' for merge",
                    clone_entry.name
                );
                continue;
            }

            // Attempt merge in the copy
            let merge_result = Command::new("git")
                .args([
                    "-C",
                    &tmp_path.to_string_lossy(),
                    "merge",
                    &upstream_ref,
                    "--no-edit",
                ])
                .output();

            match merge_result {
                Ok(output) if output.status.success() => {
                    // Merge succeeded
                    let no_merge = config.no_upstream_merge(repo_name);
                    if no_merge {
                        // Opt-out: discard copy, just clear conflict flag
                        let _ = std::fs::remove_dir_all(&tmp_path);
                        debug!(
                            "heartbeat: merge would succeed for clone '{}' (no_upstream_merge set, discarding)",
                            clone_entry.name
                        );
                    } else {
                        // Auto-merge: swap copy into place
                        let _ = std::fs::remove_dir_all(&clone_entry.path);
                        if std::fs::rename(&tmp_path, &clone_entry.path).is_ok() {
                            info!(
                                "heartbeat: merged clone '{}' successfully",
                                clone_entry.name
                            );
                        } else {
                            warn!(
                                "heartbeat: failed to swap merged copy for clone '{}'",
                                clone_entry.name
                            );
                            continue;
                        }
                    }
                    if clone_entry.upstream_conflicts {
                        clone_entry.upstream_conflicts = false;
                        changed = true;
                    }
                }
                _ => {
                    // Merge failed — abort in tmp, clean up, set flag
                    let _ = Command::new("git")
                        .args(["-C", &tmp_path.to_string_lossy(), "merge", "--abort"])
                        .status();
                    let _ = std::fs::remove_dir_all(&tmp_path);
                    warn!(
                        "heartbeat: merge failed for clone '{}', marking upstream_conflicts",
                        clone_entry.name
                    );
                    if !clone_entry.upstream_conflicts {
                        clone_entry.upstream_conflicts = true;
                        changed = true;
                    }
                }
            }
        }
    }

    if changed {
        metadata.save(repo_name, config)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::metadata::{CloneEntry, Metadata};
    use chrono::Utc;
    use std::path::{Path, PathBuf};
    use std::process::Command;
    use tempfile::TempDir;

    /// Create a Config rooted in `base`.
    fn test_config(base: &Path) -> Config {
        Config {
            vault_dir: base.join("vault"),
            pristines_dir: base.join("pristines"),
            clones_dir: base.join("clones"),
            plugins_dir: base.join("plugins"),
            logs_dir: base.join("logs"),
            agent_heartbeat_interval: None,
            json_output: None,
            max_parallel: None,
            repos: None,
        }
    }

    /// Helper: initialise a bare "pristine" repo with one commit on `main`,
    /// clone it into the clones dir, add a "pristine" remote pointing back to
    /// the bare repo, and persist a Metadata file with a matching CloneEntry.
    ///
    /// Returns (config, clone_path).
    fn setup_pristine_and_clone(tmp: &Path, repo_name: &str) -> (Config, PathBuf) {
        let config = test_config(tmp);

        // Directories
        std::fs::create_dir_all(&config.vault_dir).unwrap();
        std::fs::create_dir_all(&config.pristines_dir).unwrap();
        std::fs::create_dir_all(&config.clones_dir).unwrap();

        let pristine_path = config.pristines_dir.join(repo_name);

        // 1. Create a bare repo that acts as the pristine
        Command::new("git")
            .args(["init", "--bare", &pristine_path.to_string_lossy()])
            .output()
            .expect("git init --bare");

        // 2. Create a temporary staging repo to push an initial commit
        let staging = tmp.join("staging");
        Command::new("git")
            .args([
                "clone",
                &pristine_path.to_string_lossy(),
                &staging.to_string_lossy(),
            ])
            .output()
            .expect("git clone to staging");

        // Configure git identity in staging
        Command::new("git")
            .args([
                "-C",
                &staging.to_string_lossy(),
                "config",
                "user.email",
                "test@test.com",
            ])
            .output()
            .unwrap();
        Command::new("git")
            .args([
                "-C",
                &staging.to_string_lossy(),
                "config",
                "user.name",
                "Test",
            ])
            .output()
            .unwrap();

        // Initial commit
        std::fs::write(staging.join("file.txt"), "initial\n").unwrap();
        Command::new("git")
            .args(["-C", &staging.to_string_lossy(), "add", "."])
            .output()
            .unwrap();
        Command::new("git")
            .args(["-C", &staging.to_string_lossy(), "commit", "-m", "initial"])
            .output()
            .unwrap();

        // Determine the branch name used by git (may be "main" or "master")
        let branch_out = Command::new("git")
            .args([
                "-C",
                &staging.to_string_lossy(),
                "rev-parse",
                "--abbrev-ref",
                "HEAD",
            ])
            .output()
            .unwrap();
        let branch = String::from_utf8_lossy(&branch_out.stdout)
            .trim()
            .to_string();

        Command::new("git")
            .args(["-C", &staging.to_string_lossy(), "push", "origin", &branch])
            .output()
            .unwrap();

        // 3. Clone from pristine into the clones dir
        let clone_path = config.clones_dir.join(format!("{}-work", repo_name));
        Command::new("git")
            .args([
                "clone",
                &pristine_path.to_string_lossy(),
                &clone_path.to_string_lossy(),
            ])
            .output()
            .expect("git clone for working copy");

        // Configure git identity in clone
        Command::new("git")
            .args([
                "-C",
                &clone_path.to_string_lossy(),
                "config",
                "user.email",
                "test@test.com",
            ])
            .output()
            .unwrap();
        Command::new("git")
            .args([
                "-C",
                &clone_path.to_string_lossy(),
                "config",
                "user.name",
                "Test",
            ])
            .output()
            .unwrap();

        // 4. Add a "pristine" remote (heartbeat_update_clones prefers this name)
        Command::new("git")
            .args([
                "-C",
                &clone_path.to_string_lossy(),
                "remote",
                "add",
                "pristine",
                &pristine_path.to_string_lossy(),
            ])
            .output()
            .unwrap();

        // 5. Create metadata with a CloneEntry pointing at the clone
        let mut metadata = Metadata::new(vec!["file:///dummy".to_string()]);
        metadata.clones.push(CloneEntry {
            name: format!("{}-work", repo_name),
            path: clone_path.clone(),
            created: Utc::now(),
            upstream_conflicts: false,
        });
        metadata.save(repo_name, &config).unwrap();

        (config, clone_path)
    }

    /// Push a new commit to the pristine via the staging repo helper.
    fn push_commit_to_pristine(tmp: &Path, _pristine_path: &Path, filename: &str, content: &str) {
        let staging = tmp.join("staging");
        // Pull latest first
        Command::new("git")
            .args(["-C", &staging.to_string_lossy(), "pull"])
            .output()
            .unwrap();

        std::fs::write(staging.join(filename), content).unwrap();
        Command::new("git")
            .args(["-C", &staging.to_string_lossy(), "add", "."])
            .output()
            .unwrap();
        Command::new("git")
            .args([
                "-C",
                &staging.to_string_lossy(),
                "commit",
                "-m",
                &format!("update {}", filename),
            ])
            .output()
            .unwrap();

        let branch_out = Command::new("git")
            .args([
                "-C",
                &staging.to_string_lossy(),
                "rev-parse",
                "--abbrev-ref",
                "HEAD",
            ])
            .output()
            .unwrap();
        let branch = String::from_utf8_lossy(&branch_out.stdout)
            .trim()
            .to_string();

        Command::new("git")
            .args(["-C", &staging.to_string_lossy(), "push", "origin", &branch])
            .output()
            .unwrap();
    }

    /// Return HEAD sha for a repo path.
    fn head_sha(repo_path: &Path) -> String {
        let out = Command::new("git")
            .args(["-C", &repo_path.to_string_lossy(), "rev-parse", "HEAD"])
            .output()
            .unwrap();
        String::from_utf8_lossy(&out.stdout).trim().to_string()
    }

    // -----------------------------------------------------------------------
    // Tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_heartbeat_fast_forward() {
        let tmp = TempDir::new().unwrap();
        let base = tmp.path();
        let repo_name = "ff-repo";

        let (config, clone_path) = setup_pristine_and_clone(base, repo_name);
        let pristine_path = config.pristines_dir.join(repo_name);

        // Push a new commit to pristine (clone is now behind)
        push_commit_to_pristine(base, &pristine_path, "new.txt", "hello\n");

        let sha_before = head_sha(&clone_path);

        // Run heartbeat
        heartbeat_update_clones(repo_name, &config).unwrap();

        let sha_after = head_sha(&clone_path);
        assert_ne!(
            sha_before, sha_after,
            "clone should have been fast-forwarded"
        );

        // The clone should now contain the new file
        assert!(
            clone_path.join("new.txt").exists(),
            "new.txt should exist after fast-forward"
        );

        // upstream_conflicts should remain false
        let metadata = Metadata::load(repo_name, &config).unwrap();
        assert!(
            !metadata.clones[0].upstream_conflicts,
            "upstream_conflicts should be false after clean fast-forward"
        );
    }

    #[test]
    fn test_heartbeat_up_to_date() {
        let tmp = TempDir::new().unwrap();
        let base = tmp.path();
        let repo_name = "uptodate-repo";

        let (config, clone_path) = setup_pristine_and_clone(base, repo_name);

        let sha_before = head_sha(&clone_path);

        // Run heartbeat without any new commits
        heartbeat_update_clones(repo_name, &config).unwrap();

        let sha_after = head_sha(&clone_path);
        assert_eq!(sha_before, sha_after, "clone should not have changed");
    }

    #[test]
    fn test_heartbeat_conflict() {
        let tmp = TempDir::new().unwrap();
        let base = tmp.path();
        let repo_name = "conflict-repo";

        let (config, clone_path) = setup_pristine_and_clone(base, repo_name);
        let pristine_path = config.pristines_dir.join(repo_name);

        // Edit file.txt differently in the clone (local divergence)
        std::fs::write(clone_path.join("file.txt"), "local change\n").unwrap();
        Command::new("git")
            .args(["-C", &clone_path.to_string_lossy(), "add", "."])
            .output()
            .unwrap();
        Command::new("git")
            .args([
                "-C",
                &clone_path.to_string_lossy(),
                "commit",
                "-m",
                "local edit",
            ])
            .output()
            .unwrap();

        // Push a conflicting change to file.txt via pristine
        push_commit_to_pristine(base, &pristine_path, "file.txt", "upstream change\n");

        // Run heartbeat
        heartbeat_update_clones(repo_name, &config).unwrap();

        // upstream_conflicts should now be true
        let metadata = Metadata::load(repo_name, &config).unwrap();
        assert!(
            metadata.clones[0].upstream_conflicts,
            "upstream_conflicts should be true when merge fails"
        );
    }

    #[test]
    fn test_heartbeat_skips_no_pristine() {
        let tmp = TempDir::new().unwrap();
        let base = tmp.path();
        let repo_name = "no-pristine-repo";

        let config = test_config(base);

        // Create the vault dir but NOT the pristine dir
        std::fs::create_dir_all(&config.vault_dir).unwrap();

        // Should return Ok(()) silently (no pristine path exists)
        let result = heartbeat_update_clones(repo_name, &config);
        assert!(
            result.is_ok(),
            "should silently succeed when pristine is absent"
        );
    }
}
