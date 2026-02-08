use tokio::task;

use crate::config::Config;
use crate::error::Result;
use crate::operations;

pub async fn handle_update(name: Option<String>, config: &Config) -> Result<()> {
    match name {
        Some(name) => {
            operations::update_repo(&name, config)?;
        }
        None => {
            let updatable = operations::get_updatable_repos(config)?;

            if updatable.is_empty() {
                println!("No pristines to update. Run 'repoman init' first.");
                return Ok(());
            }

            println!("Updating {} repositories in parallel...", updatable.len());

            let config = config.clone();
            let mut handles = Vec::new();

            for name in updatable {
                let config = config.clone();
                let handle = task::spawn_blocking(move || {
                    let result = operations::update_repo(&name, &config);
                    (name, result)
                });
                handles.push(handle);
            }

            let mut successes = 0;
            let mut failures = 0;

            for handle in handles {
                match handle.await {
                    Ok((_name, Ok(_))) => {
                        successes += 1;
                    }
                    Ok((name, Err(e))) => {
                        println!("Failed to update {}: {}", name, e);
                        failures += 1;
                    }
                    Err(e) => {
                        println!("Task error: {}", e);
                        failures += 1;
                    }
                }
            }

            println!(
                "\nUpdate complete: {} succeeded, {} failed",
                successes, failures
            );
        }
    }

    Ok(())
}
