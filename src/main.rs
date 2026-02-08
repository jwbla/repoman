use clap::{Parser, Subcommand};
use log::{debug, error, info, LevelFilter};
use simplelog::{CombinedLogger, Config as LogConfig, SharedLogger, TermLogger, TerminalMode, WriteLogger};
use std::path::Path;

mod agent;
mod commands;
mod config;
mod error;
mod metadata;
mod operations;
mod vault;

use config::Config;

#[derive(Parser)]
#[command(name = "repoman")]
#[command(about = "A git repository manager with disposable workspaces")]
#[command(version)]
#[command(after_help = concat!("version ", env!("CARGO_PKG_VERSION")))]
struct Cli {
    /// Print debug logs to console
    #[arg(long, global = true)]
    debug: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Add repository to vault
    Add {
        /// Git URL to add. If not provided, detects from current directory.
        url: Option<String>,
    },

    /// Create pristine(s) of vaulted repository
    Init {
        /// Vault name to initialize. If not provided, initializes all.
        vault_name: Option<String>,
    },

    /// Create clone from a pristine
    Clone {
        /// Name of the pristine to clone from
        pristine: String,
        /// Optional name for the clone
        clone_name: Option<String>,
        /// Branch to check out (defaults to HEAD)
        #[arg(short, long)]
        branch: Option<String>,
    },

    /// Update pristine(s) from origin
    Sync {
        /// Pristine to sync. If not provided, syncs all.
        pristine: Option<String>,
    },

    /// Destroy target clone or pristine
    Destroy {
        /// Name of clone or pristine to destroy
        target: Option<String>,
        /// Destroy all clones for a given pristine
        #[arg(long)]
        all_clones: Option<String>,
        /// Destroy all pristines (keeps vault entries)
        #[arg(long)]
        all_pristines: bool,
        /// Destroy clones with HEAD older than N days
        #[arg(long)]
        stale: Option<u64>,
    },

    /// List all repositories and their status
    List {
        /// Show verbose details
        #[arg(short, long)]
        verbose: bool,
    },

    /// Start/stop or collect status info on background agent
    Agent {
        /// Action: start, stop, status, or run (internal)
        action: String,
    },

    /// Show detailed status for a repository
    Status {
        /// Repository name (or alias)
        name: String,
    },

    /// Print filesystem path for a pristine or clone
    Open {
        /// Pristine name, clone suffix, or clone directory name
        target: String,
    },

    /// Manage aliases for repository names
    Alias {
        /// Repository name to alias (omit to list all aliases)
        name: Option<String>,
        /// The alias to create
        alias: Option<String>,
        /// Remove the alias instead of creating it
        #[arg(short, long)]
        remove: bool,
    },

    /// Sync pristine and fast-forward all clones
    Update {
        /// Repository to update. If not provided, updates all.
        name: Option<String>,
    },

    /// Garbage-collect stale clones and compact pristines
    Gc {
        /// Remove clones with HEAD older than this many days
        #[arg(long, default_value = "30")]
        days: u64,
        /// Show what would be done without making changes
        #[arg(long)]
        dry_run: bool,
    },

    /// Remove repository from vault and delete all its data
    Remove {
        /// Repository name (or alias)
        name: String,
    },
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    let config = Config::load();
    init_logging(&config, cli.debug);

    if let Err(e) = run(cli, config).await {
        error!("{}", e);
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

fn init_logging(config: &Config, verbose: bool) {
    let mut loggers: Vec<Box<dyn SharedLogger>> = Vec::new();

    // Terminal logger: only when -v flag is passed
    if verbose {
        loggers.push(TermLogger::new(
            LevelFilter::Debug,
            LogConfig::default(),
            TerminalMode::Stderr,
            simplelog::ColorChoice::Auto,
        ));
    }

    // File logger: always enabled at debug level for diagnosing issues
    let log_path = config.logs_dir.join("repoman.log");
    if let Ok(()) = std::fs::create_dir_all(&config.logs_dir) {
        if let Ok(file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)
        {
            loggers.push(WriteLogger::new(LevelFilter::Debug, LogConfig::default(), file));
        }
    }

    if !loggers.is_empty() {
        let _ = CombinedLogger::init(loggers);
    }
}

async fn run(cli: Cli, config: Config) -> Result<(), Box<dyn std::error::Error>> {
    ensure_dirs(&config)?;

    match cli.command {
        Commands::Add { ref url } => {
            info!("command: add (url={:?})", url);
            commands::handle_add(url.clone(), &config)?;
        }
        Commands::Init { ref vault_name } => {
            info!("command: init (vault_name={:?})", vault_name);
            commands::handle_init(vault_name.clone(), &config).await?;
        }
        Commands::Clone {
            ref pristine,
            ref clone_name,
            ref branch,
        } => {
            info!("command: clone (pristine={}, clone_name={:?}, branch={:?})", pristine, clone_name, branch);
            commands::handle_clone(pristine, clone_name.clone(), branch.clone(), &config)?;
        }
        Commands::Sync { ref pristine } => {
            info!("command: sync (pristine={:?})", pristine);
            commands::handle_sync(pristine.clone(), &config).await?;
        }
        Commands::Destroy {
            ref target,
            ref all_clones,
            all_pristines,
            ref stale,
        } => {
            info!("command: destroy (target={:?}, all_clones={:?}, all_pristines={}, stale={:?})", target, all_clones, all_pristines, stale);
            commands::handle_destroy(target.clone(), all_clones.clone(), all_pristines, *stale, &config)?;
        }
        Commands::List { verbose } => {
            debug!("command: list (verbose={})", verbose);
            commands::handle_list(verbose, &config)?;
        }
        Commands::Agent { ref action } => {
            info!("command: agent (action={})", action);
            if action == "run" {
                agent::run_agent_loop(&config).await?;
            } else {
                commands::handle_agent(action, &config)?;
            }
        }
        Commands::Status { ref name } => {
            info!("command: status (name={})", name);
            commands::handle_status(name, &config)?;
        }
        Commands::Open { ref target } => {
            info!("command: open (target={})", target);
            commands::handle_open(target, &config)?;
        }
        Commands::Alias {
            ref name,
            ref alias,
            remove,
        } => {
            if let (Some(name), Some(alias)) = (name, alias) {
                info!("command: alias (name={}, alias={}, remove={})", name, alias, remove);
                commands::handle_alias(name, alias, remove, &config)?;
            } else {
                info!("command: alias list");
                commands::handle_alias_list(&config)?;
            }
        }
        Commands::Update { ref name } => {
            info!("command: update (name={:?})", name);
            commands::handle_update(name.clone(), &config).await?;
        }
        Commands::Gc { days, dry_run } => {
            info!("command: gc (days={}, dry_run={})", days, dry_run);
            commands::handle_gc(days, dry_run, &config)?;
        }
        Commands::Remove { ref name } => {
            info!("command: remove (name={})", name);
            commands::handle_remove(name, &config)?;
        }
    }

    Ok(())
}

fn ensure_dirs(config: &Config) -> Result<(), Box<dyn std::error::Error>> {
    create_dir_if_needed(&config.vault_dir)?;
    create_dir_if_needed(&config.pristines_dir)?;
    create_dir_if_needed(&config.clones_dir)?;
    create_dir_if_needed(&config.plugins_dir)?;
    create_dir_if_needed(&config.logs_dir)?;
    Ok(())
}

fn create_dir_if_needed(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    if !path.exists() {
        std::fs::create_dir_all(path)?;
    }
    Ok(())
}
