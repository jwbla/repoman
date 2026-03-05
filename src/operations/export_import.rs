use log::{debug, info};
use serde::{Deserialize, Serialize};

use crate::config::Config;
use crate::error::Result;
use crate::metadata::Metadata;
use crate::vault::Vault;

#[derive(Debug, Serialize, Deserialize)]
struct ExportEntry {
    name: String,
    url: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    aliases: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ExportData {
    repositories: Vec<ExportEntry>,
}

/// Export vault contents to YAML string.
pub fn export_vault(config: &Config) -> Result<String> {
    let vault = Vault::load(config)?;

    let mut entries = Vec::new();
    for entry in &vault.entries {
        let aliases: Vec<String> = vault
            .aliases
            .iter()
            .filter(|(_, target)| target.as_str() == entry.name)
            .map(|(alias, _)| alias.clone())
            .collect();

        entries.push(ExportEntry {
            name: entry.name.clone(),
            url: entry.url.clone(),
            aliases,
        });
    }

    let data = ExportData {
        repositories: entries,
    };

    let yaml = serde_yml::to_string(&data)
        .map_err(|e| crate::error::RepomanError::ConfigError(e.to_string()))?;

    info!(
        "export_vault: exported {} repositories",
        data.repositories.len()
    );
    Ok(yaml)
}

/// Import repositories from a YAML string. Skips duplicates. Returns count of newly added repos.
pub fn import_vault_from_string(yaml: &str, config: &Config) -> Result<usize> {
    let data: ExportData = serde_yml::from_str(yaml)
        .map_err(|e| crate::error::RepomanError::ConfigError(e.to_string()))?;

    let mut vault = Vault::load(config)?;
    let mut count = 0;

    for entry in &data.repositories {
        if vault.contains(&entry.name) {
            debug!("import_vault: skipping '{}' (already in vault)", entry.name);
            println!("  Skipping {} (already in vault)", entry.name);
            continue;
        }

        vault.add_entry(entry.name.clone(), entry.url.clone())?;

        // Create metadata
        let metadata = Metadata::new(vec![entry.url.clone()]);
        metadata.save(&entry.name, config)?;

        // Add aliases
        for alias in &entry.aliases {
            let _ = vault.add_alias(alias.clone(), entry.name.clone());
        }

        println!("  Added {}", entry.name);
        count += 1;
    }

    vault.save(config)?;
    info!("import_vault: imported {} new repositories", count);
    Ok(count)
}

/// Import repositories from a YAML file. Skips duplicates. Returns count of newly added repos.
pub fn import_vault(path: &str, config: &Config) -> Result<usize> {
    let contents = std::fs::read_to_string(path)?;
    import_vault_from_string(&contents, config)
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
            agent_heartbeat_interval: None,
            json_output: None,
            max_parallel: None,
            repos: None,
        };
        std::fs::create_dir_all(&config.vault_dir).unwrap();
        (temp_dir, config)
    }

    #[test]
    fn test_export_empty_vault() {
        let (_temp, config) = create_test_config();
        let yaml = export_vault(&config).unwrap();
        assert!(yaml.contains("repositories"));
    }

    #[test]
    fn test_export_import_roundtrip() {
        let (temp, config) = create_test_config();

        // Set up vault with repos and aliases
        let mut vault = Vault::default();
        vault
            .add_entry(
                "repo1".to_string(),
                "https://github.com/user/repo1.git".to_string(),
            )
            .unwrap();
        vault
            .add_entry(
                "repo2".to_string(),
                "git@github.com:user/repo2.git".to_string(),
            )
            .unwrap();
        vault
            .add_alias("r1".to_string(), "repo1".to_string())
            .unwrap();
        vault.save(&config).unwrap();

        // Create metadata
        Metadata::new(vec!["https://github.com/user/repo1.git".to_string()])
            .save("repo1", &config)
            .unwrap();
        Metadata::new(vec!["git@github.com:user/repo2.git".to_string()])
            .save("repo2", &config)
            .unwrap();

        // Export
        let yaml = export_vault(&config).unwrap();
        assert!(yaml.contains("repo1"));
        assert!(yaml.contains("repo2"));

        // Write to file
        let export_path = temp.path().join("export.yaml");
        std::fs::write(&export_path, &yaml).unwrap();

        // Import into fresh vault
        let (_, config2) = create_test_config();
        let count = import_vault(export_path.to_str().unwrap(), &config2).unwrap();
        assert_eq!(count, 2);

        // Verify
        let vault2 = Vault::load(&config2).unwrap();
        assert!(vault2.contains("repo1"));
        assert!(vault2.contains("repo2"));
        assert_eq!(vault2.resolve_name("r1"), "repo1");
    }

    #[test]
    fn test_import_skips_duplicates() {
        let (temp, config) = create_test_config();

        // Add repo1
        let mut vault = Vault::default();
        vault
            .add_entry("repo1".to_string(), "url1".to_string())
            .unwrap();
        vault.save(&config).unwrap();

        // Write import file with repo1 and repo2
        let yaml = r#"
repositories:
  - name: repo1
    url: url1
  - name: repo2
    url: url2
"#;
        let import_path = temp.path().join("import.yaml");
        std::fs::write(&import_path, yaml).unwrap();

        let count = import_vault(import_path.to_str().unwrap(), &config).unwrap();
        assert_eq!(count, 1); // only repo2 added

        let vault = Vault::load(&config).unwrap();
        assert!(vault.contains("repo1"));
        assert!(vault.contains("repo2"));
    }
}
