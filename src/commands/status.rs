use crate::config::Config;
use crate::error::Result;
use crate::operations;

pub fn handle_status(name: &str, json: bool, config: &Config) -> Result<()> {
    let status = operations::get_detailed_status(name, config)?;

    if json {
        let json_str = serde_json::to_string_pretty(&status)
            .map_err(|e| crate::error::RepomanError::ConfigError(e.to_string()))?;
        println!("{}", json_str);
    } else {
        println!("{}", status);
    }

    Ok(())
}
