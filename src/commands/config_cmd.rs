use colored::Colorize;

use crate::config::Config;
use crate::error::Result;

pub fn handle_config(action: Option<&str>, config: &Config) -> Result<()> {
    match action {
        Some("path") => {
            let config_path = dirs::config_dir()
                .map(|p| p.join("repoman").join("config.yaml"))
                .unwrap_or_default();
            println!("{}", config_path.display());
        }
        Some("validate") => {
            let config_path = dirs::config_dir().map(|p| p.join("repoman").join("config.yaml"));
            match config_path {
                Some(ref p) if p.exists() => {
                    let contents = std::fs::read_to_string(p)?;
                    match serde_yml::from_str::<Config>(&contents) {
                        Ok(_) => println!("{} Configuration is valid", "OK".green()),
                        Err(e) => println!("{} Configuration error: {}", "ERROR".red(), e),
                    }
                }
                _ => println!("No config file found (using defaults)"),
            }
        }
        Some("init") => {
            let config_path = dirs::config_dir().map(|p| p.join("repoman").join("config.yaml"));
            if let Some(ref p) = config_path {
                if p.exists() {
                    println!("Config file already exists: {}", p.display());
                } else {
                    std::fs::create_dir_all(p.parent().unwrap())?;
                    let default_yaml = serde_yml::to_string(config)
                        .map_err(|e| crate::error::RepomanError::ConfigError(e.to_string()))?;
                    std::fs::write(p, default_yaml)?;
                    println!("Created config file: {}", p.display());
                }
            }
        }
        _ => {
            // Print effective config
            println!("{}", "Effective Configuration:".bold());
            println!("  vault_dir:     {}", config.vault_dir.display());
            println!("  pristines_dir: {}", config.pristines_dir.display());
            println!("  clones_dir:    {}", config.clones_dir.display());
            println!("  plugins_dir:   {}", config.plugins_dir.display());
            println!("  logs_dir:      {}", config.logs_dir.display());
            println!(
                "  agent_heartbeat_interval: {}s",
                config.agent_heartbeat_interval.unwrap_or(300)
            );
            println!("  json_output:   {}", config.json_output.unwrap_or(false));
            if let Some(ref repos) = config.repos {
                println!("  repos:");
                for (name, rc) in repos {
                    println!("    {}:", name.cyan());
                    if let Some(si) = rc.sync_interval {
                        println!("      sync_interval: {}s", si);
                    }
                    if let Some(ref db) = rc.default_branch {
                        println!("      default_branch: {}", db);
                    }
                    if rc.hooks.is_some() {
                        println!("      hooks: configured");
                    }
                    if rc.build.is_some() {
                        println!("      build: configured");
                    }
                    if rc.auto_init == Some(true) {
                        println!("      auto_init: true");
                    }
                    if let Some(ref tags) = rc.tags {
                        println!("      tags: {}", tags.join(", "));
                    }
                }
            }
        }
    }
    Ok(())
}
