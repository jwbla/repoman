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
    if path.starts_with("~/") {
        let home = dirs::home_dir().expect("Could not determine home directory");
        home.join(&path[2..])
    } else if path == "~" {
        dirs::home_dir().expect("Could not determine home directory")
    } else {
        PathBuf::from(path)
    }
}

impl Config {
    pub fn load() -> Self {
        let config_path = dirs::config_dir()
            .map(|p| p.join("repoman").join("config.yaml"));

        if let Some(path) = config_path {
            if path.exists() {
                if let Ok(contents) = std::fs::read_to_string(&path) {
                    if let Ok(config) = serde_yaml::from_str::<Config>(&contents) {
                        return config;
                    }
                }
            }
        }

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

