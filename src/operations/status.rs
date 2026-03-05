use colored::Colorize;
use git2::Repository;
use log::{debug, warn};
use serde::Serialize;
use std::fmt;
use std::path::PathBuf;

use crate::config::Config;
use crate::error::Result;
use crate::metadata::Metadata;
use crate::util;
use crate::vault::Vault;

#[derive(Serialize)]
pub struct CloneStatus {
    pub name: String,
    pub path: PathBuf,
    pub branch: Option<String>,
    pub dirty_files: usize,
    pub ahead: usize,
    pub behind: usize,
    pub upstream_conflicts: bool,
}

#[derive(Serialize)]
pub struct DetailedStatus {
    pub name: String,
    pub url: String,
    pub pristine_exists: bool,
    pub pristine_branches: Vec<String>,
    pub clones: Vec<CloneStatus>,
    pub latest_tag: Option<String>,
    pub last_sync: Option<String>,
    pub last_sync_relative: Option<String>,
    pub sync_interval: Option<u64>,
    pub alternates_ok: bool,
}

impl fmt::Display for DetailedStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "{}: {}", "Repository".bold(), self.name.cyan())?;
        writeln!(f, "  {}: {}", "URL".bold(), self.url)?;
        writeln!(
            f,
            "  {}: {}",
            "Pristine".bold(),
            if self.pristine_exists {
                "yes".green().to_string()
            } else {
                "no".red().to_string()
            }
        )?;

        if !self.pristine_branches.is_empty() {
            writeln!(
                f,
                "  {}: {}",
                "Branches".bold(),
                self.pristine_branches.join(", ")
            )?;
        }

        if let Some(ref tag) = self.latest_tag {
            writeln!(f, "  {}: {}", "Latest tag".bold(), tag.yellow())?;
        }
        if let Some(ref sync) = self.last_sync {
            let relative = self.last_sync_relative.as_deref().unwrap_or("");
            if relative.is_empty() {
                writeln!(f, "  {}: {}", "Last sync".bold(), sync)?;
            } else {
                writeln!(f, "  {}: {} ({})", "Last sync".bold(), sync, relative)?;
            }
        }
        if let Some(interval) = self.sync_interval {
            writeln!(f, "  {}: {}s", "Sync interval".bold(), interval)?;
        }

        if self.clones.is_empty() {
            writeln!(f, "  {}: none", "Clones".bold())?;
        } else {
            writeln!(f, "  {} ({}):", "Clones".bold(), self.clones.len())?;
            for c in &self.clones {
                let branch = c.branch.as_deref().unwrap_or("detached");
                let dirty = if c.dirty_files > 0 {
                    format!(" ({} dirty)", c.dirty_files).yellow().to_string()
                } else {
                    String::new()
                };
                let ahead_behind = if c.ahead > 0 || c.behind > 0 {
                    format!(" [+{}/-{}]", c.ahead, c.behind)
                } else {
                    String::new()
                };
                let conflict_warning = if c.upstream_conflicts {
                    format!(" {}", "UPSTREAM CONFLICTS".red().bold())
                } else {
                    String::new()
                };
                writeln!(
                    f,
                    "    {} on {}{}{}{}",
                    c.name, branch, dirty, ahead_behind, conflict_warning
                )?;
            }
        }

        if !self.alternates_ok {
            writeln!(
                f,
                "  {}",
                "WARNING: alternates health check failed".yellow()
            )?;
        }

        Ok(())
    }
}

/// Get detailed status for a single repository
pub fn get_detailed_status(name: &str, config: &Config) -> Result<DetailedStatus> {
    let vault = Vault::load(config)?;
    let resolved = vault.resolve_name(name);
    let entry = vault
        .get_entry(resolved)
        .ok_or_else(|| crate::error::RepomanError::RepoNotInVault(resolved.to_string()))?;

    let metadata = Metadata::load(resolved, config)?;
    let pristine_path = config.pristines_dir.join(resolved);
    let pristine_exists = pristine_path.exists();

    // Get pristine branches
    let mut pristine_branches = Vec::new();
    if pristine_exists
        && let Ok(repo) = Repository::open_bare(&pristine_path)
        && let Ok(branches) = repo.branches(Some(git2::BranchType::Local))
    {
        for branch in branches.flatten() {
            if let Some(name) = branch.0.name().ok().flatten() {
                pristine_branches.push(name.to_string());
            }
        }
    }

    // Get clone statuses
    let mut clones = Vec::new();
    for clone_entry in &metadata.clones {
        let mut cs = CloneStatus {
            name: clone_entry.name.clone(),
            path: clone_entry.path.clone(),
            branch: None,
            dirty_files: 0,
            ahead: 0,
            behind: 0,
            upstream_conflicts: clone_entry.upstream_conflicts,
        };

        if clone_entry.path.exists()
            && let Ok(repo) = Repository::open(&clone_entry.path)
        {
            // Current branch
            if let Ok(head) = repo.head() {
                cs.branch = head.shorthand().map(String::from);

                // Ahead/behind: check pristine remote first, fallback to origin
                let ref_prefix = if repo.find_remote("pristine").is_ok() {
                    "refs/remotes/pristine"
                } else {
                    "refs/remotes/origin"
                };

                if let (Ok(local_oid), Ok(remote_ref)) = (
                    head.target().ok_or(()),
                    repo.find_reference(&format!(
                        "{}/{}",
                        ref_prefix,
                        cs.branch.as_deref().unwrap_or("")
                    ))
                    .map_err(|_| ()),
                ) && let Some(remote_oid) = remote_ref.target()
                    && let Ok((ahead, behind)) = repo.graph_ahead_behind(local_oid, remote_oid)
                {
                    cs.ahead = ahead;
                    cs.behind = behind;
                }
            }

            // Dirty state
            if let Ok(statuses) = repo.statuses(None) {
                cs.dirty_files = statuses.len();
            }
        }

        clones.push(cs);
    }

    // Check alternates health
    let mut alternates_ok = true;
    for clone_entry in &metadata.clones {
        let alt_file = clone_entry
            .path
            .join(".git")
            .join("objects")
            .join("info")
            .join("alternates");
        if alt_file.exists()
            && let Ok(content) = std::fs::read_to_string(&alt_file)
        {
            for line in content.lines() {
                let alt_path = PathBuf::from(line.trim());
                if !alt_path.exists() {
                    warn!(
                        "alternates path missing for clone '{}': {}",
                        clone_entry.name,
                        alt_path.display()
                    );
                    alternates_ok = false;
                }
            }
        }
    }

    let last_sync = metadata.last_sync.as_ref().map(|s| {
        format!(
            "{} ({})",
            s.timestamp.format("%Y-%m-%d %H:%M:%S UTC"),
            s.sync_type
        )
    });

    let last_sync_relative = metadata
        .last_sync
        .as_ref()
        .map(|s| util::relative_time(&s.timestamp));

    debug!("get_detailed_status: done for '{}'", resolved);

    Ok(DetailedStatus {
        name: resolved.to_string(),
        url: entry.url.clone(),
        pristine_exists,
        pristine_branches,
        clones,
        latest_tag: metadata.latest_tag.clone(),
        last_sync,
        last_sync_relative,
        sync_interval: metadata.sync_interval,
        alternates_ok,
    })
}
