use crate::config::Config;
use crate::error::Result;
use crate::operations;
use crate::util;

pub fn handle_remove(name: &str, confirmed: bool, config: &Config) -> Result<()> {
    if !confirmed
        && !util::confirm(&format!(
            "Remove '{}' and all its data (pristine, clones, metadata)?",
            name
        ))
    {
        println!("Aborted.");
        return Ok(());
    }
    operations::remove_repo(name, config)
}
