use tokio::task;

use crate::config::Config;
use crate::error::Result;
use crate::operations;

pub async fn handle_sync(pristine: Option<String>, config: &Config) -> Result<()> {
    match pristine {
        Some(name) => {
            // Sync single repo
            operations::sync_pristine(&name, config)?;
        }
        None => {
            // Sync all repos with pristines in parallel
            let syncable = operations::get_syncable_repos(config)?;

            if syncable.is_empty() {
                println!("No pristines to sync. Run 'repoman init' first.");
                return Ok(());
            }

            println!("Syncing {} repositories in parallel...", syncable.len());

            let config = config.clone();
            let mut handles = Vec::new();

            for name in syncable {
                let config = config.clone();
                let handle = task::spawn_blocking(move || {
                    let result = operations::sync_pristine(&name, &config);
                    (name, result)
                });
                handles.push(handle);
            }

            // Wait for all tasks
            let mut successes = 0;
            let mut failures = 0;

            for handle in handles {
                match handle.await {
                    Ok((_name, Ok(_))) => {
                        successes += 1;
                    }
                    Ok((name, Err(e))) => {
                        println!("Failed to sync {}: {}", name, e);
                        failures += 1;
                    }
                    Err(e) => {
                        println!("Task error: {}", e);
                        failures += 1;
                    }
                }
            }

            println!(
                "\nSync complete: {} succeeded, {} failed",
                successes, failures
            );
        }
    }

    Ok(())
}
