use log::{info, warn};
use std::fs;

use crate::config::Config;
use crate::error::{RepomanError, Result};
use crate::metadata::Metadata;
use crate::vault::Vault;

/// Fully remove a repository: destroy all clones and pristine from disk,
/// remove aliases, metadata, and vault entry.
pub fn remove_repo(name: &str, config: &Config) -> Result<()> {
    let mut vault = Vault::load(config)?;

    // Resolve alias -> canonical name
    let canonical = vault.resolve_name(name).to_string();

    if !vault.contains(&canonical) {
        return Err(RepomanError::RepoNotInVault(canonical));
    }

    info!("remove_repo: removing '{}' (resolved from '{}')", canonical, name);

    // Load metadata (best-effort) and destroy all clones from disk
    if let Ok(metadata) = Metadata::load(&canonical, config) {
        for clone in &metadata.clones {
            if clone.path.exists() {
                println!("  Removing clone: {}", clone.path.display());
                if let Err(e) = fs::remove_dir_all(&clone.path) {
                    warn!("remove_repo: failed to remove clone '{}': {}", clone.path.display(), e);
                }
            }
        }
    }

    // Remove pristine directory
    let pristine_path = config.pristines_dir.join(&canonical);
    if pristine_path.exists() {
        println!("  Removing pristine: {}", pristine_path.display());
        if let Err(e) = fs::remove_dir_all(&pristine_path) {
            warn!("remove_repo: failed to remove pristine '{}': {}", pristine_path.display(), e);
        }
    }

    // Remove metadata directory (vault/<repo>/)
    let metadata_dir = config.vault_dir.join(&canonical);
    if metadata_dir.exists() {
        println!("  Removing metadata: {}", metadata_dir.display());
        if let Err(e) = fs::remove_dir_all(&metadata_dir) {
            warn!("remove_repo: failed to remove metadata dir '{}': {}", metadata_dir.display(), e);
        }
    }

    // Remove all aliases pointing at this repo
    let removed_aliases = vault.remove_aliases_for(&canonical);
    if !removed_aliases.is_empty() {
        println!("  Removed aliases: {}", removed_aliases.join(", "));
    }

    // Remove vault entry and save
    vault.remove_entry(&canonical);
    vault.save(config)?;

    println!("Repository '{}' removed", canonical);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_config() -> (TempDir, Config) {
        let temp_dir = TempDir::new().unwrap();
        let base = temp_dir.path().to_path_buf();
        let config = Config {
            vault_dir: base.join("vault"),
            pristines_dir: base.join("pristines"),
            clones_dir: base.join("clones"),
            plugins_dir: base.join("plugins"),
            logs_dir: base.join("logs"),
        };
        fs::create_dir_all(&config.vault_dir).unwrap();
        fs::create_dir_all(&config.pristines_dir).unwrap();
        fs::create_dir_all(&config.clones_dir).unwrap();
        (temp_dir, config)
    }

    fn setup_repo(config: &Config, name: &str) {
        let mut vault = Vault::load(config).unwrap_or_default();
        vault.add_entry(name.to_string(), format!("https://example.com/{}.git", name)).unwrap();
        vault.save(config).unwrap();

        let mut metadata = Metadata::new(vec![format!("https://example.com/{}.git", name)]);
        metadata.mark_pristine_created();
        metadata.save(name, config).unwrap();

        fs::create_dir_all(config.pristines_dir.join(name)).unwrap();
    }

    #[test]
    fn test_remove_full_cleanup() {
        let (_temp, config) = create_test_config();
        setup_repo(&config, "test-repo");

        // Add a clone
        let clone_path = config.clones_dir.join("test-repo-abc123");
        fs::create_dir_all(&clone_path).unwrap();
        let mut metadata = Metadata::load("test-repo", &config).unwrap();
        metadata.add_clone("abc123".to_string(), clone_path.clone());
        metadata.save("test-repo", &config).unwrap();

        // Add an alias
        let mut vault = Vault::load(&config).unwrap();
        vault.add_alias("tr".to_string(), "test-repo".to_string()).unwrap();
        vault.save(&config).unwrap();

        // Remove
        remove_repo("test-repo", &config).unwrap();

        // Verify everything is gone
        assert!(!config.pristines_dir.join("test-repo").exists());
        assert!(!clone_path.exists());
        assert!(!config.vault_dir.join("test-repo").exists());

        let vault = Vault::load(&config).unwrap();
        assert!(!vault.contains("test-repo"));
        assert!(!vault.aliases.contains_key("tr"));
    }

    #[test]
    fn test_remove_not_in_vault() {
        let (_temp, config) = create_test_config();

        let result = remove_repo("nonexistent", &config);
        assert!(result.is_err());
        match result.unwrap_err() {
            RepomanError::RepoNotInVault(name) => assert_eq!(name, "nonexistent"),
            other => panic!("Expected RepoNotInVault, got {:?}", other),
        }
    }

    #[test]
    fn test_remove_alias_cleanup() {
        let (_temp, config) = create_test_config();
        setup_repo(&config, "my-repo");

        let mut vault = Vault::load(&config).unwrap();
        vault.add_alias("mr".to_string(), "my-repo".to_string()).unwrap();
        vault.add_alias("myrepo".to_string(), "my-repo".to_string()).unwrap();
        vault.save(&config).unwrap();

        remove_repo("my-repo", &config).unwrap();

        let vault = Vault::load(&config).unwrap();
        assert!(vault.aliases.is_empty());
    }

    #[test]
    fn test_remove_via_alias() {
        let (_temp, config) = create_test_config();
        setup_repo(&config, "my-repo");

        let mut vault = Vault::load(&config).unwrap();
        vault.add_alias("mr".to_string(), "my-repo".to_string()).unwrap();
        vault.save(&config).unwrap();

        // Remove using the alias
        remove_repo("mr", &config).unwrap();

        let vault = Vault::load(&config).unwrap();
        assert!(!vault.contains("my-repo"));
        assert!(vault.aliases.is_empty());
    }

    #[test]
    fn test_remove_no_pristine_or_clones() {
        let (_temp, config) = create_test_config();

        // Add to vault but don't create pristine or clones
        let mut vault = Vault::default();
        vault.add_entry("bare-repo".to_string(), "url".to_string()).unwrap();
        vault.save(&config).unwrap();

        // Should succeed without errors (best-effort removal)
        remove_repo("bare-repo", &config).unwrap();

        let vault = Vault::load(&config).unwrap();
        assert!(!vault.contains("bare-repo"));
    }
}
