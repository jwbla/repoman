use crate::config::Config;
use crate::error::{RepomanError, Result};
use crate::metadata::Metadata;
use crate::vault::Vault;

pub fn handle_rename(old_name: &str, new_name: &str, config: &Config) -> Result<()> {
    let mut vault = Vault::load(config)?;

    // Resolve alias
    let canonical = vault.resolve_name(old_name).to_string();
    if !vault.contains(&canonical) {
        return Err(RepomanError::RepoNotInVault(canonical));
    }

    // Check new name doesn't conflict
    if vault.contains(new_name) {
        return Err(RepomanError::RepoAlreadyInVault(new_name.to_string()));
    }

    // Load metadata
    let metadata = Metadata::load(&canonical, config)?;

    // Save metadata under new name
    metadata.save(new_name, config)?;

    // Remove old metadata directory
    let old_metadata_dir = config.vault_dir.join(&canonical);
    if old_metadata_dir.exists() {
        std::fs::remove_dir_all(&old_metadata_dir)?;
    }

    // Rename pristine directory if it exists
    let old_pristine = config.pristines_dir.join(&canonical);
    let new_pristine = config.pristines_dir.join(new_name);
    if old_pristine.exists() {
        std::fs::rename(&old_pristine, &new_pristine)?;
    }

    // Update vault entry
    if let Some(entry) = vault.entries.iter_mut().find(|e| e.name == canonical) {
        entry.name = new_name.to_string();
    }

    // Update aliases pointing to old name
    let aliases_to_update: Vec<String> = vault
        .aliases
        .iter()
        .filter(|(_, target)| target.as_str() == canonical)
        .map(|(alias, _)| alias.clone())
        .collect();
    for alias in aliases_to_update {
        vault.aliases.insert(alias, new_name.to_string());
    }

    vault.save(config)?;

    println!("Renamed '{}' to '{}'", canonical, new_name);
    Ok(())
}
