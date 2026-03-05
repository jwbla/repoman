use log::debug;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Per-repo hook commands (lifecycle events). Keys match config.yaml under repos.<name>.hooks.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct HookConfig {
    pub post_init_pristine: Option<String>,
    pub pre_clone: Option<String>,
    pub post_clone: Option<String>,
    pub post_sync: Option<String>,
    #[serde(rename = "post_sync_on_new_tag")]
    pub post_sync_on_new_tag: Option<String>,
    pub pre_destroy: Option<String>,
    pub post_destroy: Option<String>,
}

/// Build commands to run after clone/sync.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct BuildConfig {
    pub command: Option<String>,
    pub pre_build: Option<String>,
    pub post_build: Option<String>,
    pub working_dir: Option<String>,
}

/// Per-repo auth overrides.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct AuthOverride {
    pub ssh_key_path: Option<String>,
    pub token_env_var: Option<String>,
}

/// Defaults for new clones.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct CloneDefaults {
    pub branch: Option<String>,
    pub shallow: Option<bool>,
}

/// Per-repo config (hooks, build, auth, etc.). Keyed by repo name in config.yaml under repos.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct RepoConfig {
    pub hooks: Option<HookConfig>,
    pub build: Option<BuildConfig>,
    pub sync_interval: Option<u64>,
    pub auth: Option<AuthOverride>,
    pub default_branch: Option<String>,
    pub auto_init: Option<bool>,
    pub clone_defaults: Option<CloneDefaults>,
    #[serde(default)]
    pub tags: Option<Vec<String>>,
    #[serde(default)]
    pub no_upstream_merge: Option<bool>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    #[serde(deserialize_with = "deserialize_path")]
    pub vault_dir: PathBuf,
    #[serde(deserialize_with = "deserialize_path")]
    pub pristines_dir: PathBuf,
    #[serde(deserialize_with = "deserialize_path")]
    pub clones_dir: PathBuf,
    #[serde(deserialize_with = "deserialize_path")]
    pub plugins_dir: PathBuf,
    #[serde(deserialize_with = "deserialize_path")]
    pub logs_dir: PathBuf,
    #[serde(default)]
    pub agent_heartbeat_interval: Option<u64>,
    #[serde(default)]
    pub json_output: Option<bool>,
    /// Maximum number of parallel operations for bulk commands (default 8).
    #[serde(default)]
    pub max_parallel: Option<u16>,
    /// Per-repo overrides (hooks, etc.). Key = repo name as in vault/list.
    #[serde(default)]
    pub repos: Option<HashMap<String, RepoConfig>>,
}

fn deserialize_path<'de, D>(deserializer: D) -> Result<PathBuf, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    Ok(expand_tilde(&s))
}

fn expand_tilde(path: &str) -> PathBuf {
    if let Some(stripped) = path.strip_prefix("~/") {
        let home = dirs::home_dir().expect("Could not determine home directory");
        home.join(stripped)
    } else if path == "~" {
        dirs::home_dir().expect("Could not determine home directory")
    } else {
        PathBuf::from(path)
    }
}

impl Default for Config {
    fn default() -> Self {
        let home = dirs::home_dir().expect("Could not determine home directory");

        Self {
            vault_dir: home.join(".repoman").join("vault"),
            pristines_dir: home.join(".repoman").join("pristines"),
            clones_dir: home.join(".repoman").join("clones"),
            plugins_dir: dirs::config_dir().map_or_else(
                || home.join(".config").join("repoman").join("plugins"),
                |c| c.join("repoman").join("plugins"),
            ),
            logs_dir: home.join(".repoman").join("logs"),
            agent_heartbeat_interval: None,
            json_output: None,
            max_parallel: None,
            repos: None,
        }
    }
}

impl Config {
    pub fn load() -> Self {
        let config_path = dirs::config_dir().map(|p| p.join("repoman").join("config.yaml"));

        if let Some(ref path) = config_path
            && path.exists()
            && let Ok(contents) = std::fs::read_to_string(path)
            && let Ok(config) = serde_yml::from_str::<Config>(&contents)
        {
            debug!("config: loaded from {}", path.display());
            return config;
        }

        debug!(
            "config: using defaults (no config file found at {:?})",
            config_path
        );
        Self::default()
    }

    /// Maximum concurrency for bulk operations (default 8).
    pub fn max_parallel(&self) -> usize {
        self.max_parallel.map_or(8, |v| v as usize)
    }

    /// Return hooks for a repo if configured.
    pub fn hooks_for_repo(&self, repo_name: &str) -> Option<&HookConfig> {
        self.repos
            .as_ref()?
            .get(repo_name)
            .and_then(|r| r.hooks.as_ref())
    }

    /// Return per-repo config if configured.
    pub fn repo_config(&self, repo_name: &str) -> Option<&RepoConfig> {
        self.repos.as_ref()?.get(repo_name)
    }

    /// Effective sync interval: config override > metadata > default 3600s.
    pub fn effective_sync_interval(
        &self,
        repo_name: &str,
        metadata: &crate::metadata::Metadata,
    ) -> u64 {
        self.repo_config(repo_name)
            .and_then(|r| r.sync_interval)
            .or(metadata.sync_interval)
            .unwrap_or(3600)
    }

    /// Effective default branch: config override > metadata > None.
    pub fn effective_default_branch(
        &self,
        repo_name: &str,
        metadata: &crate::metadata::Metadata,
    ) -> Option<String> {
        self.repo_config(repo_name)
            .and_then(|r| r.default_branch.clone())
            .or(metadata.default_branch.clone())
    }

    /// Effective auth config: merge config auth into metadata auth.
    pub fn effective_auth(
        &self,
        repo_name: &str,
        metadata: &crate::metadata::Metadata,
    ) -> Option<crate::metadata::AuthConfig> {
        let config_auth = self.repo_config(repo_name).and_then(|r| r.auth.as_ref());
        let meta_auth = metadata.auth_config.as_ref();

        match (config_auth, meta_auth) {
            (Some(ca), _) => Some(crate::metadata::AuthConfig {
                ssh_key_path: ca.ssh_key_path.as_ref().map(std::path::PathBuf::from),
                token_env_var: ca.token_env_var.clone(),
            }),
            (None, Some(ma)) => Some(ma.clone()),
            _ => None,
        }
    }

    /// Whether upstream merge is disabled for a repo (detect conflicts but don't auto-merge).
    pub fn no_upstream_merge(&self, repo_name: &str) -> bool {
        self.repo_config(repo_name)
            .and_then(|r| r.no_upstream_merge)
            .unwrap_or(false)
    }

    /// Whether JSON output is enabled (CLI flag or config default).
    pub fn json_enabled(&self, cli_json: bool) -> bool {
        cli_json || self.json_output.unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_expand_tilde_home() {
        let home = dirs::home_dir().unwrap();
        let result = expand_tilde("~");
        assert_eq!(result, home);
    }

    #[test]
    fn test_expand_tilde_subpath() {
        let home = dirs::home_dir().unwrap();
        let result = expand_tilde("~/foo/bar");
        assert_eq!(result, home.join("foo").join("bar"));
    }

    #[test]
    fn test_expand_tilde_no_tilde() {
        let result = expand_tilde("/absolute/path");
        assert_eq!(result, PathBuf::from("/absolute/path"));
    }

    #[test]
    fn test_expand_tilde_relative() {
        let result = expand_tilde("relative/path");
        assert_eq!(result, PathBuf::from("relative/path"));
    }

    #[test]
    fn test_config_default_paths() {
        let config = Config::default();
        let home = dirs::home_dir().unwrap();

        assert_eq!(config.vault_dir, home.join(".repoman").join("vault"));
        assert_eq!(
            config.pristines_dir,
            home.join(".repoman").join("pristines")
        );
        assert_eq!(config.clones_dir, home.join(".repoman").join("clones"));
        assert_eq!(
            config.plugins_dir,
            home.join(".config").join("repoman").join("plugins")
        );
        assert_eq!(config.logs_dir, home.join(".repoman").join("logs"));
    }

    #[test]
    fn test_config_yaml_parsing() {
        let yaml = r#"
vault_dir: ~/custom/vault
pristines_dir: ~/custom/pristines
clones_dir: ~/custom/clones
plugins_dir: ~/custom/plugins
logs_dir: ~/custom/logs
"#;
        let config: Config = serde_yml::from_str(yaml).unwrap();
        let home = dirs::home_dir().unwrap();

        assert_eq!(config.vault_dir, home.join("custom").join("vault"));
        assert_eq!(config.pristines_dir, home.join("custom").join("pristines"));
    }

    #[test]
    fn test_config_yaml_absolute_paths() {
        let yaml = r#"
vault_dir: /absolute/vault
pristines_dir: /absolute/pristines
clones_dir: /absolute/clones
plugins_dir: /absolute/plugins
logs_dir: /absolute/logs
"#;
        let config: Config = serde_yml::from_str(yaml).unwrap();

        assert_eq!(config.vault_dir, PathBuf::from("/absolute/vault"));
        assert_eq!(config.pristines_dir, PathBuf::from("/absolute/pristines"));
    }

    #[test]
    fn test_config_yaml_without_repos() {
        let yaml = r#"
vault_dir: ~/custom/vault
pristines_dir: ~/custom/pristines
clones_dir: ~/custom/clones
plugins_dir: ~/custom/plugins
logs_dir: ~/custom/logs
"#;
        let config: Config = serde_yml::from_str(yaml).unwrap();
        assert!(config.repos.is_none());
        assert!(config.hooks_for_repo("any").is_none());
    }

    #[test]
    fn test_config_yaml_with_repos_and_hooks() {
        let yaml = r#"
vault_dir: ~/custom/vault
pristines_dir: ~/custom/pristines
clones_dir: ~/custom/clones
plugins_dir: ~/custom/plugins
logs_dir: ~/custom/logs
repos:
  my-repo:
    hooks:
      post_clone: "npm ci"
      post_sync: "./scripts/deploy.sh"
  other-repo:
    hooks:
      post_init_pristine: "echo ready"
"#;
        let config: Config = serde_yml::from_str(yaml).unwrap();
        assert!(config.repos.is_some());
        let hooks = config.hooks_for_repo("my-repo").unwrap();
        assert_eq!(hooks.post_clone.as_deref(), Some("npm ci"));
        assert_eq!(hooks.post_sync.as_deref(), Some("./scripts/deploy.sh"));
        assert!(hooks.post_init_pristine.is_none());

        let hooks2 = config.hooks_for_repo("other-repo").unwrap();
        assert_eq!(hooks2.post_init_pristine.as_deref(), Some("echo ready"));

        assert!(config.hooks_for_repo("nonexistent").is_none());
    }
}
