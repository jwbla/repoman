use crate::config::Config;
use crate::error::Result;
use crate::operations;

pub fn handle_remove(name: &str, config: &Config) -> Result<()> {
    operations::remove_repo(name, config)
}
