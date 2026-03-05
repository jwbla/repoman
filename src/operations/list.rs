use chrono::{DateTime, Utc};
use colored::Colorize;
use serde::Serialize;
use std::path::PathBuf;

use crate::config::Config;
use crate::error::Result;
use crate::metadata::{CloneEntry, Metadata};
use crate::util;
use crate::vault::Vault;

/// Status of a repository in the system
#[derive(Debug, Serialize)]
pub struct RepoStatus {
    pub name: String,
    pub url: String,
    pub added_date: DateTime<Utc>,
    pub has_pristine: bool,
    pub pristine_path: Option<PathBuf>,
    pub pristine_created: Option<DateTime<Utc>>,
    pub clones: Vec<CloneEntry>,
    pub last_sync: Option<DateTime<Utc>>,
    pub default_branch: Option<String>,
    pub latest_tag: Option<String>,
}

/// List all repositories with their status
pub fn list_all_repos(config: &Config) -> Result<Vec<RepoStatus>> {
    let vault = Vault::load(config)?;
    let mut statuses = Vec::new();

    for entry in &vault.entries {
        let pristine_path = config.pristines_dir.join(&entry.name);
        let has_pristine = pristine_path.exists();

        // Try to load metadata, use defaults if not available
        let metadata = Metadata::load(&entry.name, config).ok();

        let status = RepoStatus {
            name: entry.name.clone(),
            url: entry.url.clone(),
            added_date: entry.added_date,
            has_pristine,
            pristine_path: if has_pristine {
                Some(pristine_path)
            } else {
                None
            },
            pristine_created: metadata.as_ref().and_then(|m| m.pristine_created),
            clones: metadata
                .as_ref()
                .map(|m| m.clones.clone())
                .unwrap_or_default(),
            last_sync: metadata
                .as_ref()
                .and_then(|m| m.last_sync.as_ref())
                .map(|s| s.timestamp),
            default_branch: metadata.as_ref().and_then(|m| m.default_branch.clone()),
            latest_tag: metadata.as_ref().and_then(|m| m.latest_tag.clone()),
        };

        statuses.push(status);
    }

    Ok(statuses)
}

/// Format repo status for display
pub fn format_repo_status(status: &RepoStatus) -> String {
    let mut output = String::new();

    // Header with name and URL
    output.push_str(&format!("  {} \n", status.name.cyan()));
    output.push_str(&format!("    {} {}\n", "URL:".bold(), status.url));
    output.push_str(&format!(
        "    {} {}\n",
        "Added:".bold(),
        status.added_date.format("%Y-%m-%d %H:%M")
    ));

    // Pristine status
    if status.has_pristine {
        output.push_str(&format!(
            "    {} {} initialized\n",
            "Pristine:".bold(),
            "✓".green()
        ));
        if let Some(path) = &status.pristine_path {
            output.push_str(&format!("      {} {}\n", "Path:".bold(), path.display()));
        }
        if let Some(created) = status.pristine_created {
            output.push_str(&format!(
                "      {} {}\n",
                "Created:".bold(),
                created.format("%Y-%m-%d %H:%M")
            ));
        }
    } else {
        output.push_str(&format!(
            "    {} {} not initialized\n",
            "Pristine:".bold(),
            "✗".red()
        ));
    }

    // Last sync
    if let Some(sync_time) = status.last_sync {
        output.push_str(&format!(
            "    {} {} ({})\n",
            "Last sync:".bold(),
            sync_time.format("%Y-%m-%d %H:%M"),
            util::relative_time(&sync_time)
        ));
    }

    // Default branch
    if let Some(branch) = &status.default_branch {
        output.push_str(&format!(
            "    {} {}\n",
            "Default branch:".bold(),
            branch.yellow()
        ));
    }

    // Latest tag
    if let Some(tag) = &status.latest_tag {
        output.push_str(&format!("    {} {}\n", "Latest tag:".bold(), tag));
    }

    // Clones
    if status.clones.is_empty() {
        output.push_str(&format!("    {} none\n", "Clones:".bold()));
    } else {
        output.push_str(&format!(
            "    {} {} total\n",
            "Clones:".bold(),
            status.clones.len()
        ));
        for clone in &status.clones {
            output.push_str(&format!(
                "      - {} ({})\n",
                clone.name,
                clone.path.display()
            ));
            output.push_str(&format!(
                "        {} {}\n",
                "Created:".bold(),
                clone.created.format("%Y-%m-%d %H:%M")
            ));
        }
    }

    output
}

/// Format all repo statuses as a summary table
pub fn format_summary(statuses: &[RepoStatus]) -> String {
    if statuses.is_empty() {
        return "No repositories in vault.\n".to_string();
    }

    let mut output = String::new();

    // Header
    output.push_str(&format!(
        "{:<20} {:<12} {:<8} {:<20}\n",
        "NAME".bold(),
        "PRISTINE".bold(),
        "CLONES".bold(),
        "LAST SYNC".bold()
    ));
    output.push_str(&format!("{}\n", "-".repeat(64)));

    for status in statuses {
        let pristine_status = if status.has_pristine {
            "✓".green().to_string()
        } else {
            "✗".red().to_string()
        };
        let clone_count = status.clones.len().to_string();
        let last_sync = status.last_sync.map_or_else(
            || "never".to_string(),
            |t| {
                format!(
                    "{} ({})",
                    t.format("%Y-%m-%d %H:%M"),
                    util::relative_time(&t)
                )
            },
        );

        output.push_str(&format!(
            "{:<20} {:<12} {:<8} {:<20}\n",
            truncate_string(&status.name, 18).cyan(),
            pristine_status,
            clone_count,
            last_sync
        ));

        // Show clone names and paths indented under the repo
        for clone in &status.clones {
            output.push_str(&format!(
                "  {}-{}  {}\n",
                status.name,
                clone.name,
                clone.path.display()
            ));
        }
    }

    output
}

fn truncate_string(s: &str, max_len: usize) -> String {
    util::truncate(s, max_len)
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
        std::fs::create_dir_all(&config.vault_dir).unwrap();
        std::fs::create_dir_all(&config.pristines_dir).unwrap();
        (temp_dir, config)
    }

    #[test]
    fn test_truncate_string_short() {
        assert_eq!(truncate_string("short", 10), "short");
    }

    #[test]
    fn test_truncate_string_exact() {
        assert_eq!(truncate_string("exactly10c", 10), "exactly10c");
    }

    #[test]
    fn test_truncate_string_long() {
        assert_eq!(truncate_string("this is a long string", 10), "this is...");
    }

    #[test]
    fn test_list_empty_vault() {
        let (_temp, config) = create_test_config();
        let statuses = list_all_repos(&config).unwrap();
        assert!(statuses.is_empty());
    }

    #[test]
    fn test_list_with_repos() {
        let (_temp, config) = create_test_config();

        // Add repos to vault
        let mut vault = Vault::default();
        vault
            .add_entry("repo1".to_string(), "url1".to_string())
            .unwrap();
        vault
            .add_entry("repo2".to_string(), "url2".to_string())
            .unwrap();
        vault.save(&config).unwrap();

        // Create metadata for repo1
        let metadata = Metadata::new(vec!["url1".to_string()]);
        metadata.save("repo1", &config).unwrap();

        // Create pristine directory for repo2
        std::fs::create_dir_all(config.pristines_dir.join("repo2")).unwrap();

        let statuses = list_all_repos(&config).unwrap();
        assert_eq!(statuses.len(), 2);

        let repo1 = statuses.iter().find(|s| s.name == "repo1").unwrap();
        assert!(!repo1.has_pristine);

        let repo2 = statuses.iter().find(|s| s.name == "repo2").unwrap();
        assert!(repo2.has_pristine);
    }

    #[test]
    fn test_format_summary_empty() {
        let summary = format_summary(&[]);
        assert!(summary.contains("No repositories in vault"));
    }

    #[test]
    fn test_format_summary_with_repos() {
        let statuses = vec![RepoStatus {
            name: "repo1".to_string(),
            url: "url1".to_string(),
            added_date: Utc::now(),
            has_pristine: true,
            pristine_path: Some(PathBuf::from("/path")),
            pristine_created: None,
            clones: vec![],
            last_sync: None,
            default_branch: None,
            latest_tag: None,
        }];

        let summary = format_summary(&statuses);
        assert!(summary.contains("NAME"));
        assert!(summary.contains("PRISTINE"));
        assert!(summary.contains("repo1"));
        assert!(summary.contains("✓"));
    }

    #[test]
    fn test_format_repo_status_basic() {
        let status = RepoStatus {
            name: "test-repo".to_string(),
            url: "https://github.com/user/test-repo.git".to_string(),
            added_date: Utc::now(),
            has_pristine: false,
            pristine_path: None,
            pristine_created: None,
            clones: vec![],
            last_sync: None,
            default_branch: None,
            latest_tag: None,
        };

        let output = format_repo_status(&status);
        assert!(output.contains("test-repo"));
        assert!(output.contains("https://github.com/user/test-repo.git"));
        assert!(output.contains("✗ not initialized"));
        assert!(output.contains("Clones: none"));
    }

    #[test]
    fn test_format_repo_status_with_pristine() {
        let status = RepoStatus {
            name: "test-repo".to_string(),
            url: "url".to_string(),
            added_date: Utc::now(),
            has_pristine: true,
            pristine_path: Some(PathBuf::from("/path/to/pristine")),
            pristine_created: Some(Utc::now()),
            clones: vec![],
            last_sync: Some(Utc::now()),
            default_branch: Some("main".to_string()),
            latest_tag: Some("v1.0.0".to_string()),
        };

        let output = format_repo_status(&status);
        assert!(output.contains("✓ initialized"));
        assert!(output.contains("/path/to/pristine"));
        assert!(output.contains("Default branch: main"));
        assert!(output.contains("Latest tag: v1.0.0"));
    }

    #[test]
    fn test_format_repo_status_with_clones() {
        let status = RepoStatus {
            name: "test-repo".to_string(),
            url: "url".to_string(),
            added_date: Utc::now(),
            has_pristine: true,
            pristine_path: None,
            pristine_created: None,
            clones: vec![
                CloneEntry {
                    name: "clone1".to_string(),
                    path: PathBuf::from("/path/clone1"),
                    created: Utc::now(),
                    upstream_conflicts: false,
                },
                CloneEntry {
                    name: "clone2".to_string(),
                    path: PathBuf::from("/path/clone2"),
                    created: Utc::now(),
                    upstream_conflicts: false,
                },
            ],
            last_sync: None,
            default_branch: None,
            latest_tag: None,
        };

        let output = format_repo_status(&status);
        assert!(output.contains("Clones: 2 total"));
        assert!(output.contains("clone1"));
        assert!(output.contains("clone2"));
    }
}
