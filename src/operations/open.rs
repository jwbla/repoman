use log::debug;
use std::path::PathBuf;

use crate::config::Config;
use crate::error::{RepomanError, Result};
use crate::metadata::Metadata;
use crate::vault::Vault;

/// Find the filesystem path for a target (pristine name, clone suffix, or full clone dir name).
/// Search order: pristine names -> clone suffixes in metadata -> full clone dir names.
pub fn find_path(target: &str, config: &Config) -> Result<PathBuf> {
    let vault = Vault::load(config)?;
    let resolved = vault.resolve_name(target);

    // 1. Check if it's a pristine name
    let pristine_path = config.pristines_dir.join(resolved);
    if pristine_path.exists() {
        debug!("find_path: '{}' resolved to pristine {}", target, pristine_path.display());
        return Ok(pristine_path);
    }

    // 2. Check clone suffixes in metadata
    for repo_name in vault.get_all_names() {
        if let Ok(metadata) = Metadata::load(repo_name, config)
            && let Some(clone_entry) = metadata.get_clone(target)
            && clone_entry.path.exists()
        {
            debug!("find_path: '{}' resolved to clone {}", target, clone_entry.path.display());
            return Ok(clone_entry.path.clone());
        }
    }

    // 3. Check full clone directory names
    let clone_path = config.clones_dir.join(target);
    if clone_path.exists() {
        debug!("find_path: '{}' resolved to clone dir {}", target, clone_path.display());
        return Ok(clone_path);
    }

    Err(RepomanError::CloneNotFound(target.to_string()))
}
