use git2::Repository;
use log::{debug, info};
use std::env;

use crate::config::Config;
use crate::error::{RepomanError, Result};
use crate::metadata::Metadata;
use crate::vault::{Vault, extract_repo_name};

/// Detect remote URLs from the current directory's git repository
/// Returns Vec<String> with the default remote URL as the first element
pub fn detect_current_repo_urls() -> Result<Vec<String>> {
    let current_dir = env::current_dir()?;
    debug!("detect_current_repo_urls: scanning {}", current_dir.display());

    // Try to open the current directory as a git repository
    let repo =
        Repository::discover(&current_dir).map_err(|_| RepomanError::NotAGitRepo(current_dir))?;

    let mut urls = Vec::new();
    let mut default_remote_name: Option<String> = None;

    // Determine the default remote:
    // 1. Check branch.<current-branch>.remote
    // 2. Check remote.pushDefault
    // 3. Use "origin" if it exists
    // 4. Use first remote alphabetically

    if let Ok(head) = repo.head()
        && let Some(branch_name) = head.shorthand()
    {
        let config_key = format!("branch.{}.remote", branch_name);
        if let Ok(config) = repo.config()
            && let Ok(remote) = config.get_string(&config_key)
        {
            default_remote_name = Some(remote);
        }
    }

    if default_remote_name.is_none()
        && let Ok(config) = repo.config()
        && let Ok(push_default) = config.get_string("remote.pushDefault")
    {
        default_remote_name = Some(push_default);
    }

    // Get all remotes
    let remotes = repo.remotes().map_err(|_| RepomanError::NoRemotesFound)?;

    if remotes.is_empty() {
        return Err(RepomanError::NoRemotesFound);
    }

    let mut remote_urls: Vec<(String, String)> = Vec::new();

    for remote_name in remotes.iter().flatten() {
        if let Ok(remote) = repo.find_remote(remote_name)
            && let Some(url) = remote.url()
        {
            remote_urls.push((remote_name.to_string(), url.to_string()));
        }
    }

    if remote_urls.is_empty() {
        return Err(RepomanError::NoRemotesFound);
    }

    // If no default found yet, use "origin" if it exists
    if default_remote_name.is_none() && remote_urls.iter().any(|(name, _)| name == "origin") {
        default_remote_name = Some("origin".to_string());
    }

    // If still no default, use first alphabetically
    if default_remote_name.is_none() {
        remote_urls.sort_by(|a, b| a.0.cmp(&b.0));
        default_remote_name = remote_urls.first().map(|(name, _)| name.clone());
    }

    // Build the result with default first
    let default_name = default_remote_name.unwrap_or_default();

    // Add default URL first
    if let Some((_, url)) = remote_urls.iter().find(|(name, _)| name == &default_name) {
        urls.push(url.clone());
    }

    // Add remaining URLs
    for (name, url) in &remote_urls {
        if name != &default_name {
            urls.push(url.clone());
        }
    }

    Ok(urls)
}

/// Add a repository to the vault
pub fn add_repo(url: Option<String>, config: &Config) -> Result<String> {
    let urls: Vec<String>;
    let detected_from_current: bool;

    match url {
        Some(u) => {
            urls = vec![u];
            detected_from_current = false;
        }
        None => {
            urls = detect_current_repo_urls()?;
            detected_from_current = true;
        }
    }

    debug!("add_repo: resolved {} url(s), detected_from_cwd={}", urls.len(), detected_from_current);

    // Warn about multiple remotes
    if detected_from_current && urls.len() > 1 {
        let default_url = urls.first().map(|s| s.as_str()).unwrap_or("");
        println!(
            "Multiple remotes detected. Adding all remotes with '{}' as default.",
            default_url
        );
        println!("You can change defaults later by editing metadata.");
    }

    // Extract repo name from default URL
    let default_url = urls.first().ok_or(RepomanError::NoRemotesFound)?;
    let repo_name = extract_repo_name(default_url)?;

    // Load vault and check for duplicates
    let mut vault = Vault::load(config)?;

    if vault.contains(&repo_name) {
        return Err(RepomanError::RepoAlreadyInVault(repo_name));
    }

    // Add to vault
    vault.add_entry(repo_name.clone(), default_url.clone())?;
    vault.save(config)?;

    // Create metadata directory and save metadata
    let default_url_owned = default_url.to_string();
    let metadata = Metadata::new(urls);
    metadata.save(&repo_name, config)?;

    info!("add_repo: '{}' added to vault (url={})", repo_name, default_url_owned);
    Ok(repo_name)
}
