use crate::config::Config;
use crate::error::Result;
use crate::operations;
use crate::util;

pub async fn handle_sync(pristine: Option<String>, config: &Config) -> Result<()> {
    if let Some(name) = pristine {
        // Sync single repo
        operations::sync_pristine(&name, config)?;
    } else {
        // Sync all repos with pristines in parallel
        let syncable = operations::get_syncable_repos(config)?;

        if syncable.is_empty() {
            println!("No pristines to sync. Run 'repoman init' first.");
            return Ok(());
        }

        println!("Syncing {} repositories in parallel...", syncable.len());

        let config = config.clone();
        let max = config.max_parallel();
        let results = util::run_parallel(syncable, max, move |name| {
            operations::sync_pristine(name, &config)
        })
        .await;

        let mut successes = 0;
        let mut failures = 0;

        for (name, result) in results {
            match result {
                Ok(Ok(())) => successes += 1,
                Ok(Err(e)) => {
                    println!("Failed to sync {}: {}", name, e);
                    failures += 1;
                }
                Err(e) => {
                    println!("Task error for {}: {}", name, e);
                    failures += 1;
                }
            }
        }

        println!(
            "\nSync complete: {} succeeded, {} failed",
            successes, failures
        );
    }

    Ok(())
}
