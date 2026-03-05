use crate::config::Config;
use crate::error::Result;
use crate::operations;
use crate::util;

pub async fn handle_update(name: Option<String>, config: &Config) -> Result<()> {
    if let Some(name) = name {
        operations::update_repo(&name, config)?;
    } else {
        let updatable = operations::get_updatable_repos(config)?;

        if updatable.is_empty() {
            println!("No pristines to update. Run 'repoman init' first.");
            return Ok(());
        }

        println!("Updating {} repositories in parallel...", updatable.len());

        let config = config.clone();
        let max = config.max_parallel();
        let results = util::run_parallel(updatable, max, move |name| {
            operations::update_repo(name, &config)
        })
        .await;

        let mut successes = 0;
        let mut failures = 0;

        for (name, result) in results {
            match result {
                Ok(Ok(())) => successes += 1,
                Ok(Err(e)) => {
                    println!("Failed to update {}: {}", name, e);
                    failures += 1;
                }
                Err(e) => {
                    println!("Task error for {}: {}", name, e);
                    failures += 1;
                }
            }
        }

        println!(
            "\nUpdate complete: {} succeeded, {} failed",
            successes, failures
        );
    }

    Ok(())
}
