use crate::config::Config;
use crate::error::Result;
use crate::operations;

pub fn handle_alias(name: &str, alias: &str, remove: bool, config: &Config) -> Result<()> {
    if remove {
        operations::remove_alias(alias, config)?;
        println!("Alias '{}' removed", alias);
    } else {
        operations::add_alias(alias, name, config)?;
        println!("Alias '{}' -> '{}'", alias, name);
    }
    Ok(())
}

pub fn handle_alias_list(config: &Config) -> Result<()> {
    let aliases = operations::list_aliases(config)?;
    if aliases.is_empty() {
        println!("No aliases defined");
    } else {
        println!("Aliases:");
        for (alias, target) in &aliases {
            println!("  {} -> {}", alias, target);
        }
    }
    Ok(())
}
