use crate::config::Config;
use crate::error::Result;
use crate::operations;

pub fn handle_destroy(
    target: Option<String>,
    all_clones: Option<String>,
    all_pristines: bool,
    stale: Option<u64>,
    config: &Config,
) -> Result<()> {
    if let Some(pristine_name) = all_clones {
        operations::destroy_all_clones(&pristine_name, config)?;
    } else if all_pristines {
        operations::destroy_all_pristines(config)?;
    } else if let Some(days) = stale {
        operations::destroy_stale_clones(days, config)?;
    } else if let Some(target) = target {
        operations::destroy_target(&target, config)?;
    } else {
        eprintln!("Error: provide a target, --all-clones <name>, --all-pristines, or --stale <days>");
        std::process::exit(1);
    }
    Ok(())
}
