use chrono::Utc;
use log::{debug, error, info, warn};
use std::fs;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::Duration;

use crate::config::Config;
use crate::error::{RepomanError, Result};
use crate::metadata::Metadata;
use crate::operations;
use crate::vault::Vault;

/// Get the path to the PID file
pub fn pid_file_path(config: &Config) -> PathBuf {
    config.logs_dir.join("agent.pid")
}

/// Get the path to the agent log file
pub fn log_file_path(config: &Config) -> PathBuf {
    config.logs_dir.join("agent.log")
}

/// Check if the agent is currently running
pub fn is_agent_running(config: &Config) -> Option<u32> {
    let pid_path = pid_file_path(config);

    if !pid_path.exists() {
        return None;
    }

    // Read PID from file
    let pid_str = fs::read_to_string(&pid_path).ok()?;
    let pid: u32 = pid_str.trim().parse().ok()?;

    // Check if process is actually running
    #[cfg(unix)]
    {
        // On Unix, we can check if process exists by sending signal 0
        let status = Command::new("kill")
            .args(["-0", &pid.to_string()])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .ok()?;

        if status.success() {
            Some(pid)
        } else {
            // Process doesn't exist, clean up stale PID file
            let _ = fs::remove_file(&pid_path);
            None
        }
    }

    #[cfg(not(unix))]
    {
        // On non-Unix, just check if PID file exists
        // This is a simplification - in production you'd use platform-specific APIs
        Some(pid)
    }
}

/// Start the agent in the background
pub fn start_agent(config: &Config) -> Result<()> {
    info!("start_agent: starting background agent");

    // Check if already running
    if let Some(pid) = is_agent_running(config) {
        warn!("start_agent: agent already running with pid {}", pid);
        return Err(RepomanError::AgentAlreadyRunning(pid));
    }

    // Ensure logs directory exists
    fs::create_dir_all(&config.logs_dir)?;

    // Get current executable path
    let exe_path =
        std::env::current_exe().map_err(|e| RepomanError::AgentSpawnError(e.to_string()))?;

    // Open log file for output
    let log_path = log_file_path(config);
    let log_file =
        fs::File::create(&log_path).map_err(|e| RepomanError::AgentSpawnError(e.to_string()))?;

    // Spawn the agent process
    let child = Command::new(&exe_path)
        .arg("agent")
        .arg("run")
        .stdout(Stdio::from(
            log_file
                .try_clone()
                .map_err(|e| RepomanError::AgentSpawnError(e.to_string()))?,
        ))
        .stderr(Stdio::from(log_file))
        .spawn()
        .map_err(|e| RepomanError::AgentSpawnError(e.to_string()))?;

    // Write PID to file
    let pid_path = pid_file_path(config);
    fs::write(&pid_path, child.id().to_string())
        .map_err(|e| RepomanError::AgentSpawnError(e.to_string()))?;

    Ok(())
}

/// Stop the running agent
pub fn stop_agent(config: &Config) -> Result<()> {
    let pid = is_agent_running(config).ok_or(RepomanError::AgentNotRunning)?;

    // Kill the process
    #[cfg(unix)]
    {
        Command::new("kill")
            .args(["-TERM", &pid.to_string()])
            .status()
            .map_err(|e| RepomanError::AgentSpawnError(e.to_string()))?;
    }

    #[cfg(not(unix))]
    {
        Command::new("taskkill")
            .args(["/PID", &pid.to_string(), "/F"])
            .status()
            .map_err(|e| RepomanError::AgentSpawnError(e.to_string()))?;
    }

    // Remove PID file
    let pid_path = pid_file_path(config);
    let _ = fs::remove_file(&pid_path);

    Ok(())
}

/// Get agent status as a formatted string
pub fn get_agent_status(config: &Config) -> Result<String> {
    match is_agent_running(config) {
        Some(pid) => {
            let log_path = log_file_path(config);
            Ok(format!(
                "Agent is running (PID: {})\nLog file: {}",
                pid,
                log_path.display()
            ))
        }
        None => Ok("Agent is not running".to_string()),
    }
}

/// Run the agent main loop (called when agent starts)
///
/// Uses per-repo `sync_interval` from metadata. Sleeps until the next repo
/// is due, rather than using a single global poll interval.
pub async fn run_agent_loop(config: &Config) -> Result<()> {
    info!("agent loop started");
    println!("Repoman agent started");

    let default_interval: i64 = 3600; // 1 hour fallback

    loop {
        info!("agent: polling cycle starting");

        let mut earliest_due_secs: u64 = default_interval as u64;

        match Vault::load(config) {
            Ok(vault) => {
                let repos = vault.get_all_names();
                debug!("agent: checking {} vaulted repos", repos.len());

                let now = Utc::now();

                for repo_name in repos {
                    let pristine_path = config.pristines_dir.join(repo_name);
                    if !pristine_path.exists() {
                        debug!("agent: skipping '{}' (no pristine)", repo_name);
                        continue;
                    }

                    // Load metadata to check per-repo sync_interval
                    let metadata = match Metadata::load(repo_name, config) {
                        Ok(m) => m,
                        Err(e) => {
                            debug!("agent: failed to load metadata for '{}': {}", repo_name, e);
                            continue;
                        }
                    };

                    let interval = metadata.sync_interval.unwrap_or(default_interval as u64) as i64;

                    // Check if this repo is due for a sync
                    let due = if let Some(ref last_sync) = metadata.last_sync {
                        let elapsed = (now - last_sync.timestamp).num_seconds();
                        elapsed >= interval
                    } else {
                        true // never synced — due immediately
                    };

                    if !due {
                        // Calculate how long until this repo is due
                        if let Some(ref last_sync) = metadata.last_sync {
                            let elapsed = (now - last_sync.timestamp).num_seconds();
                            let remaining = (interval - elapsed).max(1) as u64;
                            earliest_due_secs = earliest_due_secs.min(remaining);
                        }
                        debug!("agent: '{}' not yet due for sync", repo_name);
                        continue;
                    }

                    // Check for new tags + sync
                    match operations::check_for_new_tag(repo_name, config) {
                        Ok(Some(new_tag)) => {
                            info!("agent: new tag for '{}': {}", repo_name, new_tag);
                            println!("New tag found for {}: {}", repo_name, new_tag);

                            if let Err(e) =
                                operations::update_latest_tag(repo_name, &new_tag, config)
                            {
                                error!("agent: failed to update tag for '{}': {}", repo_name, e);
                                println!("Failed to update tag for {}: {}", repo_name, e);
                            }
                        }
                        Ok(None) => {
                            debug!("agent: no new tags for '{}'", repo_name);
                        }
                        Err(e) => {
                            error!("agent: failed to check tags for '{}': {}", repo_name, e);
                            println!("Failed to check tags for {}: {}", repo_name, e);
                        }
                    }

                    info!("agent: auto-syncing '{}'", repo_name);
                    println!("Auto-syncing {}...", repo_name);
                    if let Err(e) = operations::sync_pristine(repo_name, config) {
                        error!("agent: failed to sync '{}': {}", repo_name, e);
                        println!("Failed to sync {}: {}", repo_name, e);
                    }
                }
            }
            Err(e) => {
                error!("agent: failed to load vault: {}", e);
            }
        }

        info!(
            "agent: polling cycle complete, sleeping {}s",
            earliest_due_secs
        );
        tokio::time::sleep(Duration::from_secs(earliest_due_secs)).await;
    }
}
