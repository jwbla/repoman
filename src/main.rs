#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]
#![allow(clippy::uninlined_format_args)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::doc_markdown)]
#![allow(clippy::too_many_lines)]
#![allow(clippy::cast_possible_wrap)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::items_after_statements)]
#![allow(clippy::format_push_string)]
#![allow(clippy::manual_let_else)]
#![allow(clippy::needless_pass_by_value)]
#![allow(clippy::match_same_arms)]

use clap::{CommandFactory, Parser, Subcommand};
use log::{LevelFilter, debug, error, info, warn};
use simplelog::{
    CombinedLogger, Config as LogConfig, SharedLogger, TermLogger, TerminalMode, WriteLogger,
};
use std::path::Path;

mod agent;
mod commands;
mod config;
mod dashboard;
mod error;
mod hooks;
mod mcp;
mod metadata;
mod operations;
mod plugins;
mod util;
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

    /// Output in JSON format (for list and status commands)
    #[arg(long, global = true)]
    json: bool,

    /// Skip confirmation prompts (assume yes)
    #[arg(short = 'y', long, global = true)]
    yes: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum AgentAction {
    /// Start the background agent
    Start,
    /// Stop the background agent
    Stop,
    /// Show agent status
    Status,
    /// Run agent loop (internal, not for direct use)
    Run,
}

#[derive(Subcommand)]
enum ConfigAction {
    /// Show effective configuration
    Show,
    /// Print config file path
    Path,
    /// Validate config file
    Validate,
    /// Create default config file
    Init,
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
        /// Shallow clone depth (number of commits to fetch)
        #[arg(long)]
        depth: Option<i32>,
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
        /// Skip confirmation prompt
        #[arg(short = 'y', long)]
        yes: bool,
    },

    /// List all repositories and their status
    #[command(visible_alias = "ls")]
    List {
        /// Show verbose details
        #[arg(short, long)]
        verbose: bool,
    },

    /// Background agent management
    Agent {
        #[command(subcommand)]
        action: AgentAction,
    },

    /// Show detailed status for a repository
    Status {
        /// Repository name (or alias)
        name: String,
    },

    /// Print filesystem path for a pristine or clone
    Open {
        /// Pristine name, clone suffix, or clone directory name (omit for picker)
        target: Option<String>,
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
        /// Skip confirmation prompt
        #[arg(short = 'y', long)]
        yes: bool,
    },

    /// Generate shell completions
    Completions {
        /// Shell: bash, zsh, fish, elvish, powershell
        shell: clap_complete::Shell,
    },

    /// Output shell completions and a wrapper function for eval
    #[command(name = "shell-init")]
    ShellInit {
        /// Shell: bash, zsh, fish, elvish, powershell
        shell: clap_complete::Shell,
    },

    /// Export vault to YAML
    Export,

    /// Import repositories from YAML file
    Import {
        /// Path to YAML file
        path: String,
    },

    /// Interactive TUI dashboard
    Dashboard,

    /// View and manage configuration
    #[command(name = "config")]
    Config {
        #[command(subcommand)]
        action: Option<ConfigAction>,
    },

    /// Run health checks on the repoman installation
    Doctor,

    /// Init missing pristines and sync existing ones in one pass
    Refresh,

    /// Rename a vault entry
    Rename {
        /// Current name (or alias)
        old_name: String,
        /// New name
        new_name: String,
    },

    /// Start MCP server for LLM agent integration
    Mcp,

    /// Check for and install the latest release from GitHub
    Upgrade,

    /// Generate man page
    #[command(name = "man")]
    ManPage,
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    let config = Config::load();
    init_logging(&config, cli.debug);

    if let Err(e) = run(cli, config).await {
        error!("{}", e);
        eprintln!("Error: {}", e);

        // Suggest similar repo names for "not in vault" errors
        if let Some(e) = e.downcast_ref::<error::RepomanError>()
            && let error::RepomanError::RepoNotInVault(name) = e
        {
            let cfg = config::Config::load();
            if let Ok(vault) = vault::Vault::load(&cfg) {
                let names: Vec<&str> = vault.get_all_names();
                if let Some(suggestion) = util::suggest_similar(name, &names) {
                    eprintln!("  Did you mean '{}'?", suggestion);
                }
            }
        }

        std::process::exit(1);
    }
}

fn init_logging(config: &Config, verbose: bool) {
    let mut loggers: Vec<Box<dyn SharedLogger>> = Vec::new();

    // Terminal logger: only when --debug flag is passed
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
    if let Ok(()) = std::fs::create_dir_all(&config.logs_dir)
        && let Ok(file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)
    {
        loggers.push(WriteLogger::new(
            LevelFilter::Debug,
            LogConfig::default(),
            file,
        ));
    }

    if !loggers.is_empty() {
        let _ = CombinedLogger::init(loggers);
    }
}

/// Initialize the plugin manager: load Lua plugins from the plugins directory.
/// Skips vault loading entirely if no .lua files exist in the plugins dir.
fn init_plugins(config: &Config) -> Option<plugins::PluginManager> {
    // Skip if no .lua files exist (avoids unnecessary vault load)
    let has_plugins = config.plugins_dir.is_dir()
        && std::fs::read_dir(&config.plugins_dir)
            .ok()
            .is_some_and(|entries| {
                entries
                    .flatten()
                    .any(|e| e.path().extension().is_some_and(|ext| ext == "lua"))
            });

    if !has_plugins {
        return None;
    }

    let vault = vault::Vault::load(config).ok();
    match plugins::PluginManager::new(vault.as_ref()) {
        Ok(mut pm) => {
            if let Err(e) = pm.load_plugins(&config.plugins_dir) {
                warn!("Failed to load plugins: {}", e);
            }
            let count = pm.list_loaded().len();
            if count > 0 {
                debug!("Loaded {} plugin(s)", count);
            }
            Some(pm)
        }
        Err(e) => {
            warn!("Failed to initialize plugin manager: {}", e);
            None
        }
    }
}

async fn run(cli: Cli, config: Config) -> Result<(), Box<dyn std::error::Error>> {
    ensure_dirs(&config)?;

    // Merge JSON flag: CLI --json overrides config default
    let json = config.json_enabled(cli.json);
    let skip_confirm = cli.yes;

    // Initialize plugins for commands that use hooks
    // (lazy: only init when needed, not for completions/config/etc.)
    let needs_plugins = !matches!(
        cli.command,
        Commands::Completions { .. }
            | Commands::ShellInit { .. }
            | Commands::Config { .. }
            | Commands::Doctor
            | Commands::Upgrade
            | Commands::Mcp
            | Commands::ManPage
    );
    let plugin_manager = if needs_plugins {
        init_plugins(&config)
    } else {
        None
    };

    // Store plugin_manager in hooks module for use by hook runners
    if let Some(ref pm) = plugin_manager {
        hooks::set_plugin_manager(pm);
    }

    match cli.command {
        Commands::Add { ref url } => {
            info!("command: add (url={:?})", url);
            commands::handle_add(url.clone(), &config)?;
        }
        Commands::Init {
            ref vault_name,
            depth,
        } => {
            info!(
                "command: init (vault_name={:?}, depth={:?})",
                vault_name, depth
            );
            commands::handle_init(vault_name.clone(), depth, &config).await?;
        }
        Commands::Clone {
            ref pristine,
            ref clone_name,
            ref branch,
        } => {
            info!(
                "command: clone (pristine={}, clone_name={:?}, branch={:?})",
                pristine, clone_name, branch
            );
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
            yes,
        } => {
            info!(
                "command: destroy (target={:?}, all_clones={:?}, all_pristines={}, stale={:?})",
                target, all_clones, all_pristines, stale
            );
            let confirmed = yes || skip_confirm;
            commands::handle_destroy(
                target.clone(),
                all_clones.clone(),
                all_pristines,
                *stale,
                confirmed,
                &config,
            )?;
        }
        Commands::List { verbose } => {
            debug!("command: list (verbose={})", verbose);
            commands::handle_list(verbose, json, &config)?;
        }
        Commands::Agent { ref action } => match action {
            AgentAction::Run => {
                info!("command: agent run");
                agent::run_agent_loop(&config).await?;
            }
            AgentAction::Start => {
                info!("command: agent start");
                commands::handle_agent("start", &config)?;
            }
            AgentAction::Stop => {
                info!("command: agent stop");
                commands::handle_agent("stop", &config)?;
            }
            AgentAction::Status => {
                info!("command: agent status");
                commands::handle_agent("status", &config)?;
            }
        },
        Commands::Status { ref name } => {
            info!("command: status (name={})", name);
            commands::handle_status(name, json, &config)?;
        }
        Commands::Open { ref target } => {
            info!("command: open (target={:?})", target);
            commands::handle_open(target.as_deref(), &config)?;
        }
        Commands::Alias {
            ref name,
            ref alias,
            remove,
        } => {
            if let (Some(name), Some(alias)) = (name, alias) {
                info!(
                    "command: alias (name={}, alias={}, remove={})",
                    name, alias, remove
                );
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
            commands::handle_gc(days, dry_run, skip_confirm, &config)?;
        }
        Commands::Remove { ref name, yes } => {
            info!("command: remove (name={})", name);
            let confirmed = yes || skip_confirm;
            commands::handle_remove(name, confirmed, &config)?;
        }
        Commands::Completions { shell } => {
            clap_complete::generate(
                shell,
                &mut Cli::command(),
                "repoman",
                &mut std::io::stdout(),
            );
        }
        Commands::ShellInit { shell } => {
            commands::handle_shell_init(shell, &mut Cli::command());
        }
        Commands::Export => {
            info!("command: export");
            commands::handle_export(&config)?;
        }
        Commands::Import { ref path } => {
            info!("command: import (path={})", path);
            commands::handle_import(path, &config)?;
        }
        Commands::Dashboard => {
            info!("command: dashboard");
            dashboard::run_dashboard(&config)?;
        }
        Commands::Config { ref action } => {
            info!("command: config");
            let action_str = match action {
                Some(ConfigAction::Show) | None => None,
                Some(ConfigAction::Path) => Some("path"),
                Some(ConfigAction::Validate) => Some("validate"),
                Some(ConfigAction::Init) => Some("init"),
            };
            commands::handle_config(action_str, &config)?;
        }
        Commands::Doctor => {
            info!("command: doctor");
            commands::handle_doctor(&config)?;
        }
        Commands::Refresh => {
            info!("command: refresh");
            commands::handle_refresh(&config).await?;
        }
        Commands::Rename {
            ref old_name,
            ref new_name,
        } => {
            info!("command: rename ({} -> {})", old_name, new_name);
            commands::handle_rename(old_name, new_name, &config)?;
        }
        Commands::Upgrade => {
            info!("command: upgrade");
            commands::handle_upgrade(skip_confirm).await?;
        }
        Commands::Mcp => {
            info!("command: mcp");
            mcp::run_mcp_server(&config)?;
        }
        Commands::ManPage => {
            info!("command: man");
            let cmd = Cli::command();
            let man = clap_mangen::Man::new(cmd);
            man.render(&mut std::io::stdout())?;
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
