use crate::config::Config;
use crate::error::Result;
use crate::operations::{format_repo_status, format_summary, list_all_repos};

pub fn handle_list(verbose: bool, json: bool, config: &Config) -> Result<()> {
    let statuses = list_all_repos(config)?;

    if json {
        let json_str = serde_json::to_string_pretty(&statuses)
            .map_err(|e| crate::error::RepomanError::ConfigError(e.to_string()))?;
        println!("{}", json_str);
    } else if verbose {
        println!("\nRepository Details:\n");
        for status in &statuses {
            println!("{}", format_repo_status(status));
        }
    } else {
        println!("\n{}", format_summary(&statuses));
    }

    Ok(())
}
