use chrono::Utc;
use git2::Repository;
use log::{debug, info, warn};
use std::path::PathBuf;
use std::process::Command;

use crate::config::Config;
use crate::error::Result;
use crate::metadata::Metadata;
use crate::vault::Vault;

pub struct StaleClone {
    pub repo_name: String,
    pub clone_name: String,
    pub path: PathBuf,
    pub days_old: i64,
}

pub struct GcReport {
    pub stale_clones: Vec<StaleClone>,
    pub pristines_gc_run: usize,
}

/// Find clones whose HEAD commit is older than `days` days.
pub fn find_stale_clones(days: u64, config: &Config) -> Result<Vec<StaleClone>> {
    let vault = Vault::load(config)?;
    let cutoff = Utc::now() - chrono::Duration::days(days as i64);
    let mut stale = Vec::new();

    for repo_name in vault.get_all_names() {
        let metadata = match Metadata::load(repo_name, config) {
            Ok(m) => m,
            Err(_) => continue,
        };

        for clone_entry in &metadata.clones {
            if !clone_entry.path.exists() {
                continue;
            }

            let repo = match Repository::open(&clone_entry.path) {
                Ok(r) => r,
                Err(_) => continue,
            };

            let head_time = match repo.head()
                .ok()
                .and_then(|h| h.peel_to_commit().ok())
                .map(|c| c.time())
            {
                Some(t) => t,
                None => continue,
            };

            let commit_ts = chrono::DateTime::from_timestamp(head_time.seconds(), 0);
            if let Some(ts) = commit_ts
                && ts < cutoff
            {
                let days_old = (Utc::now() - ts).num_days();
                stale.push(StaleClone {
                    repo_name: repo_name.to_string(),
                    clone_name: clone_entry.name.clone(),
                    path: clone_entry.path.clone(),
                    days_old,
                });
            }
        }
    }

    Ok(stale)
}

/// Run `git gc --auto` on each pristine bare repo.
fn gc_pristines(config: &Config, dry_run: bool) -> Result<usize> {
    let vault = Vault::load(config)?;
    let mut count = 0;

    for repo_name in vault.get_all_names() {
        let pristine_path = config.pristines_dir.join(repo_name);
        if !pristine_path.exists() {
            continue;
        }

        if dry_run {
            debug!("gc: would run git gc --auto in {}", pristine_path.display());
            count += 1;
            continue;
        }

        info!("gc: running git gc --auto in {}", pristine_path.display());
        match Command::new("git")
            .args(["gc", "--auto"])
            .current_dir(&pristine_path)
            .output()
        {
            Ok(output) => {
                if !output.status.success() {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    warn!("gc: git gc failed for '{}': {}", repo_name, stderr.trim());
                } else {
                    count += 1;
                }
            }
            Err(e) => {
                warn!("gc: failed to run git gc for '{}': {}", repo_name, e);
            }
        }
    }

    Ok(count)
}

/// Run the full GC cycle: find stale clones + gc pristines.
pub fn run_gc(days: u64, dry_run: bool, config: &Config) -> Result<GcReport> {
    info!("run_gc: days={}, dry_run={}", days, dry_run);

    let stale_clones = find_stale_clones(days, config)?;
    let pristines_gc_run = gc_pristines(config, dry_run)?;

    if !dry_run {
        // Actually remove stale clones
        for sc in &stale_clones {
            if sc.path.exists() {
                info!("gc: removing stale clone '{}' at {}", sc.clone_name, sc.path.display());
                let _ = std::fs::remove_dir_all(&sc.path);

                // Update metadata
                if let Ok(mut metadata) = Metadata::load(&sc.repo_name, config) {
                    metadata.remove_clone(&sc.clone_name);
                    let _ = metadata.save(&sc.repo_name, config);
                }
            }
        }
    }

    Ok(GcReport {
        stale_clones,
        pristines_gc_run,
    })
}
