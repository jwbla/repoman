use log::debug;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

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

impl Config {
    pub fn load() -> Self {
        let config_path = dirs::config_dir().map(|p| p.join("repoman").join("config.yaml"));

        if let Some(ref path) = config_path
            && path.exists()
            && let Ok(contents) = std::fs::read_to_string(path)
            && let Ok(config) = serde_yaml::from_str::<Config>(&contents)
        {
            debug!("config: loaded from {}", path.display());
            return config;
        }

        debug!("config: using defaults (no config file found at {:?})", config_path);
        Self::default()
    }

    pub fn default() -> Self {
        let home = dirs::home_dir().expect("Could not determine home directory");

        Self {
            vault_dir: home.join(".repoman").join("vault"),
            pristines_dir: home.join(".repoman").join("pristines"),
            clones_dir: home.join(".repoman").join("clones"),
            plugins_dir: home.join(".repoman").join("plugins"),
            logs_dir: home.join(".repoman").join("logs"),
        }
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
        assert_eq!(config.plugins_dir, home.join(".repoman").join("plugins"));
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
        let config: Config = serde_yaml::from_str(yaml).unwrap();
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
        let config: Config = serde_yaml::from_str(yaml).unwrap();

        assert_eq!(config.vault_dir, PathBuf::from("/absolute/vault"));
        assert_eq!(config.pristines_dir, PathBuf::from("/absolute/pristines"));
    }
}
