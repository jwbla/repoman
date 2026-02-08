use crate::config::Config;
use crate::error::Result;
use crate::operations;

pub fn handle_add(url: Option<String>, config: &Config) -> Result<()> {
    let repo_name = operations::add_repo(url, config)?;
    println!("Repository '{}' added to vault", repo_name);
    Ok(())
}
