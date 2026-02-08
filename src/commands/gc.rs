use crate::config::Config;
use crate::error::Result;
use crate::operations;

pub fn handle_gc(days: u64, dry_run: bool, config: &Config) -> Result<()> {
    let report = operations::run_gc(days, dry_run, config)?;

    let prefix = if dry_run { "[dry-run] " } else { "" };

    if report.stale_clones.is_empty() {
        println!("{}No stale clones found (threshold: {} days)", prefix, days);
    } else {
        println!("{}Stale clones ({}):", prefix, report.stale_clones.len());
        for sc in &report.stale_clones {
            println!(
                "  {} ({}) — {} days old",
                sc.clone_name, sc.repo_name, sc.days_old
            );
        }
    }

    println!(
        "{}Pristines GC'd: {}",
        prefix, report.pristines_gc_run
    );

    Ok(())
}
