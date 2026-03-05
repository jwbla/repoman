use chrono::Utc;
use log::{debug, error, info, warn};
use std::fs;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::Duration;

use crate::config::Config;
use crate::error::{RepomanError, Result};
use crate::hooks;
use crate::metadata::Metadata;
use crate::operations;
use crate::util;
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

    #[cfg(windows)]
    {
        // Check if the process is still running via tasklist
        let status = Command::new("tasklist")
            .args(["/FI", &format!("PID eq {}", pid), "/NH"])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .output()
            .ok();

        match status {
            Some(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                if stdout.contains(&pid.to_string()) {
                    Some(pid)
                } else {
                    let _ = fs::remove_file(&pid_path);
                    None
                }
            }
            None => {
                let _ = fs::remove_file(&pid_path);
                None
            }
        }
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
pub fn get_agent_status(config: &Config) -> String {
    match is_agent_running(config) {
        Some(pid) => {
            let log_path = log_file_path(config);
            format!(
                "Agent is running (PID: {})\nLog file: {}",
                pid,
                log_path.display()
            )
        }
        None => "Agent is not running".to_string(),
    }
}

/// Result of a single agent iteration.
pub struct IterationResult {
    /// How many seconds to sleep before the next iteration.
    pub sleep_secs: u64,
}

/// Run one iteration of the agent loop: check repos, sync due ones, heartbeat.
/// Returns how long to sleep before the next iteration.
pub async fn run_agent_iteration(
    config: &Config,
    heartbeat_secs: u64,
    last_heartbeat: &mut std::time::Instant,
) -> IterationResult {
    let default_interval: i64 = 3600;
    let mut earliest_due_secs: u64 = default_interval as u64;

    match Vault::load(config) {
        Ok(vault) => {
            let repos = vault.get_all_names();
            debug!("agent: checking {} vaulted repos", repos.len());

            let now = Utc::now();

            // Phase 1: collect which repos are due for sync (fast, sequential)
            let mut due_repos: Vec<String> = Vec::new();

            for repo_name in repos {
                let pristine_path = config.pristines_dir.join(repo_name);
                if !pristine_path.exists() {
                    debug!("agent: skipping '{}' (no pristine)", repo_name);
                    continue;
                }

                let metadata = match Metadata::load(repo_name, config) {
                    Ok(m) => m,
                    Err(e) => {
                        debug!("agent: failed to load metadata for '{}': {}", repo_name, e);
                        continue;
                    }
                };

                let interval = config.effective_sync_interval(repo_name, &metadata) as i64;

                let due = if let Some(ref last_sync) = metadata.last_sync {
                    let elapsed = (now - last_sync.timestamp).num_seconds();
                    elapsed >= interval
                } else {
                    true // never synced — due immediately
                };

                if !due {
                    if let Some(ref last_sync) = metadata.last_sync {
                        let elapsed = (now - last_sync.timestamp).num_seconds();
                        let remaining = (interval - elapsed).max(1) as u64;
                        earliest_due_secs = earliest_due_secs.min(remaining);
                    }
                    debug!("agent: '{}' not yet due for sync", repo_name);
                    continue;
                }

                due_repos.push(repo_name.to_string());
            }

            // Phase 2: tag-check + sync in parallel
            if !due_repos.is_empty() {
                let config_clone = config.clone();
                let max = config.max_parallel();
                let results = util::run_parallel(due_repos, max, move |name| {
                    // Check for new tags
                    let new_tag = match operations::check_for_new_tag(name, &config_clone) {
                        Ok(Some(ref new_tag)) => {
                            info!("agent: new tag for '{}': {}", name, new_tag);
                            if let Err(e) =
                                operations::update_latest_tag(name, new_tag, &config_clone)
                            {
                                error!("agent: failed to update tag for '{}': {}", name, e);
                            }
                            Some(new_tag.clone())
                        }
                        Ok(None) => None,
                        Err(e) => {
                            error!("agent: failed to check tags for '{}': {}", name, e);
                            None
                        }
                    };

                    // Sync
                    let sync_result = operations::sync_pristine(name, &config_clone);
                    (sync_result, new_tag)
                })
                .await;

                // Phase 3: run hooks sequentially (plugin manager is single-threaded)
                for (name, result) in results {
                    match result {
                        Ok((sync_result, new_tag)) => {
                            if let Err(e) = sync_result {
                                error!("agent: failed to sync '{}': {}", name, e);
                                println!("Failed to sync {}: {}", name, e);
                            } else if let Some(ref tag) = new_tag {
                                println!("New tag found for {}: {}", name, tag);
                                let pristine_path = config.pristines_dir.join(&name);
                                let _ = hooks::run_post_sync_on_new_tag(
                                    config,
                                    &name,
                                    &pristine_path,
                                    tag,
                                );
                            }
                        }
                        Err(e) => {
                            error!("agent: task error for '{}': {}", name, e);
                        }
                    }
                }
            }
        }
        Err(e) => {
            error!("agent: failed to load vault: {}", e);
        }
    }

    // Heartbeat: update clones from pristine state (parallel)
    if last_heartbeat.elapsed() >= Duration::from_secs(heartbeat_secs) {
        debug!("agent: heartbeat — checking clones for upstream updates");
        if let Ok(vault) = Vault::load(config) {
            let repo_names: Vec<String> = vault
                .get_all_names()
                .into_iter()
                .map(String::from)
                .collect();
            let config_clone = config.clone();
            let max = config.max_parallel();
            let results = util::run_parallel(repo_names, max, move |name| {
                operations::heartbeat_update_clones(name, &config_clone)
            })
            .await;
            for (name, result) in results {
                match result {
                    Ok(Err(e)) => {
                        debug!("agent: heartbeat failed for '{}': {}", name, e);
                    }
                    Err(e) => {
                        debug!("agent: heartbeat task error for '{}': {}", name, e);
                    }
                    _ => {}
                }
            }
        }
        *last_heartbeat = std::time::Instant::now();
    }

    // Sleep for min(earliest_due_secs, time_until_next_heartbeat)
    let heartbeat_remaining = heartbeat_secs.saturating_sub(last_heartbeat.elapsed().as_secs());
    let sleep_secs = earliest_due_secs.min(heartbeat_remaining.max(1));

    IterationResult { sleep_secs }
}

/// Run the agent main loop (called when agent starts)
///
/// Uses per-repo `sync_interval` from metadata. Sleeps until the next repo
/// is due, rather than using a single global poll interval.
pub async fn run_agent_loop(config: &Config) -> Result<()> {
    info!("agent loop started");
    println!("Repoman agent started");

    let heartbeat_secs = config.agent_heartbeat_interval.unwrap_or(300);
    let mut last_heartbeat = std::time::Instant::now();

    loop {
        info!("agent: polling cycle starting");

        let result = run_agent_iteration(config, heartbeat_secs, &mut last_heartbeat).await;

        info!(
            "agent: polling cycle complete, sleeping {}s",
            result.sleep_secs
        );
        tokio::time::sleep(Duration::from_secs(result.sleep_secs)).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_config() -> (TempDir, Config) {
        let temp_dir = TempDir::new().unwrap();
        let base = temp_dir.path().to_path_buf();
        let config = Config {
            vault_dir: base.join("vault"),
            pristines_dir: base.join("pristines"),
            clones_dir: base.join("clones"),
            plugins_dir: base.join("plugins"),
            logs_dir: base.join("logs"),
            agent_heartbeat_interval: None,
            json_output: None,
            max_parallel: None,
            repos: None,
        };
        fs::create_dir_all(&config.vault_dir).unwrap();
        fs::create_dir_all(&config.pristines_dir).unwrap();
        fs::create_dir_all(&config.clones_dir).unwrap();
        fs::create_dir_all(&config.logs_dir).unwrap();
        (temp_dir, config)
    }

    #[tokio::test]
    async fn test_agent_iteration_empty_vault() {
        let (_temp, config) = create_test_config();

        // Empty vault
        let vault = Vault::default();
        vault.save(&config).unwrap();

        let mut last_heartbeat = std::time::Instant::now();
        let result = run_agent_iteration(&config, 300, &mut last_heartbeat).await;

        // Should return a sleep duration without error
        assert!(result.sleep_secs > 0);
    }

    #[tokio::test]
    async fn test_agent_iteration_skips_not_due() {
        let (_temp, config) = create_test_config();

        // Create vault with a repo
        let mut vault = Vault::default();
        vault
            .add_entry("test-repo".to_string(), "file:///nonexistent".to_string())
            .unwrap();
        vault.save(&config).unwrap();

        // Create metadata with recent sync
        let mut metadata = Metadata::new(vec!["file:///nonexistent".to_string()]);
        metadata.mark_synced("auto");
        metadata.save("test-repo", &config).unwrap();

        // Create pristine dir (so repo isn't skipped for missing pristine)
        fs::create_dir_all(config.pristines_dir.join("test-repo")).unwrap();

        let mut last_heartbeat = std::time::Instant::now();
        let result = run_agent_iteration(&config, 300, &mut last_heartbeat).await;

        // Should return a sleep duration (repo not due since we just synced)
        assert!(result.sleep_secs > 0);

        // Metadata should be unchanged (no new sync)
        let loaded = Metadata::load("test-repo", &config).unwrap();
        assert_eq!(loaded.last_sync.as_ref().unwrap().sync_type, "auto");
    }

    #[test]
    fn test_agent_pid_file_path() {
        let (_temp, config) = create_test_config();
        let pid_path = pid_file_path(&config);
        assert!(pid_path.ends_with("agent.pid"));
    }

    #[test]
    fn test_agent_log_file_path() {
        let (_temp, config) = create_test_config();
        let log_path = log_file_path(&config);
        assert!(log_path.ends_with("agent.log"));
    }

    #[test]
    fn test_is_agent_running_no_pid_file() {
        let (_temp, config) = create_test_config();
        assert!(is_agent_running(&config).is_none());
    }

    #[test]
    fn test_is_agent_running_stale_pid() {
        let (_temp, config) = create_test_config();
        // Write a PID that definitely doesn't exist
        let pid_path = pid_file_path(&config);
        fs::write(&pid_path, "999999999").unwrap();
        // Should return None and clean up the stale file
        assert!(is_agent_running(&config).is_none());
    }
}
