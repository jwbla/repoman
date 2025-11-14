use clap::{Parser, Subcommand};
use std::path::Path;
mod config;
use config::Config;

#[derive(Parser)]
#[command(name = "repoman")]
#[command(about = "A git repository manager")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Add {
        url: Option<String>,
    },

    Init {
        vault_name: Option<String>,
    },

    Clone {
        pristine: String,
        clone_name: Option<String>,
    },

    Sync {
        pristine: Option<String>,
    },

    Destroy {
        target: String,
    },

    Agent {
        action: String,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    let config = Config::load();

    check_config_and_dirs(&config)?;

    match cli.command {
        Commands::Add { url } => {
            println!("Add command - forthcoming");
            if let Some(url) = url {
                println!("URL: {}", url);
            }
        }
        Commands::Init { vault_name } => {
            println!("Init command - forthcoming");
            if let Some(name) = vault_name {
                println!("Vault name: {}", name);
            }
        }
        Commands::Clone { pristine, clone_name } => {
            println!("Clone command - forthcoming");
            println!("Pristine: {}", pristine);
            if let Some(name) = clone_name {
                println!("Clone name: {}", name);
            }
        }
        Commands::Sync { pristine } => {
            println!("Sync command - forthcoming");
            if let Some(name) = pristine {
                println!("Pristine: {}", name);
            }
        }
        Commands::Destroy { target } => {
            println!("Destroy command - forthcoming");
            println!("Target: {}", target);
        }
        Commands::Agent { action } => {
            println!("Agent command - forthcoming");
            println!("Action: {}", action);
        }
    }

    Ok(())
}

fn check_config_and_dirs (config: &Config) -> Result<(), Box<dyn std::error::Error>> {
    let config_path = dirs::config_dir()
        .map(|p| p.join("repoman").join("config.yaml"))
        .ok_or("Could not determine config directory")?;

    if config_path.exists() {
        println!("Config file found: {}", config_path.display());
    } else {
        println!("Config file NOT found: {}", config_path.display());
    }

    create_dir_if_needed("vault", &config.vault_dir)?;
    create_dir_if_needed("pristines", &config.pristines_dir)?;
    create_dir_if_needed("clones", &config.clones_dir)?;
    create_dir_if_needed("plugins", &config.plugins_dir)?;
    create_dir_if_needed("logs", &config.logs_dir)?;

    Ok(())
}

fn create_dir_if_needed(name: &str, path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    if path.exists() {
        println!(" {} directory found: {}", name, path.display());
    } else {
        std::fs::create_dir_all(path)?;
        println!(" {} directory created: {}", name, path.display());
    }
    Ok(())
}
