use crate::config::Config;
use crate::error::Result;
use crate::operations;

pub fn handle_open(target: &str, config: &Config) -> Result<()> {
    let path = operations::find_path(target, config)?;
    // Print path only to stdout — designed for `cd $(repoman open foo)`
    println!("{}", path.display());
    Ok(())
}
