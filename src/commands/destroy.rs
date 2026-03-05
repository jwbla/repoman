use crate::config::Config;
use crate::error::Result;
use crate::operations;
use crate::util;

pub fn handle_destroy(
    target: Option<String>,
    all_clones: Option<String>,
    all_pristines: bool,
    stale: Option<u64>,
    confirmed: bool,
    config: &Config,
) -> Result<()> {
    if let Some(pristine_name) = all_clones {
        if !confirmed && !util::confirm(&format!("Destroy all clones for '{}'?", pristine_name)) {
            println!("Aborted.");
            return Ok(());
        }
        operations::destroy_all_clones(&pristine_name, config)?;
    } else if all_pristines {
        if !confirmed && !util::confirm("Destroy ALL pristines? (vault entries will be kept)") {
            println!("Aborted.");
            return Ok(());
        }
        operations::destroy_all_pristines(config)?;
    } else if let Some(days) = stale {
        if !confirmed
            && !util::confirm(&format!(
                "Destroy clones with HEAD older than {} days?",
                days
            ))
        {
            println!("Aborted.");
            return Ok(());
        }
        operations::destroy_stale_clones(days, config)?;
    } else if let Some(target) = target {
        if !confirmed && !util::confirm(&format!("Destroy '{}'?", target)) {
            println!("Aborted.");
            return Ok(());
        }
        operations::destroy_target(&target, config)?;
    } else {
        eprintln!(
            "Error: provide a target, --all-clones <name>, --all-pristines, or --stale <days>"
        );
        std::process::exit(1);
    }
    Ok(())
}
