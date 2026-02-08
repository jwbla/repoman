use crate::config::Config;
use crate::error::Result;
use crate::operations;

pub fn handle_clone(
    pristine: &str,
    clone_name: Option<String>,
    branch: Option<String>,
    config: &Config,
) -> Result<()> {
    let clone_path = operations::clone_from_pristine(pristine, clone_name, branch, config)?;
    println!("Clone created at: {}", clone_path.display());
    Ok(())
}
