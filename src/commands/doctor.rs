use colored::Colorize;
use git2::Repository;
use std::path::PathBuf;

use crate::config::Config;
use crate::error::Result;
use crate::metadata::Metadata;
use crate::vault::Vault;

pub fn handle_doctor(config: &Config) -> Result<()> {
    println!("{}", "Repoman Health Check".bold());
    println!();

    let mut issues = 0;

    // Check config
    let config_path = dirs::config_dir().map(|p| p.join("repoman").join("config.yaml"));
    match config_path {
        Some(ref p) if p.exists() => println!("  {} Config file: {}", "OK".green(), p.display()),
        _ => println!("  {} No config file (using defaults)", "INFO".blue()),
    }

    // Check directories
    for (name, path) in [
        ("vault_dir", &config.vault_dir),
        ("pristines_dir", &config.pristines_dir),
        ("clones_dir", &config.clones_dir),
        ("plugins_dir", &config.plugins_dir),
        ("logs_dir", &config.logs_dir),
    ] {
        if path.exists() {
            println!("  {} {}: {}", "OK".green(), name, path.display());
        } else {
            println!("  {} {} missing: {}", "WARN".yellow(), name, path.display());
            issues += 1;
        }
    }

    // Check vault integrity
    let vault = Vault::load(config)?;
    println!(
        "\n  {} {} repositories in vault",
        "OK".green(),
        vault.entries.len()
    );

    let mut orphan_metadata = 0;
    let mut broken_pristines = 0;
    let mut broken_alternates = 0;
    let mut missing_clones = 0;
    let mut total_clones = 0;

    for entry in &vault.entries {
        // Check metadata exists
        let metadata = if let Ok(m) = Metadata::load(&entry.name, config) {
            m
        } else {
            println!("  {} No metadata for '{}'", "WARN".yellow(), entry.name);
            orphan_metadata += 1;
            continue;
        };

        // Check pristine
        let pristine_path = config.pristines_dir.join(&entry.name);
        if pristine_path.exists() && Repository::open_bare(&pristine_path).is_err() {
            println!(
                "  {} Pristine '{}' is not a valid bare repo",
                "ERROR".red(),
                entry.name
            );
            broken_pristines += 1;
        }

        // Check clones
        for clone in &metadata.clones {
            total_clones += 1;
            if !clone.path.exists() {
                println!(
                    "  {} Clone '{}' path missing: {}",
                    "WARN".yellow(),
                    clone.name,
                    clone.path.display()
                );
                missing_clones += 1;
                continue;
            }

            // Check alternates
            let alt_file = clone
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
                        println!(
                            "  {} Clone '{}' has broken alternates: {}",
                            "ERROR".red(),
                            clone.name,
                            alt_path.display()
                        );
                        broken_alternates += 1;
                    }
                }
            }
        }
    }

    // Check SSH
    let ssh_agent = std::env::var("SSH_AUTH_SOCK").is_ok();
    if ssh_agent {
        println!("  {} SSH agent available", "OK".green());
    } else {
        println!("  {} SSH_AUTH_SOCK not set", "WARN".yellow());
        issues += 1;
    }

    // Check agent
    if let Some(pid) = crate::agent::is_agent_running(config) {
        println!("  {} Agent running (PID {})", "OK".green(), pid);
    } else {
        println!("  {} Agent not running", "INFO".blue());
    }

    // Summary
    let total_issues =
        issues + orphan_metadata + broken_pristines + broken_alternates + missing_clones;
    println!();
    println!(
        "  {} repos, {} clones, {} issues",
        vault.entries.len(),
        total_clones,
        total_issues
    );

    if total_issues == 0 {
        println!("  {}", "Everything looks good!".green().bold());
    } else {
        println!(
            "  {}",
            format!("{} issue(s) found", total_issues).yellow().bold()
        );
    }

    Ok(())
}
