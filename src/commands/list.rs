use crate::config::Config;
use crate::error::Result;
use crate::operations::{format_repo_status, format_summary, list_all_repos};

pub fn handle_list(verbose: bool, config: &Config) -> Result<()> {
    let statuses = list_all_repos(config)?;

    if verbose {
        println!("\nRepository Details:\n");
        for status in &statuses {
            println!("{}", format_repo_status(status));
        }
    } else {
        println!("\n{}", format_summary(&statuses));
    }

    Ok(())
}
