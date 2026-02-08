use crate::config::Config;
use crate::error::Result;
use crate::vault::Vault;

/// Add an alias for a repo
pub fn add_alias(alias: &str, repo_name: &str, config: &Config) -> Result<()> {
    let mut vault = Vault::load(config)?;
    vault.add_alias(alias.to_string(), repo_name.to_string())?;
    vault.save(config)?;
    Ok(())
}

/// Remove an alias
pub fn remove_alias(alias: &str, config: &Config) -> Result<()> {
    let mut vault = Vault::load(config)?;
    vault.remove_alias(alias)?;
    vault.save(config)?;
    Ok(())
}

/// List all aliases
pub fn list_aliases(config: &Config) -> Result<Vec<(String, String)>> {
    let vault = Vault::load(config)?;
    let mut aliases: Vec<(String, String)> = vault
        .list_aliases()
        .iter()
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    aliases.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(aliases)
}
