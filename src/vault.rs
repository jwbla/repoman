use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::config::Config;
use crate::error::{RepomanError, Result};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultEntry {
    pub name: String,
    pub url: String,
    pub added_date: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Vault {
    pub entries: Vec<VaultEntry>,
    #[serde(default)]
    pub aliases: HashMap<String, String>,
}

impl Vault {
    /// Load vault from disk, or return empty vault if file doesn't exist
    pub fn load(config: &Config) -> Result<Self> {
        let vault_path = config.vault_dir.join("vault.json");

        if !vault_path.exists() {
            return Ok(Self::default());
        }

        let contents = std::fs::read_to_string(&vault_path)
            .map_err(|e| RepomanError::VaultLoadError(e.to_string()))?;

        serde_json::from_str(&contents).map_err(|e| RepomanError::VaultLoadError(e.to_string()))
    }

    /// Save vault to disk
    pub fn save(&self, config: &Config) -> Result<()> {
        let vault_path = config.vault_dir.join("vault.json");

        let contents = serde_json::to_string_pretty(self)
            .map_err(|e| RepomanError::VaultSaveError(e.to_string()))?;

        std::fs::write(&vault_path, contents)
            .map_err(|e| RepomanError::VaultSaveError(e.to_string()))
    }

    /// Add a new entry to the vault
    pub fn add_entry(&mut self, name: String, url: String) -> Result<()> {
        if self.get_entry(&name).is_some() {
            return Err(RepomanError::RepoAlreadyInVault(name));
        }

        self.entries.push(VaultEntry {
            name,
            url,
            added_date: Utc::now(),
        });

        Ok(())
    }

    /// Resolve an alias to its canonical name, or return the input unchanged.
    pub fn resolve_name<'a>(&'a self, name: &'a str) -> &'a str {
        self.aliases.get(name).map(|s| s.as_str()).unwrap_or(name)
    }

    /// Get entry by name (resolves aliases transparently)
    pub fn get_entry(&self, name: &str) -> Option<&VaultEntry> {
        let resolved = self.resolve_name(name);
        self.entries.iter().find(|e| e.name == resolved)
    }

    /// Get all repo names
    pub fn get_all_names(&self) -> Vec<&str> {
        self.entries.iter().map(|e| e.name.as_str()).collect()
    }

    /// Check if a repo exists in the vault (resolves aliases transparently)
    pub fn contains(&self, name: &str) -> bool {
        let resolved = self.resolve_name(name);
        self.entries.iter().any(|e| e.name == resolved)
    }

    /// Remove an entry from the vault by name
    pub fn remove_entry(&mut self, name: &str) -> Option<VaultEntry> {
        if let Some(pos) = self.entries.iter().position(|e| e.name == name) {
            Some(self.entries.remove(pos))
        } else {
            None
        }
    }

    /// Add an alias mapping `alias` -> `repo_name`
    pub fn add_alias(&mut self, alias: String, repo_name: String) -> Result<()> {
        if !self.entries.iter().any(|e| e.name == repo_name) {
            return Err(RepomanError::RepoNotInVault(repo_name));
        }
        self.aliases.insert(alias, repo_name);
        Ok(())
    }

    /// Remove an alias
    pub fn remove_alias(&mut self, alias: &str) -> Result<()> {
        if self.aliases.remove(alias).is_none() {
            return Err(RepomanError::AliasNotFound(alias.to_string()));
        }
        Ok(())
    }

    /// List all aliases
    pub fn list_aliases(&self) -> &HashMap<String, String> {
        &self.aliases
    }

    /// Remove all aliases pointing at a canonical repo name.
    /// Returns the list of alias keys that were removed.
    pub fn remove_aliases_for(&mut self, repo_name: &str) -> Vec<String> {
        let to_remove: Vec<String> = self
            .aliases
            .iter()
            .filter(|(_, target)| target.as_str() == repo_name)
            .map(|(alias, _)| alias.clone())
            .collect();
        for alias in &to_remove {
            self.aliases.remove(alias);
        }
        to_remove
    }
}

/// Extract repository name from a git URL
pub fn extract_repo_name(url: &str) -> Result<String> {
    // Handle various URL formats:
    // https://github.com/user/repo.git
    // git@github.com:user/repo.git
    // https://github.com/user/repo
    // /path/to/local/repo

    let url = url.trim();

    // Remove trailing .git if present
    let url = url.strip_suffix(".git").unwrap_or(url);

    // Remove trailing slash if present
    let url = url.strip_suffix('/').unwrap_or(url);

    // Get the last path component
    let name = if url.contains(':') && !url.contains("://") {
        // SSH format: git@github.com:user/repo
        url.rsplit(':')
            .next()
            .and_then(|path| path.rsplit('/').next())
    } else {
        // HTTPS or local path format
        url.rsplit('/').next()
    };

    name.filter(|n| !n.is_empty())
        .map(String::from)
        .ok_or_else(|| RepomanError::InvalidRepoUrl(url.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    // Helper to create a test config
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
        std::fs::create_dir_all(&config.vault_dir).unwrap();
        (temp_dir, config)
    }

    // ============ URL Extraction Tests ============

    #[test]
    fn test_extract_repo_name_https() {
        assert_eq!(
            extract_repo_name("https://github.com/user/repo.git").unwrap(),
            "repo"
        );
        assert_eq!(
            extract_repo_name("https://github.com/user/repo").unwrap(),
            "repo"
        );
    }

    #[test]
    fn test_extract_repo_name_https_trailing_slash() {
        assert_eq!(
            extract_repo_name("https://github.com/user/repo/").unwrap(),
            "repo"
        );
    }

    #[test]
    fn test_extract_repo_name_https_deep_path() {
        assert_eq!(
            extract_repo_name("https://gitlab.com/group/subgroup/repo.git").unwrap(),
            "repo"
        );
    }

    #[test]
    fn test_extract_repo_name_ssh() {
        assert_eq!(
            extract_repo_name("git@github.com:user/repo.git").unwrap(),
            "repo"
        );
        assert_eq!(
            extract_repo_name("git@github.com:user/repo").unwrap(),
            "repo"
        );
    }

    #[test]
    fn test_extract_repo_name_ssh_deep_path() {
        assert_eq!(
            extract_repo_name("git@gitlab.com:group/subgroup/repo.git").unwrap(),
            "repo"
        );
    }

    #[test]
    fn test_extract_repo_name_local() {
        assert_eq!(extract_repo_name("/path/to/local/repo").unwrap(), "repo");
    }

    #[test]
    fn test_extract_repo_name_local_trailing_slash() {
        assert_eq!(extract_repo_name("/path/to/local/repo/").unwrap(), "repo");
    }

    #[test]
    fn test_extract_repo_name_with_whitespace() {
        assert_eq!(
            extract_repo_name("  https://github.com/user/repo.git  ").unwrap(),
            "repo"
        );
    }

    #[test]
    fn test_extract_repo_name_invalid_empty() {
        assert!(extract_repo_name("").is_err());
    }

    // ============ Vault CRUD Tests ============

    #[test]
    fn test_vault_load_empty() {
        let (_temp, config) = create_test_config();
        let vault = Vault::load(&config).unwrap();
        assert!(vault.entries.is_empty());
    }

    #[test]
    fn test_vault_add_entry() {
        let vault_entry_name = "test-repo";
        let vault_entry_url = "https://github.com/user/test-repo.git";

        let mut vault = Vault::default();
        vault
            .add_entry(vault_entry_name.to_string(), vault_entry_url.to_string())
            .unwrap();

        assert_eq!(vault.entries.len(), 1);
        assert_eq!(vault.entries[0].name, vault_entry_name);
        assert_eq!(vault.entries[0].url, vault_entry_url);
    }

    #[test]
    fn test_vault_add_duplicate_fails() {
        let mut vault = Vault::default();
        vault
            .add_entry("repo".to_string(), "url1".to_string())
            .unwrap();

        let result = vault.add_entry("repo".to_string(), "url2".to_string());
        assert!(result.is_err());
    }

    #[test]
    fn test_vault_get_entry() {
        let mut vault = Vault::default();
        vault
            .add_entry("repo1".to_string(), "url1".to_string())
            .unwrap();
        vault
            .add_entry("repo2".to_string(), "url2".to_string())
            .unwrap();

        let entry = vault.get_entry("repo1");
        assert!(entry.is_some());
        assert_eq!(entry.unwrap().url, "url1");

        assert!(vault.get_entry("nonexistent").is_none());
    }

    #[test]
    fn test_vault_contains() {
        let mut vault = Vault::default();
        vault
            .add_entry("repo".to_string(), "url".to_string())
            .unwrap();

        assert!(vault.contains("repo"));
        assert!(!vault.contains("other"));
    }

    #[test]
    fn test_vault_get_all_names() {
        let mut vault = Vault::default();
        vault
            .add_entry("repo1".to_string(), "url1".to_string())
            .unwrap();
        vault
            .add_entry("repo2".to_string(), "url2".to_string())
            .unwrap();
        vault
            .add_entry("repo3".to_string(), "url3".to_string())
            .unwrap();

        let names = vault.get_all_names();
        assert_eq!(names.len(), 3);
        assert!(names.contains(&"repo1"));
        assert!(names.contains(&"repo2"));
        assert!(names.contains(&"repo3"));
    }

    #[test]
    fn test_vault_remove_entry() {
        let mut vault = Vault::default();
        vault
            .add_entry("repo1".to_string(), "url1".to_string())
            .unwrap();
        vault
            .add_entry("repo2".to_string(), "url2".to_string())
            .unwrap();

        let removed = vault.remove_entry("repo1");
        assert!(removed.is_some());
        assert_eq!(removed.unwrap().name, "repo1");
        assert_eq!(vault.entries.len(), 1);
        assert!(!vault.contains("repo1"));
    }

    #[test]
    fn test_vault_remove_nonexistent() {
        let mut vault = Vault::default();
        let removed = vault.remove_entry("nonexistent");
        assert!(removed.is_none());
    }

    #[test]
    fn test_vault_remove_aliases_for() {
        let mut vault = Vault::default();
        vault
            .add_entry("repo1".to_string(), "url1".to_string())
            .unwrap();
        vault
            .add_entry("repo2".to_string(), "url2".to_string())
            .unwrap();
        vault.add_alias("r1".to_string(), "repo1".to_string()).unwrap();
        vault.add_alias("r1-short".to_string(), "repo1".to_string()).unwrap();
        vault.add_alias("r2".to_string(), "repo2".to_string()).unwrap();

        let removed = vault.remove_aliases_for("repo1");
        assert_eq!(removed.len(), 2);
        assert!(removed.contains(&"r1".to_string()));
        assert!(removed.contains(&"r1-short".to_string()));
        // repo2 alias should remain
        assert!(vault.aliases.contains_key("r2"));
        assert!(!vault.aliases.contains_key("r1"));
    }

    #[test]
    fn test_vault_remove_aliases_for_none() {
        let mut vault = Vault::default();
        vault
            .add_entry("repo1".to_string(), "url1".to_string())
            .unwrap();
        let removed = vault.remove_aliases_for("repo1");
        assert!(removed.is_empty());
    }

    #[test]
    fn test_vault_save_and_load() {
        let (_temp, config) = create_test_config();

        // Create and save vault
        let mut vault = Vault::default();
        vault
            .add_entry("repo1".to_string(), "url1".to_string())
            .unwrap();
        vault
            .add_entry("repo2".to_string(), "url2".to_string())
            .unwrap();
        vault.save(&config).unwrap();

        // Load and verify
        let loaded = Vault::load(&config).unwrap();
        assert_eq!(loaded.entries.len(), 2);
        assert!(loaded.contains("repo1"));
        assert!(loaded.contains("repo2"));
    }
}
