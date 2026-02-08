use git2::{FetchOptions, RemoteCallbacks, build::RepoBuilder};
use log::{debug, error, info};
use std::path::PathBuf;

use crate::config::Config;
use crate::error::{RepomanError, Result, git_error_with_context};
use crate::metadata::Metadata;
use crate::vault::Vault;
use super::credentials;

/// Initialize a pristine (reference clone) for a single repository
pub fn init_pristine(repo_name: &str, config: &Config) -> Result<PathBuf> {
    info!("init_pristine: starting for '{}'", repo_name);

    // Check if repo exists in vault
    let vault = Vault::load(config)?;
    if !vault.contains(repo_name) {
        error!("init_pristine: '{}' not found in vault", repo_name);
        return Err(RepomanError::RepoNotInVault(repo_name.to_string()));
    }

    // Load metadata to get git URL
    let mut metadata = Metadata::load(repo_name, config)?;
    let git_url = metadata
        .default_url()
        .ok_or_else(|| RepomanError::InvalidRepoUrl(repo_name.to_string()))?
        .to_string();

    debug!("init_pristine: git url for '{}' is '{}'", repo_name, git_url);

    // Check if pristine already exists
    let pristine_path = config.pristines_dir.join(repo_name);
    if pristine_path.exists() {
        error!("init_pristine: pristine already exists at {}", pristine_path.display());
        return Err(RepomanError::PristineAlreadyExists(repo_name.to_string()));
    }

    // Attempt counter MUST be declared before callbacks (drop order: callbacks
    // dropped first, then counter — so the borrow in the closure stays valid).
    let cred_attempts = std::cell::Cell::new(0u32);
    let mut callbacks = RemoteCallbacks::new();

    credentials::setup_credentials(
        &mut callbacks,
        &cred_attempts,
        metadata.auth_config.as_ref(),
        "init",
    );

    // Transfer progress — log/print at ~5% intervals to avoid flooding.
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

        // During receive phase, report based on received objects.
        // During index phase (received == total), report based on indexed objects.
        let (phase, done, label) = if received < total {
            ("receiving", received, "receiving")
        } else {
            ("indexing", indexed, "indexing")
        };

        let pct = (done as f64 / total as f64 * 100.0) as u32;
        let prev = last_pct.get();

        // Log at every 5% step, and once at 100%
        if pct >= prev + 5 || (pct == 100 && prev != 100) {
            last_pct.set(pct);
            debug!("init transfer: {} {}/{} objects ({:.1} MiB)", phase, done, total, mb);
            eprint!("\r  {}: {}% ({}/{}), {:.1} MiB   ", label, pct, done, total, mb);
        }
        true
    });

    // Server-side progress messages — only log to file, don't clutter console.
    callbacks.sideband_progress(|msg| {
        if let Ok(s) = std::str::from_utf8(msg) {
            let s = s.trim();
            if !s.is_empty() {
                debug!("init remote: {}", s);
            }
        }
        true
    });

    // Create fetch options with callbacks
    let mut fetch_opts = FetchOptions::new();
    fetch_opts.remote_callbacks(callbacks);

    // Clone the repository (bare clone for pristine)
    let mut builder = RepoBuilder::new();
    builder.bare(true);
    builder.fetch_options(fetch_opts);

    println!("Cloning {} into pristine...", repo_name);
    info!("init_pristine: cloning '{}' -> {}", git_url, pristine_path.display());

    builder
        .clone(&git_url, &pristine_path)
        .map_err(|e| {
            eprintln!(); // newline after progress output
            error!("init_pristine: clone failed for '{}': {}", repo_name, e);
            git_error_with_context(e, repo_name)
        })?;

    eprintln!(); // newline after progress output

    // Update metadata
    metadata.mark_pristine_created();
    metadata.save(repo_name, config)?;

    info!("init_pristine: pristine created at {}", pristine_path.display());
    println!("Pristine created: {}", pristine_path.display());

    Ok(pristine_path)
}

/// Initialize pristines for all vaulted repositories
/// Returns a Vec of (repo_name, Result) tuples
#[allow(dead_code)]
pub fn init_all_pristines(config: &Config) -> Vec<(String, Result<PathBuf>)> {
    let vault = match Vault::load(config) {
        Ok(v) => v,
        Err(e) => return vec![("vault".to_string(), Err(e))],
    };

    vault
        .get_all_names()
        .into_iter()
        .map(|name| {
            let result = init_pristine(name, config);
            (name.to_string(), result)
        })
        .collect()
}

/// Get list of repos that need initialization (not yet pristined)
pub fn get_uninitialized_repos(config: &Config) -> Result<Vec<String>> {
    let vault = Vault::load(config)?;

    let uninitialized: Vec<String> = vault
        .get_all_names()
        .into_iter()
        .filter(|name| {
            let pristine_path = config.pristines_dir.join(name);
            !pristine_path.exists()
        })
        .map(String::from)
        .collect();

    debug!("get_uninitialized_repos: found {} repos needing init", uninitialized.len());
    Ok(uninitialized)
}
