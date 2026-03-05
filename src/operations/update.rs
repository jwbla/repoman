use git2::Repository;
use log::{debug, error, info, warn};

use super::sync::sync_pristine;
use crate::config::Config;
use crate::error::{RepomanError, Result};
use crate::metadata::{CloneEntry, Metadata};
use crate::vault::Vault;

/// Update a single clone: fetch from pristine and fast-forward if possible.
/// Returns a status message to print.
fn update_single_clone(clone_entry: &CloneEntry, _resolved: &str) -> String {
    if !clone_entry.path.exists() {
        warn!(
            "update_repo: clone '{}' path missing, skipping",
            clone_entry.name
        );
        return format!("  Clone {} path missing, skipped", clone_entry.name);
    }

    debug!("update_repo: updating clone '{}'", clone_entry.name);

    let repo = match Repository::open(&clone_entry.path) {
        Ok(r) => r,
        Err(e) => {
            error!(
                "update_repo: failed to open clone '{}': {}",
                clone_entry.name, e
            );
            return format!("  Failed to open clone {}: {}", clone_entry.name, e);
        }
    };

    // Detect which remote points to the local pristine:
    // new clones have "pristine", legacy clones use "origin"
    let (remote_name, ref_prefix) = if repo.find_remote("pristine").is_ok() {
        ("pristine", "refs/remotes/pristine")
    } else {
        ("origin", "refs/remotes/origin")
    };

    // Fetch from the pristine remote (local, no auth needed)
    let mut remote = match repo.find_remote(remote_name) {
        Ok(r) => r,
        Err(e) => {
            warn!(
                "update_repo: clone '{}' has no {} remote: {}",
                clone_entry.name, remote_name, e
            );
            return format!(
                "  Clone {} has no {} remote, skipped",
                clone_entry.name, remote_name
            );
        }
    };

    let fetch_refspec = format!("refs/heads/*:{}/*", ref_prefix);
    if let Err(e) = remote.fetch(&[&fetch_refspec], None, None) {
        error!(
            "update_repo: fetch failed for clone '{}': {}",
            clone_entry.name, e
        );
        return format!("  Failed to fetch clone {}: {}", clone_entry.name, e);
    }

    // Attempt fast-forward merge on current branch
    let head = match repo.head() {
        Ok(h) => h,
        Err(_) => return String::new(),
    };

    let branch_name = match head.shorthand() {
        Some(b) => b.to_string(),
        None => return String::new(),
    };

    let remote_ref_name = format!("{}/{}", ref_prefix, branch_name);
    let remote_ref = if let Ok(r) = repo.find_reference(&remote_ref_name) {
        r
    } else {
        debug!(
            "update_repo: no remote tracking branch for '{}' in clone '{}'",
            branch_name, clone_entry.name
        );
        return String::new();
    };

    let remote_oid = match remote_ref.target() {
        Some(oid) => oid,
        None => return String::new(),
    };

    let local_oid = match head.target() {
        Some(oid) => oid,
        None => return String::new(),
    };

    if local_oid == remote_oid {
        return format!(
            "  Clone {} ({}) already up-to-date",
            clone_entry.name, branch_name
        );
    }

    // Check merge analysis
    let annotated = match repo.find_annotated_commit(remote_oid) {
        Ok(c) => c,
        Err(e) => {
            warn!(
                "update_repo: failed to find annotated commit for clone '{}': {}",
                clone_entry.name, e
            );
            return format!(
                "  Clone {} ({}) failed to analyze merge",
                clone_entry.name, branch_name
            );
        }
    };

    let analysis = match repo.merge_analysis(&[&annotated]) {
        Ok((analysis, _)) => analysis,
        Err(e) => {
            warn!(
                "update_repo: merge analysis failed for clone '{}': {}",
                clone_entry.name, e
            );
            return format!(
                "  Clone {} ({}) merge analysis failed",
                clone_entry.name, branch_name
            );
        }
    };

    if analysis.is_fast_forward() || analysis.is_unborn() {
        // Fast-forward: move the branch ref and checkout
        let refname = format!("refs/heads/{}", branch_name);
        if let Err(e) = repo
            .find_reference(&refname)
            .and_then(|mut r| r.set_target(remote_oid, "repoman update: fast-forward"))
        {
            error!(
                "update_repo: ff failed for clone '{}': {}",
                clone_entry.name, e
            );
            return format!(
                "  Clone {} ({}) fast-forward failed: {}",
                clone_entry.name, branch_name, e
            );
        }

        if let Err(e) = repo.checkout_head(Some(git2::build::CheckoutBuilder::default().force())) {
            return format!(
                "  Clone {} ({}) checkout failed after ff: {}",
                clone_entry.name, branch_name, e
            );
        }

        format!(
            "  Clone {} ({}) fast-forwarded",
            clone_entry.name, branch_name
        )
    } else if analysis.is_up_to_date() {
        format!(
            "  Clone {} ({}) already up-to-date",
            clone_entry.name, branch_name
        )
    } else {
        format!(
            "  Clone {} ({}) has diverged — manual merge required",
            clone_entry.name, branch_name
        )
    }
}

/// Update a single repo: sync pristine, then fast-forward each clone.
pub fn update_repo(repo_name: &str, config: &Config) -> Result<()> {
    info!("update_repo: starting for '{}'", repo_name);

    let vault = Vault::load(config)?;
    let resolved = vault.resolve_name(repo_name).to_string();

    if !vault.contains(&resolved) {
        return Err(RepomanError::RepoNotInVault(resolved));
    }

    // Check pristine exists
    let pristine_path = config.pristines_dir.join(&resolved);
    if !pristine_path.exists() {
        return Err(RepomanError::PristineNotFound(resolved));
    }

    // 1. Sync pristine from remote
    println!("Updating {}...", resolved);
    sync_pristine(&resolved, config)?;

    // 2. Update each clone in parallel using thread::scope
    let metadata = Metadata::load(&resolved, config)?;

    let valid_clones: Vec<&CloneEntry> =
        metadata.clones.iter().filter(|c| c.path.exists()).collect();

    let max_parallel = config.max_parallel();

    for chunk in valid_clones.chunks(max_parallel) {
        std::thread::scope(|s| {
            let handles: Vec<_> = chunk
                .iter()
                .map(|entry| s.spawn(|| update_single_clone(entry, &resolved)))
                .collect();
            for h in handles {
                if let Ok(msg) = h.join()
                    && !msg.is_empty()
                {
                    println!("{}", msg);
                }
            }
        });
    }

    info!("update_repo: done for '{}'", resolved);
    Ok(())
}

/// Get list of repos that can be updated (have pristines)
pub fn get_updatable_repos(config: &Config) -> Result<Vec<String>> {
    super::sync::get_syncable_repos(config)
}
