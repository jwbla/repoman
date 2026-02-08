use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::config::Config;
use crate::error::{RepomanError, Result};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloneEntry {
    pub name: String,
    pub path: PathBuf,
    pub created: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SyncInfo {
    pub timestamp: DateTime<Utc>,
    pub sync_type: String, // "auto" or "manual"
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BuildConfig {
    pub command: Option<String>,
    pub pre_build: Option<String>,
    pub post_build: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct HookConfig {
    pub pre_clone: Option<String>,
    pub post_clone: Option<String>,
    pub pre_build: Option<String>,
    pub post_build: Option<String>,
    pub pre_destroy: Option<String>,
    pub post_destroy: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AuthConfig {
    pub ssh_key_path: Option<PathBuf>,
    pub token_env_var: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Metadata {
    /// Git URLs - element 0 is the default remote
    pub git_urls: Vec<String>,
    pub created_on: DateTime<Utc>,
    pub last_updated: DateTime<Utc>,
    pub default_branch: Option<String>,
    pub tracked_branches: Vec<String>,
    pub clones: Vec<CloneEntry>,
    pub readme: Option<String>,
    pub sync_interval: Option<u64>, // seconds between syncs
    pub last_sync: Option<SyncInfo>,
    pub build_config: Option<BuildConfig>,
    pub hook_config: Option<HookConfig>,
    pub auth_config: Option<AuthConfig>,
    pub latest_tag: Option<String>,
    pub pristine_created: Option<DateTime<Utc>>,
}

impl Default for Metadata {
    fn default() -> Self {
        let now = Utc::now();
        Self {
            git_urls: Vec::new(),
            created_on: now,
            last_updated: now,
            default_branch: None,
            tracked_branches: Vec::new(),
            clones: Vec::new(),
            readme: None,
            sync_interval: None,
            last_sync: None,
            build_config: None,
            hook_config: None,
            auth_config: None,
            latest_tag: None,
            pristine_created: None,
        }
    }
}

impl Metadata {
    /// Create new metadata with the given URLs (urls[0] is default)
    pub fn new(urls: Vec<String>) -> Self {
        let now = Utc::now();
        Self {
            git_urls: urls,
            created_on: now,
            last_updated: now,
            default_branch: None,
            tracked_branches: Vec::new(),
            clones: Vec::new(),
            readme: None,
            sync_interval: Some(3600), // default 1 hour
            last_sync: None,
            build_config: None,
            hook_config: None,
            auth_config: None,
            latest_tag: None,
            pristine_created: None,
        }
    }

    /// Load metadata from disk for a given repo
    pub fn load(repo_name: &str, config: &Config) -> Result<Self> {
        let metadata_path = config.vault_dir.join(repo_name).join("metadata.json");

        if !metadata_path.exists() {
            return Err(RepomanError::MetadataLoadError(
                repo_name.to_string(),
                "File not found".to_string(),
            ));
        }

        let contents = std::fs::read_to_string(&metadata_path)
            .map_err(|e| RepomanError::MetadataLoadError(repo_name.to_string(), e.to_string()))?;

        serde_json::from_str(&contents)
            .map_err(|e| RepomanError::MetadataLoadError(repo_name.to_string(), e.to_string()))
    }

    /// Save metadata to disk for a given repo
    pub fn save(&self, repo_name: &str, config: &Config) -> Result<()> {
        let metadata_dir = config.vault_dir.join(repo_name);
        std::fs::create_dir_all(&metadata_dir)?;

        let metadata_path = metadata_dir.join("metadata.json");
        let contents = serde_json::to_string_pretty(self)
            .map_err(|e| RepomanError::MetadataSaveError(repo_name.to_string(), e.to_string()))?;

        std::fs::write(&metadata_path, contents)
            .map_err(|e| RepomanError::MetadataSaveError(repo_name.to_string(), e.to_string()))
    }

    /// Get the default (primary) git URL
    pub fn default_url(&self) -> Option<&str> {
        self.git_urls.first().map(|s| s.as_str())
    }

    /// Update the last_updated timestamp
    pub fn touch(&mut self) {
        self.last_updated = Utc::now();
    }

    /// Add a clone entry
    pub fn add_clone(&mut self, name: String, path: PathBuf) {
        self.clones.push(CloneEntry {
            name,
            path,
            created: Utc::now(),
        });
        self.touch();
    }

    /// Remove a clone entry by name
    pub fn remove_clone(&mut self, name: &str) -> Option<CloneEntry> {
        if let Some(pos) = self.clones.iter().position(|c| c.name == name) {
            self.touch();
            Some(self.clones.remove(pos))
        } else {
            None
        }
    }

    /// Get a clone entry by name
    pub fn get_clone(&self, name: &str) -> Option<&CloneEntry> {
        self.clones.iter().find(|c| c.name == name)
    }

    /// Mark as synced
    pub fn mark_synced(&mut self, sync_type: &str) {
        self.last_sync = Some(SyncInfo {
            timestamp: Utc::now(),
            sync_type: sync_type.to_string(),
        });
        self.touch();
    }

    /// Mark pristine as created
    pub fn mark_pristine_created(&mut self) {
        self.pristine_created = Some(Utc::now());
        self.touch();
    }
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
        std::fs::create_dir_all(&config.vault_dir).unwrap();
        (temp_dir, config)
    }

    #[test]
    fn test_metadata_new() {
        let urls = vec![
            "https://github.com/user/repo.git".to_string(),
            "git@github.com:user/repo.git".to_string(),
        ];
        let metadata = Metadata::new(urls.clone());

        assert_eq!(metadata.git_urls, urls);
        assert_eq!(metadata.sync_interval, Some(3600));
        assert!(metadata.clones.is_empty());
        assert!(metadata.pristine_created.is_none());
    }

    #[test]
    fn test_metadata_default_url() {
        let metadata = Metadata::new(vec!["url1".to_string(), "url2".to_string()]);
        assert_eq!(metadata.default_url(), Some("url1"));

        let empty_metadata = Metadata::default();
        assert_eq!(empty_metadata.default_url(), None);
    }

    #[test]
    fn test_metadata_add_clone() {
        let mut metadata = Metadata::new(vec!["url".to_string()]);
        let original_updated = metadata.last_updated;

        // Small delay to ensure timestamp changes
        std::thread::sleep(std::time::Duration::from_millis(10));

        metadata.add_clone("clone1".to_string(), PathBuf::from("/path/to/clone1"));

        assert_eq!(metadata.clones.len(), 1);
        assert_eq!(metadata.clones[0].name, "clone1");
        assert_eq!(metadata.clones[0].path, PathBuf::from("/path/to/clone1"));
        assert!(metadata.last_updated > original_updated);
    }

    #[test]
    fn test_metadata_get_clone() {
        let mut metadata = Metadata::new(vec!["url".to_string()]);
        metadata.add_clone("clone1".to_string(), PathBuf::from("/path/1"));
        metadata.add_clone("clone2".to_string(), PathBuf::from("/path/2"));

        let clone = metadata.get_clone("clone1");
        assert!(clone.is_some());
        assert_eq!(clone.unwrap().path, PathBuf::from("/path/1"));

        assert!(metadata.get_clone("nonexistent").is_none());
    }

    #[test]
    fn test_metadata_remove_clone() {
        let mut metadata = Metadata::new(vec!["url".to_string()]);
        metadata.add_clone("clone1".to_string(), PathBuf::from("/path/1"));
        metadata.add_clone("clone2".to_string(), PathBuf::from("/path/2"));

        let removed = metadata.remove_clone("clone1");
        assert!(removed.is_some());
        assert_eq!(removed.unwrap().name, "clone1");
        assert_eq!(metadata.clones.len(), 1);
        assert!(metadata.get_clone("clone1").is_none());
        assert!(metadata.get_clone("clone2").is_some());
    }

    #[test]
    fn test_metadata_remove_nonexistent_clone() {
        let mut metadata = Metadata::new(vec!["url".to_string()]);
        let removed = metadata.remove_clone("nonexistent");
        assert!(removed.is_none());
    }

    #[test]
    fn test_metadata_mark_synced() {
        let mut metadata = Metadata::new(vec!["url".to_string()]);
        assert!(metadata.last_sync.is_none());

        metadata.mark_synced("manual");

        assert!(metadata.last_sync.is_some());
        assert_eq!(metadata.last_sync.as_ref().unwrap().sync_type, "manual");
    }

    #[test]
    fn test_metadata_mark_pristine_created() {
        let mut metadata = Metadata::new(vec!["url".to_string()]);
        assert!(metadata.pristine_created.is_none());

        metadata.mark_pristine_created();

        assert!(metadata.pristine_created.is_some());
    }

    #[test]
    fn test_metadata_save_and_load() {
        let (_temp, config) = create_test_config();
        let repo_name = "test-repo";

        // Create and save metadata
        let mut metadata = Metadata::new(vec![
            "https://github.com/user/repo.git".to_string(),
            "git@github.com:user/repo.git".to_string(),
        ]);
        metadata.default_branch = Some("main".to_string());
        metadata.add_clone("clone1".to_string(), PathBuf::from("/path/to/clone"));
        metadata.mark_pristine_created();
        metadata.save(repo_name, &config).unwrap();

        // Load and verify
        let loaded = Metadata::load(repo_name, &config).unwrap();
        assert_eq!(loaded.git_urls.len(), 2);
        assert_eq!(
            loaded.default_url(),
            Some("https://github.com/user/repo.git")
        );
        assert_eq!(loaded.default_branch, Some("main".to_string()));
        assert_eq!(loaded.clones.len(), 1);
        assert!(loaded.pristine_created.is_some());
    }

    #[test]
    fn test_metadata_load_nonexistent() {
        let (_temp, config) = create_test_config();
        let result = Metadata::load("nonexistent", &config);
        assert!(result.is_err());
    }

    #[test]
    fn test_metadata_touch() {
        let mut metadata = Metadata::new(vec!["url".to_string()]);
        let original = metadata.last_updated;

        std::thread::sleep(std::time::Duration::from_millis(10));
        metadata.touch();

        assert!(metadata.last_updated > original);
    }
}
