use crate::config::Config;
use crate::error::Result;
use crate::operations;
use crate::util;

pub fn handle_gc(days: u64, dry_run: bool, confirmed: bool, config: &Config) -> Result<()> {
    if dry_run {
        // Dry-run mode: just show what would be done
        let report = operations::run_gc(days, true, config)?;
        print_gc_report(&report, days, true);
        return Ok(());
    }

    if !confirmed {
        // Preview what will be done first
        let report = operations::run_gc(days, true, config)?;
        print_gc_report(&report, days, true);

        if report.stale_clones.is_empty() {
            return Ok(());
        }

        if !util::confirm("Proceed with garbage collection?") {
            println!("Aborted.");
            return Ok(());
        }
    }

    let report = operations::run_gc(days, false, config)?;
    print_gc_report(&report, days, false);

    Ok(())
}

fn print_gc_report(report: &operations::gc::GcReport, days: u64, dry_run: bool) {
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

    println!("{}Pristines GC'd: {}", prefix, report.pristines_gc_run);
}
