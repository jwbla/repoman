use log::{debug, info, warn};
use std::fs;
use std::path::PathBuf;

use crate::config::Config;
use crate::error::{RepomanError, Result};
use crate::metadata::Metadata;
use crate::vault::Vault;
use super::gc;

/// Destroy a clone
/// Removes the clone from disk and updates metadata
pub fn destroy_clone(clone_name: &str, config: &Config) -> Result<PathBuf> {
    info!("destroy_clone: destroying '{}'", clone_name);
    let vault = Vault::load(config)?;

    // Find which repo this clone belongs to
    // Clone names are in format: <pristine-name>-<suffix>
    for repo_name in vault.get_all_names() {
        if let Ok(mut metadata) = Metadata::load(repo_name, config) {
            // Check if this clone exists in this repo's metadata
            if metadata.get_clone(clone_name).is_some() {
                // Found the clone - get the path before removing
                let clone_entry = metadata.get_clone(clone_name).unwrap();
                let clone_path = clone_entry.path.clone();

                // Remove from filesystem
                if clone_path.exists() {
                    println!("Removing clone directory: {}", clone_path.display());
                    fs::remove_dir_all(&clone_path)?;
                }

                // Update metadata
                metadata.remove_clone(clone_name);
                metadata.save(repo_name, config)?;

                println!("Clone '{}' destroyed", clone_name);
                return Ok(clone_path);
            }
        }
    }

    // Also check if it's a clone directory name (pristine-suffix format)
    let clone_path = config.clones_dir.join(clone_name);
    if clone_path.exists() {
        // Try to determine the pristine name from the directory name
        if let Some(pristine_name) = clone_name
            .rsplit('-')
            .skip(1)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect::<Vec<_>>()
            .join("-")
            .into()
        {
            let pristine_name: String = pristine_name;
            if !pristine_name.is_empty()
                && let Ok(mut metadata) = Metadata::load(&pristine_name, config)
            {
                // Try to find by path
                let clone_suffix = clone_name.strip_prefix(&format!("{}-", pristine_name));
                if let Some(suffix) = clone_suffix
                    && metadata.get_clone(suffix).is_some()
                {
                    metadata.remove_clone(suffix);
                    metadata.save(&pristine_name, config)?;
                }
            }
        }

        // Remove the directory even if we couldn't update metadata
        println!("Removing clone directory: {}", clone_path.display());
        fs::remove_dir_all(&clone_path)?;
        println!("Clone '{}' destroyed", clone_name);
        return Ok(clone_path);
    }

    Err(RepomanError::CloneNotFound(clone_name.to_string()))
}

/// Destroy a pristine
/// Removes the pristine from disk but keeps the vault entry
pub fn destroy_pristine(pristine_name: &str, config: &Config) -> Result<PathBuf> {
    info!("destroy_pristine: destroying '{}'", pristine_name);
    // Check if repo exists in vault
    let vault = Vault::load(config)?;
    if !vault.contains(pristine_name) {
        return Err(RepomanError::RepoNotInVault(pristine_name.to_string()));
    }

    // Check if pristine exists
    let pristine_path = config.pristines_dir.join(pristine_name);
    if !pristine_path.exists() {
        return Err(RepomanError::PristineNotFound(pristine_name.to_string()));
    }

    // Remove from filesystem
    println!("Removing pristine directory: {}", pristine_path.display());
    fs::remove_dir_all(&pristine_path)?;

    // Update metadata to clear pristine_created
    if let Ok(mut metadata) = Metadata::load(pristine_name, config) {
        metadata.pristine_created = None;
        metadata.save(pristine_name, config)?;
    }

    println!("Pristine '{}' destroyed (vault entry kept)", pristine_name);
    Ok(pristine_path)
}

/// Determine if a target is a clone or pristine and destroy accordingly
pub fn destroy_target(target: &str, config: &Config) -> Result<PathBuf> {
    debug!("destroy_target: resolving target '{}'", target);
    let vault = Vault::load(config)?;

    // Check if it's a pristine (repo name in vault with existing pristine)
    if vault.contains(target) {
        let pristine_path = config.pristines_dir.join(target);
        if pristine_path.exists() {
            return destroy_pristine(target, config);
        }
    }

    // Check if it's a clone directory
    let clone_path = config.clones_dir.join(target);
    if clone_path.exists() {
        return destroy_clone(target, config);
    }

    // Check if it's a clone suffix in any repo's metadata
    for repo_name in vault.get_all_names() {
        if let Ok(metadata) = Metadata::load(repo_name, config)
            && metadata.get_clone(target).is_some()
        {
            return destroy_clone(target, config);
        }
    }

    // Not found as either
    Err(RepomanError::CloneNotFound(target.to_string()))
}

/// Destroy all clones for a given pristine
pub fn destroy_all_clones(pristine_name: &str, config: &Config) -> Result<Vec<PathBuf>> {
    info!("destroy_all_clones: destroying all clones for '{}'", pristine_name);

    let vault = Vault::load(config)?;
    if !vault.contains(pristine_name) {
        return Err(RepomanError::RepoNotInVault(pristine_name.to_string()));
    }

    let mut metadata = Metadata::load(pristine_name, config)?;
    let mut removed = Vec::new();

    let clone_names: Vec<String> = metadata.clones.iter().map(|c| c.name.clone()).collect();
    for name in &clone_names {
        if let Some(entry) = metadata.get_clone(name) {
            let path = entry.path.clone();
            if path.exists() {
                println!("Removing clone: {}", path.display());
                if let Err(e) = fs::remove_dir_all(&path) {
                    warn!("destroy_all_clones: failed to remove '{}': {}", path.display(), e);
                    continue;
                }
            }
            removed.push(path);
        }
        metadata.remove_clone(name);
    }

    metadata.save(pristine_name, config)?;

    println!("Destroyed {} clones for '{}'", removed.len(), pristine_name);
    Ok(removed)
}

/// Destroy clones whose HEAD commit is older than `days` days
pub fn destroy_stale_clones(days: u64, config: &Config) -> Result<Vec<PathBuf>> {
    info!("destroy_stale_clones: destroying clones older than {} days", days);

    let stale = gc::find_stale_clones(days, config)?;
    let mut removed = Vec::new();

    for sc in &stale {
        if sc.path.exists() {
            println!("Removing stale clone '{}' ({} days old): {}", sc.clone_name, sc.days_old, sc.path.display());
            if let Err(e) = fs::remove_dir_all(&sc.path) {
                warn!("destroy_stale_clones: failed to remove '{}': {}", sc.path.display(), e);
                continue;
            }
        }

        // Update metadata
        if let Ok(mut metadata) = Metadata::load(&sc.repo_name, config) {
            metadata.remove_clone(&sc.clone_name);
            let _ = metadata.save(&sc.repo_name, config);
        }

        removed.push(sc.path.clone());
    }

    println!("Destroyed {} stale clones", removed.len());
    Ok(removed)
}

/// Destroy all pristines across all vault entries.
/// Keeps vault entries intact. Best-effort: warns on failures and continues.
pub fn destroy_all_pristines(config: &Config) -> Result<Vec<PathBuf>> {
    info!("destroy_all_pristines: destroying all pristines");

    let vault = Vault::load(config)?;
    let mut removed = Vec::new();

    for name in vault.get_all_names() {
        let pristine_path = config.pristines_dir.join(name);
        if pristine_path.exists() {
            println!("Removing pristine: {}", pristine_path.display());
            if let Err(e) = fs::remove_dir_all(&pristine_path) {
                warn!("destroy_all_pristines: failed to remove '{}': {}", pristine_path.display(), e);
                continue;
            }
            removed.push(pristine_path);
        }

        // Clear pristine_created in metadata (best-effort)
        if let Ok(mut metadata) = Metadata::load(name, config) {
            metadata.pristine_created = None;
            let _ = metadata.save(name, config);
        }
    }

    println!("Destroyed {} pristines", removed.len());
    Ok(removed)
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

    #[test]
    fn test_destroy_pristine_success() {
        let (_temp, config) = create_test_config();

        // Set up vault and pristine
        let mut vault = Vault::default();
        vault
            .add_entry("test-repo".to_string(), "url".to_string())
            .unwrap();
        vault.save(&config).unwrap();

        let pristine_path = config.pristines_dir.join("test-repo");
        fs::create_dir_all(&pristine_path).unwrap();

        // Create metadata
        let mut metadata = Metadata::new(vec!["url".to_string()]);
        metadata.mark_pristine_created();
        metadata.save("test-repo", &config).unwrap();

        // Destroy pristine
        let result = destroy_pristine("test-repo", &config);
        assert!(result.is_ok());
        assert!(!pristine_path.exists());

        // Vault entry should still exist
        let vault = Vault::load(&config).unwrap();
        assert!(vault.contains("test-repo"));

        // Metadata pristine_created should be cleared
        let metadata = Metadata::load("test-repo", &config).unwrap();
        assert!(metadata.pristine_created.is_none());
    }

    #[test]
    fn test_destroy_pristine_not_in_vault() {
        let (_temp, config) = create_test_config();

        let result = destroy_pristine("nonexistent", &config);
        assert!(result.is_err());
    }

    #[test]
    fn test_destroy_pristine_not_exists() {
        let (_temp, config) = create_test_config();

        // Add to vault but don't create pristine directory
        let mut vault = Vault::default();
        vault
            .add_entry("test-repo".to_string(), "url".to_string())
            .unwrap();
        vault.save(&config).unwrap();

        let result = destroy_pristine("test-repo", &config);
        assert!(result.is_err());
    }

    #[test]
    fn test_destroy_clone_by_suffix() {
        let (_temp, config) = create_test_config();

        // Set up vault and metadata with clone
        let mut vault = Vault::default();
        vault
            .add_entry("test-repo".to_string(), "url".to_string())
            .unwrap();
        vault.save(&config).unwrap();

        let clone_path = config.clones_dir.join("test-repo-abc123");
        fs::create_dir_all(&clone_path).unwrap();

        let mut metadata = Metadata::new(vec!["url".to_string()]);
        metadata.add_clone("abc123".to_string(), clone_path.clone());
        metadata.save("test-repo", &config).unwrap();

        // Destroy clone by suffix
        let result = destroy_clone("abc123", &config);
        assert!(result.is_ok());

        // Clone should be removed from metadata
        let metadata = Metadata::load("test-repo", &config).unwrap();
        assert!(metadata.get_clone("abc123").is_none());
    }

    #[test]
    fn test_destroy_clone_by_directory() {
        let (_temp, config) = create_test_config();

        // Create clone directory without metadata
        let clone_path = config.clones_dir.join("standalone-clone");
        fs::create_dir_all(&clone_path).unwrap();

        // Destroy by directory name
        let result = destroy_clone("standalone-clone", &config);
        assert!(result.is_ok());
        assert!(!clone_path.exists());
    }

    #[test]
    fn test_destroy_clone_not_found() {
        let (_temp, config) = create_test_config();

        let result = destroy_clone("nonexistent", &config);
        assert!(result.is_err());
    }

    #[test]
    fn test_destroy_target_detects_pristine() {
        let (_temp, config) = create_test_config();

        // Set up vault and pristine
        let mut vault = Vault::default();
        vault
            .add_entry("test-repo".to_string(), "url".to_string())
            .unwrap();
        vault.save(&config).unwrap();

        let pristine_path = config.pristines_dir.join("test-repo");
        fs::create_dir_all(&pristine_path).unwrap();

        // destroy_target should recognize this as a pristine
        let result = destroy_target("test-repo", &config);
        assert!(result.is_ok());
        assert!(!pristine_path.exists());
    }

    #[test]
    fn test_destroy_target_detects_clone_directory() {
        let (_temp, config) = create_test_config();

        let clone_path = config.clones_dir.join("some-clone");
        fs::create_dir_all(&clone_path).unwrap();

        let result = destroy_target("some-clone", &config);
        assert!(result.is_ok());
        assert!(!clone_path.exists());
    }

    #[test]
    fn test_destroy_target_not_found() {
        let (_temp, config) = create_test_config();

        let result = destroy_target("nonexistent", &config);
        assert!(result.is_err());
    }

    #[test]
    fn test_destroy_all_pristines_multiple() {
        let (_temp, config) = create_test_config();

        let mut vault = Vault::default();
        vault.add_entry("repo1".to_string(), "url1".to_string()).unwrap();
        vault.add_entry("repo2".to_string(), "url2".to_string()).unwrap();
        vault.save(&config).unwrap();

        let p1 = config.pristines_dir.join("repo1");
        let p2 = config.pristines_dir.join("repo2");
        fs::create_dir_all(&p1).unwrap();
        fs::create_dir_all(&p2).unwrap();

        // Create metadata with pristine_created set
        let mut m1 = Metadata::new(vec!["url1".to_string()]);
        m1.mark_pristine_created();
        m1.save("repo1", &config).unwrap();
        let mut m2 = Metadata::new(vec!["url2".to_string()]);
        m2.mark_pristine_created();
        m2.save("repo2", &config).unwrap();

        let result = destroy_all_pristines(&config).unwrap();
        assert_eq!(result.len(), 2);
        assert!(!p1.exists());
        assert!(!p2.exists());

        // Vault entries should still exist
        let vault = Vault::load(&config).unwrap();
        assert!(vault.contains("repo1"));
        assert!(vault.contains("repo2"));

        // pristine_created should be cleared
        let m1 = Metadata::load("repo1", &config).unwrap();
        assert!(m1.pristine_created.is_none());
    }

    #[test]
    fn test_destroy_all_pristines_empty_vault() {
        let (_temp, config) = create_test_config();

        // Empty vault
        let vault = Vault::default();
        vault.save(&config).unwrap();

        let result = destroy_all_pristines(&config).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_destroy_all_pristines_partial() {
        let (_temp, config) = create_test_config();

        let mut vault = Vault::default();
        vault.add_entry("repo1".to_string(), "url1".to_string()).unwrap();
        vault.add_entry("repo2".to_string(), "url2".to_string()).unwrap();
        vault.save(&config).unwrap();

        // Only create pristine for repo1
        let p1 = config.pristines_dir.join("repo1");
        fs::create_dir_all(&p1).unwrap();

        let result = destroy_all_pristines(&config).unwrap();
        assert_eq!(result.len(), 1);
        assert!(!p1.exists());
    }
}
