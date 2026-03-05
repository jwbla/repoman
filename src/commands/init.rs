use crate::config::Config;
use crate::error::Result;
use crate::operations;
use crate::util;

pub async fn handle_init(
    vault_name: Option<String>,
    depth: Option<i32>,
    config: &Config,
) -> Result<()> {
    if let Some(name) = vault_name {
        // Initialize single repo
        operations::init_pristine(&name, depth, config)?;
    } else {
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
        let max = config.max_parallel();
        let results = util::run_parallel(uninitialized, max, move |name| {
            // For parallel init-all, use CLI depth or fall back to per-repo config
            operations::init_pristine(name, depth, &config)
        })
        .await;

        let mut successes = 0;
        let mut failures = 0;

        for (name, result) in results {
            match result {
                Ok(Ok(_)) => successes += 1,
                Ok(Err(e)) => {
                    println!("Failed to init {}: {}", name, e);
                    failures += 1;
                }
                Err(e) => {
                    println!("Task error for {}: {}", name, e);
                    failures += 1;
                }
            }
        }

        println!(
            "\nInit complete: {} succeeded, {} failed",
            successes, failures
        );
    }

    Ok(())
}
