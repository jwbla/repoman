use crate::config::Config;
use crate::error::Result;
use crate::operations;

pub fn handle_export(config: &Config) -> Result<()> {
    let yaml = operations::export_vault(config)?;
    print!("{}", yaml);
    Ok(())
}

pub fn handle_import(path: &str, config: &Config) -> Result<()> {
    let count = operations::import_vault(path, config)?;
    println!("Imported {} repositories", count);
    Ok(())
}
