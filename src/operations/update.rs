use git2::Repository;
use log::{debug, error, info, warn};

use crate::config::Config;
use crate::error::{RepomanError, Result};
use crate::metadata::Metadata;
use crate::vault::Vault;
use super::sync::sync_pristine;

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

    // 2. Update each clone
    let metadata = Metadata::load(&resolved, config)?;

    for clone_entry in &metadata.clones {
        if !clone_entry.path.exists() {
            warn!("update_repo: clone '{}' path missing, skipping", clone_entry.name);
            continue;
        }

        debug!("update_repo: updating clone '{}'", clone_entry.name);

        let repo = match Repository::open(&clone_entry.path) {
            Ok(r) => r,
            Err(e) => {
                error!("update_repo: failed to open clone '{}': {}", clone_entry.name, e);
                println!("  Failed to open clone {}: {}", clone_entry.name, e);
                continue;
            }
        };

        // Fetch from origin (local pristine, no auth needed)
        let mut remote = match repo.find_remote("origin") {
            Ok(r) => r,
            Err(e) => {
                warn!("update_repo: clone '{}' has no origin remote: {}", clone_entry.name, e);
                continue;
            }
        };

        if let Err(e) = remote.fetch(&["refs/heads/*:refs/remotes/origin/*"], None, None) {
            error!("update_repo: fetch failed for clone '{}': {}", clone_entry.name, e);
            println!("  Failed to fetch clone {}: {}", clone_entry.name, e);
            continue;
        }

        // Attempt fast-forward merge on current branch
        let head = match repo.head() {
            Ok(h) => h,
            Err(_) => continue,
        };

        let branch_name = match head.shorthand() {
            Some(b) => b.to_string(),
            None => continue,
        };

        let remote_ref_name = format!("refs/remotes/origin/{}", branch_name);
        let remote_ref = match repo.find_reference(&remote_ref_name) {
            Ok(r) => r,
            Err(_) => {
                debug!("update_repo: no remote tracking branch for '{}' in clone '{}'",
                    branch_name, clone_entry.name);
                continue;
            }
        };

        let remote_oid = match remote_ref.target() {
            Some(oid) => oid,
            None => continue,
        };

        let local_oid = match head.target() {
            Some(oid) => oid,
            None => continue,
        };

        if local_oid == remote_oid {
            println!("  Clone {} ({}) already up-to-date", clone_entry.name, branch_name);
            continue;
        }

        // Check merge analysis
        let analysis = match repo.merge_analysis(&[&repo.find_annotated_commit(remote_oid)?]) {
            Ok((analysis, _)) => analysis,
            Err(e) => {
                warn!("update_repo: merge analysis failed for clone '{}': {}", clone_entry.name, e);
                continue;
            }
        };

        if analysis.is_fast_forward() || analysis.is_unborn() {
            // Fast-forward: move the branch ref and checkout
            let refname = format!("refs/heads/{}", branch_name);
            repo.find_reference(&refname)
                .and_then(|mut r| r.set_target(remote_oid, "repoman update: fast-forward"))
                .map_err(|e| {
                    error!("update_repo: ff failed for clone '{}': {}", clone_entry.name, e);
                    RepomanError::FastForwardFailed(clone_entry.name.clone(), e.to_string())
                })?;

            repo.checkout_head(Some(git2::build::CheckoutBuilder::default().force()))?;
            println!("  Clone {} ({}) fast-forwarded", clone_entry.name, branch_name);
        } else if analysis.is_up_to_date() {
            println!("  Clone {} ({}) already up-to-date", clone_entry.name, branch_name);
        } else {
            println!("  Clone {} ({}) has diverged — manual merge required", clone_entry.name, branch_name);
        }
    }

    info!("update_repo: done for '{}'", resolved);
    Ok(())
}

/// Get list of repos that can be updated (have pristines)
pub fn get_updatable_repos(config: &Config) -> Result<Vec<String>> {
    super::sync::get_syncable_repos(config)
}
