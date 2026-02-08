use git2::{FetchOptions, RemoteCallbacks, Repository};
use log::{debug, error, info};

use crate::config::Config;
use crate::error::{RepomanError, Result, git_error_with_context};
use crate::metadata::Metadata;
use crate::vault::Vault;
use super::credentials;

/// Sync (fetch/update) a single pristine from its origin
pub fn sync_pristine(pristine_name: &str, config: &Config) -> Result<()> {
    info!("sync_pristine: starting for '{}'", pristine_name);

    // Check if repo exists in vault
    let vault = Vault::load(config)?;
    if !vault.contains(pristine_name) {
        error!("sync_pristine: '{}' not found in vault", pristine_name);
        return Err(RepomanError::RepoNotInVault(pristine_name.to_string()));
    }

    // Check if pristine exists
    let pristine_path = config.pristines_dir.join(pristine_name);
    if !pristine_path.exists() {
        error!("sync_pristine: pristine not found at {}", pristine_path.display());
        return Err(RepomanError::PristineNotFound(pristine_name.to_string()));
    }

    // Load metadata
    let mut metadata = Metadata::load(pristine_name, config)?;

    // Open the pristine repository
    let repo = Repository::open_bare(&pristine_path)?;

    // Get the origin URL from metadata
    let origin_url = metadata
        .default_url()
        .ok_or_else(|| RepomanError::InvalidRepoUrl(pristine_name.to_string()))?;

    debug!("sync_pristine: fetching '{}' from '{}'", pristine_name, origin_url);
    println!("Syncing {} from {}...", pristine_name, origin_url);

    // Attempt counter declared before callbacks for correct drop order.
    let cred_attempts = std::cell::Cell::new(0u32);
    let mut callbacks = RemoteCallbacks::new();

    credentials::setup_credentials(
        &mut callbacks,
        &cred_attempts,
        metadata.auth_config.as_ref(),
        "sync",
    );

    // Transfer progress — log at ~5% intervals.
    let last_pct = std::cell::Cell::new(0u32);
    callbacks.transfer_progress(|stats| {
        let received = stats.received_objects();
        let indexed = stats.indexed_objects();
        let total = stats.total_objects();
        let bytes = stats.received_bytes();
        let mb = bytes as f64 / 1_048_576.0;

        if total == 0 {
            return true;
        }

        let (phase, done) = if received < total {
            ("receiving", received)
        } else {
            ("indexing", indexed)
        };

        let pct = (done as f64 / total as f64 * 100.0) as u32;
        let prev = last_pct.get();

        if pct >= prev + 5 || (pct == 100 && prev != 100) {
            last_pct.set(pct);
            debug!("sync transfer: {} {}/{} objects ({:.1} MiB)", phase, done, total, mb);
        }
        true
    });

    callbacks.sideband_progress(|msg| {
        if let Ok(s) = std::str::from_utf8(msg) {
            let s = s.trim();
            if !s.is_empty() {
                debug!("sync remote: {}", s);
            }
        }
        true
    });

    // Set up fetch options
    let mut fetch_opts = FetchOptions::new();
    fetch_opts.remote_callbacks(callbacks);

    // Find or create origin remote
    let mut remote = match repo.find_remote("origin") {
        Ok(r) => r,
        Err(_) => {
            debug!("sync_pristine: origin remote missing, creating it");
            repo.remote("origin", origin_url)?;
            repo.find_remote("origin")?
        }
    };

    // Fetch all branches and tags
    remote
        .fetch(
            &["refs/heads/*:refs/heads/*", "refs/tags/*:refs/tags/*"],
            Some(&mut fetch_opts),
            None,
        )
        .map_err(|e| {
            error!("sync_pristine: fetch failed for '{}': {}", pristine_name, e);
            git_error_with_context(e, pristine_name)
        })?;

    // Update metadata
    metadata.mark_synced("manual");
    metadata.save(pristine_name, config)?;

    info!("sync_pristine: sync complete for '{}'", pristine_name);
    println!("Sync complete for {}", pristine_name);

    Ok(())
}

/// Sync all pristines
/// Returns a Vec of (repo_name, Result) tuples
#[allow(dead_code)]
pub fn sync_all_pristines(config: &Config) -> Vec<(String, Result<()>)> {
    let vault = match Vault::load(config) {
        Ok(v) => v,
        Err(e) => return vec![("vault".to_string(), Err(e))],
    };

    vault
        .get_all_names()
        .into_iter()
        .filter(|name| {
            let pristine_path = config.pristines_dir.join(name);
            pristine_path.exists()
        })
        .map(|name| {
            let result = sync_pristine(name, config);
            (name.to_string(), result)
        })
        .collect()
}

/// Get list of repos that can be synced (have pristines)
pub fn get_syncable_repos(config: &Config) -> Result<Vec<String>> {
    let vault = Vault::load(config)?;

    let syncable: Vec<String> = vault
        .get_all_names()
        .into_iter()
        .filter(|name| {
            let pristine_path = config.pristines_dir.join(name);
            pristine_path.exists()
        })
        .map(String::from)
        .collect();

    debug!("get_syncable_repos: found {} repos with pristines", syncable.len());
    Ok(syncable)
}

/// Check for new tags on a remote
pub fn check_for_new_tag(pristine_name: &str, config: &Config) -> Result<Option<String>> {
    debug!("check_for_new_tag: checking '{}'", pristine_name);

    // Load metadata
    let metadata = Metadata::load(pristine_name, config)?;
    let current_tag = metadata.latest_tag.clone();

    // Get the origin URL
    let origin_url = metadata
        .default_url()
        .ok_or_else(|| RepomanError::InvalidRepoUrl(pristine_name.to_string()))?;

    // Create a temporary remote to list tags
    let temp_dir = std::env::temp_dir().join(format!("repoman-check-{}", pristine_name));
    let _ = std::fs::remove_dir_all(&temp_dir);

    // Use git2 to list remote refs
    let attempts = std::cell::Cell::new(0u32);
    let mut callbacks = RemoteCallbacks::new();

    credentials::setup_credentials(
        &mut callbacks,
        &attempts,
        metadata.auth_config.as_ref(),
        "tag-check",
    );

    // Connect to remote and list refs
    let mut remote = git2::Remote::create_detached(origin_url)
        .map_err(|e| git_error_with_context(e, pristine_name))?;
    remote
        .connect_auth(git2::Direction::Fetch, Some(callbacks), None)
        .map_err(|e| {
            error!("check_for_new_tag: failed to connect to remote for '{}': {}", pristine_name, e);
            git_error_with_context(e, pristine_name)
        })?;

    let refs = remote.list()?;

    // Collect tag names
    let tags: Vec<&str> = refs
        .iter()
        .filter_map(|r| {
            let name = r.name();
            if name.starts_with("refs/tags/") {
                Some(name.strip_prefix("refs/tags/").unwrap_or(name))
            } else {
                None
            }
        })
        // Filter out ^{} dereferenced tags
        .filter(|t| !t.ends_with("^{}"))
        .collect();

    // Semver-aware sort: parse tags, sort semver properly, non-semver alphabetically first
    fn strip_v(t: &str) -> &str {
        t.strip_prefix('v').or_else(|| t.strip_prefix('V')).unwrap_or(t)
    }

    let mut semver_tags: Vec<(&str, semver::Version)> = Vec::new();
    let mut other_tags: Vec<&str> = Vec::new();

    for tag in &tags {
        if let Ok(ver) = semver::Version::parse(strip_v(tag)) {
            semver_tags.push((tag, ver));
        } else {
            other_tags.push(tag);
        }
    }

    other_tags.sort();
    semver_tags.sort_by(|a, b| a.1.cmp(&b.1));

    // Latest = last semver tag if any, else last alphabetical
    let latest_tag = semver_tags
        .last()
        .map(|(name, _)| name.to_string())
        .or_else(|| other_tags.last().map(|s| s.to_string()));

    debug!(
        "check_for_new_tag: '{}' current={:?} latest={:?}",
        pristine_name, current_tag, latest_tag
    );

    // Compare with current
    if latest_tag != current_tag {
        if let Some(ref tag) = latest_tag {
            info!("check_for_new_tag: new tag found for '{}': {}", pristine_name, tag);
        }
        Ok(latest_tag)
    } else {
        Ok(None)
    }
}

/// Update the latest tag in metadata
pub fn update_latest_tag(pristine_name: &str, tag: &str, config: &Config) -> Result<()> {
    debug!("update_latest_tag: '{}' -> '{}'", pristine_name, tag);
    let mut metadata = Metadata::load(pristine_name, config)?;
    metadata.latest_tag = Some(tag.to_string());
    metadata.save(pristine_name, config)?;
    Ok(())
}
