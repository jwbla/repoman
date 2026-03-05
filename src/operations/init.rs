use git2::{FetchOptions, RemoteCallbacks, build::RepoBuilder};
use indicatif::{ProgressBar, ProgressStyle};
use log::{debug, error, info};
use std::cell::RefCell;
use std::path::PathBuf;

use super::credentials;
use crate::config::Config;
use crate::error::{RepomanError, Result, git_error_with_context};
use crate::hooks;
use crate::metadata::Metadata;
use crate::vault::Vault;

/// Initialize a pristine (reference clone) for a single repository
pub fn init_pristine(repo_name: &str, depth: Option<i32>, config: &Config) -> Result<PathBuf> {
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

    debug!(
        "init_pristine: git url for '{}' is '{}'",
        repo_name, git_url
    );

    // Check if pristine already exists
    let pristine_path = config.pristines_dir.join(repo_name);
    if pristine_path.exists() {
        error!(
            "init_pristine: pristine already exists at {}",
            pristine_path.display()
        );
        return Err(RepomanError::PristineAlreadyExists(repo_name.to_string()));
    }

    // Attempt counter MUST be declared before callbacks (drop order: callbacks
    // dropped first, then counter — so the borrow in the closure stays valid).
    let cred_attempts = std::cell::Cell::new(0u32);
    let mut callbacks = RemoteCallbacks::new();

    // Auth: config overrides metadata
    let effective_auth = config.effective_auth(repo_name, &metadata);
    credentials::setup_credentials(
        &mut callbacks,
        &cred_attempts,
        effective_auth.as_ref(),
        "init",
    );

    // Transfer progress with indicatif progress bar
    let pb: RefCell<Option<ProgressBar>> = RefCell::new(None);
    callbacks.transfer_progress(move |stats| {
        let received = stats.received_objects();
        let indexed = stats.indexed_objects();
        let total = stats.total_objects();
        let bytes = stats.received_bytes();
        let mb = bytes as f64 / 1_048_576.0;

        if total == 0 {
            return true;
        }

        let mut pb_ref = pb.borrow_mut();
        let bar = pb_ref.get_or_insert_with(|| {
            let bar = ProgressBar::new(total as u64);
            bar.set_style(
                ProgressStyle::default_bar()
                    .template("  {bar:40.cyan/blue} {pos}/{len} ({msg})")
                    .unwrap_or_else(|_| ProgressStyle::default_bar()),
            );
            bar
        });

        let (phase, done) = if received < total {
            ("receiving", received)
        } else {
            ("indexing", indexed)
        };

        bar.set_position(done as u64);
        bar.set_message(format!("{} {:.1} MiB", phase, mb));

        if indexed == total && total > 0 {
            bar.finish_and_clear();
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

    // Apply shallow clone depth if requested
    let effective_depth = depth.or_else(|| {
        config
            .repo_config(repo_name)
            .and_then(|r| r.clone_defaults.as_ref())
            .and_then(|cd| cd.shallow)
            .and_then(|shallow| if shallow { Some(1) } else { None })
    });
    if let Some(d) = effective_depth {
        fetch_opts.depth(d);
        debug!("init_pristine: using shallow depth={}", d);
    }

    // Clone the repository (bare clone for pristine)
    let mut builder = RepoBuilder::new();
    builder.bare(true);
    builder.fetch_options(fetch_opts);

    println!("Cloning {} into pristine...", repo_name);
    info!(
        "init_pristine: cloning '{}' -> {}",
        git_url,
        pristine_path.display()
    );

    builder.clone(&git_url, &pristine_path).map_err(|e| {
        error!("init_pristine: clone failed for '{}': {}", repo_name, e);
        git_error_with_context(e, repo_name)
    })?;

    // Update metadata
    metadata.mark_pristine_created();
    metadata.save(repo_name, config)?;

    hooks::run_post_init_pristine(config, repo_name, &pristine_path)?;

    info!(
        "init_pristine: pristine created at {}",
        pristine_path.display()
    );
    println!("Pristine created: {}", pristine_path.display());

    Ok(pristine_path)
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

    debug!(
        "get_uninitialized_repos: found {} repos needing init",
        uninitialized.len()
    );
    Ok(uninitialized)
}
