use tokio::task;

use crate::config::Config;
use crate::error::Result;
use crate::operations;

pub async fn handle_init(vault_name: Option<String>, config: &Config) -> Result<()> {
    match vault_name {
        Some(name) => {
            // Initialize single repo
            operations::init_pristine(&name, config)?;
        }
        None => {
            // Initialize all uninitialized repos in parallel
            let uninitialized = operations::get_uninitialized_repos(config)?;

            if uninitialized.is_empty() {
                println!("All vaulted repositories already have pristines");
                return Ok(());
            }

            println!(
                "Initializing {} repositories in parallel...",
                uninitialized.len()
            );

            let config = config.clone();
            let mut handles = Vec::new();

            for name in uninitialized {
                let config = config.clone();
                let handle = task::spawn_blocking(move || {
                    let result = operations::init_pristine(&name, &config);
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
                        println!("Failed to init {}: {}", name, e);
                        failures += 1;
                    }
                    Err(e) => {
                        println!("Task error: {}", e);
                        failures += 1;
                    }
                }
            }

            println!(
                "\nInit complete: {} succeeded, {} failed",
                successes, failures
            );
        }
    }

    Ok(())
}
