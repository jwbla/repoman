use crate::config::Config;
use crate::error::Result;
use crate::operations;

pub fn handle_status(name: &str, config: &Config) -> Result<()> {
    let status = operations::get_detailed_status(name, config)?;
    println!("{}", status);
    Ok(())
}
