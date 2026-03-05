//! Lifecycle hook execution. Hooks are shell commands run at defined points (post_clone, post_sync, etc.)
//! with REPOMAN_* env vars and cwd set. Config comes from config.yaml under repos.<name>.hooks.
//! After shell hooks run, Lua plugin callbacks are also invoked for the same event.

use log::{debug, error, warn};
use std::path::Path;
use std::process::Command;
use std::sync::OnceLock;

use crate::config::Config;
use crate::error::{RepomanError, Result};
use crate::plugins::{HookContext, PluginManager};

/// Wrapper to make a raw pointer Send+Sync.
/// Safety: we only access the PluginManager from the main thread.
struct PluginManagerPtr(*const PluginManager);
unsafe impl Send for PluginManagerPtr {}
unsafe impl Sync for PluginManagerPtr {}

/// Global plugin manager reference, set once at startup.
static PLUGIN_MANAGER: OnceLock<PluginManagerPtr> = OnceLock::new();

/// Set the global plugin manager reference. Called once from main.rs at startup.
/// Safety: The PluginManager must outlive all hook calls (guaranteed by main.rs ownership).
pub fn set_plugin_manager(pm: &PluginManager) {
    let _ = PLUGIN_MANAGER.set(PluginManagerPtr(std::ptr::from_ref::<PluginManager>(pm)));
}

/// Run plugin hooks for an event, if a plugin manager is available.
fn run_plugin_hooks(
    event: &str,
    repo_name: &str,
    pristine_path: Option<&Path>,
    clone_path: Option<&Path>,
    clone_name: Option<&str>,
    new_tag: Option<&str>,
) {
    if let Some(wrapper) = PLUGIN_MANAGER.get() {
        // Safety: the PluginManager is alive for the duration of the program,
        // and we only access it from the main thread.
        let pm = unsafe { &*wrapper.0 };
        let ctx = HookContext {
            repo: repo_name.to_string(),
            event: event.to_string(),
            pristine_path: pristine_path.map(|p| p.to_string_lossy().into_owned()),
            clone_path: clone_path.map(|p| p.to_string_lossy().into_owned()),
            clone_name: clone_name.map(String::from),
            new_tag: new_tag.map(String::from),
        };
        if let Err(e) = pm.run_hook(event, &ctx) {
            warn!("plugin hook '{}' error: {}", event, e);
        }
    }
}

/// Run a single hook command. Uses `sh -c "<command>"` so shell syntax works.
/// Sets REPOMAN_REPO, REPOMAN_EVENT, and optionally pristine/clone paths and REPOMAN_NEW_TAG.
/// If `fail_on_error` is true, non-zero exit returns HookFailed; otherwise we log and return Ok(())
#[allow(clippy::too_many_arguments)]
pub fn run_hook(
    command: &str,
    event: &str,
    repo_name: &str,
    cwd: &Path,
    pristine_path: Option<&Path>,
    clone_path: Option<&Path>,
    clone_name: Option<&str>,
    new_tag: Option<&str>,
    fail_on_error: bool,
) -> Result<()> {
    let mut env: Vec<(String, String)> = vec![
        ("REPOMAN_REPO".to_string(), repo_name.to_string()),
        ("REPOMAN_EVENT".to_string(), event.to_string()),
    ];
    if let Some(p) = pristine_path {
        env.push((
            "REPOMAN_PRISTINE_PATH".to_string(),
            p.to_string_lossy().into_owned(),
        ));
    }
    if let Some(p) = clone_path {
        env.push((
            "REPOMAN_CLONE_PATH".to_string(),
            p.to_string_lossy().into_owned(),
        ));
    }
    if let Some(n) = clone_name {
        env.push(("REPOMAN_CLONE_NAME".to_string(), n.to_string()));
    }
    if let Some(t) = new_tag {
        env.push(("REPOMAN_NEW_TAG".to_string(), t.to_string()));
    }

    debug!(
        "hooks: running {} for '{}': sh -c \"{}\"",
        event, repo_name, command
    );

    let status = Command::new("sh")
        .arg("-c")
        .arg(command)
        .current_dir(cwd)
        .envs(env.iter().map(|(k, v)| (k.as_str(), v.as_str())))
        .status()
        .map_err(|e| RepomanError::HookFailed(event.to_string(), e.to_string()))?;

    if !status.success() {
        let msg = status.code().map_or_else(
            || "process terminated by signal".to_string(),
            |c| format!("exit code {}", c),
        );
        if fail_on_error {
            error!("hooks: {} failed for '{}': {}", event, repo_name, msg);
            return Err(RepomanError::HookFailed(event.to_string(), msg));
        }
        warn!(
            "hooks: {} failed for '{}' (non-fatal): {}",
            event, repo_name, msg
        );
    }
    Ok(())
}

/// Run post_init_pristine hook if configured. Call after pristine is created and metadata saved.
/// fail_on_error: true so init is considered failed if hook fails.
pub fn run_post_init_pristine(
    config: &Config,
    repo_name: &str,
    pristine_path: &Path,
) -> Result<()> {
    let command = config
        .hooks_for_repo(repo_name)
        .and_then(|h| h.post_init_pristine.as_deref());

    if let Some(cmd) = command {
        run_hook(
            cmd,
            "post_init_pristine",
            repo_name,
            pristine_path,
            Some(pristine_path),
            None,
            None,
            None,
            true,
        )?;
    }

    run_plugin_hooks(
        "post_init_pristine",
        repo_name,
        Some(pristine_path),
        None,
        None,
        None,
    );
    Ok(())
}

/// Run pre_clone hook if configured. Call before creating the clone. cwd = pristine path.
pub fn run_pre_clone(config: &Config, repo_name: &str, pristine_path: &Path) -> Result<()> {
    let command = config
        .hooks_for_repo(repo_name)
        .and_then(|h| h.pre_clone.as_deref());

    if let Some(cmd) = command {
        run_hook(
            cmd,
            "pre_clone",
            repo_name,
            pristine_path,
            Some(pristine_path),
            None,
            None,
            None,
            true,
        )?;
    }

    run_plugin_hooks(
        "pre_clone",
        repo_name,
        Some(pristine_path),
        None,
        None,
        None,
    );
    Ok(())
}

/// Run post_clone hook if configured. Call after clone is created and metadata saved. cwd = clone path.
pub fn run_post_clone(
    config: &Config,
    repo_name: &str,
    clone_path: &Path,
    clone_name: &str,
    pristine_path: &Path,
) -> Result<()> {
    let command = config
        .hooks_for_repo(repo_name)
        .and_then(|h| h.post_clone.as_deref());

    if let Some(cmd) = command {
        run_hook(
            cmd,
            "post_clone",
            repo_name,
            clone_path,
            Some(pristine_path),
            Some(clone_path),
            Some(clone_name),
            None,
            true,
        )?;
    }

    run_plugin_hooks(
        "post_clone",
        repo_name,
        Some(pristine_path),
        Some(clone_path),
        Some(clone_name),
        None,
    );
    Ok(())
}

/// Run post_sync hook if configured. Call after sync. fail_on_error: false so sync is not blocked.
pub fn run_post_sync(config: &Config, repo_name: &str, pristine_path: &Path) -> Result<()> {
    let command = config
        .hooks_for_repo(repo_name)
        .and_then(|h| h.post_sync.as_deref());

    if let Some(cmd) = command {
        run_hook(
            cmd,
            "post_sync",
            repo_name,
            pristine_path,
            Some(pristine_path),
            None,
            None,
            None,
            false,
        )?;
    }

    run_plugin_hooks(
        "post_sync",
        repo_name,
        Some(pristine_path),
        None,
        None,
        None,
    );
    Ok(())
}

/// Run post_sync_on_new_tag hook if configured. Call after sync when agent detected a new tag.
pub fn run_post_sync_on_new_tag(
    config: &Config,
    repo_name: &str,
    pristine_path: &Path,
    new_tag: &str,
) -> Result<()> {
    let command = config
        .hooks_for_repo(repo_name)
        .and_then(|h| h.post_sync_on_new_tag.as_deref());

    if let Some(cmd) = command {
        run_hook(
            cmd,
            "post_sync_on_new_tag",
            repo_name,
            pristine_path,
            Some(pristine_path),
            None,
            None,
            Some(new_tag),
            false,
        )?;
    }

    run_plugin_hooks(
        "post_sync_on_new_tag",
        repo_name,
        Some(pristine_path),
        None,
        None,
        Some(new_tag),
    );
    Ok(())
}

/// Run pre_destroy hook if configured. Call before removing clone. fail_on_error: false.
pub fn run_pre_destroy(
    config: &Config,
    repo_name: &str,
    clone_path: &Path,
    clone_name: &str,
    pristine_path: &Path,
) -> Result<()> {
    let command = config
        .hooks_for_repo(repo_name)
        .and_then(|h| h.pre_destroy.as_deref());

    if let Some(cmd) = command {
        run_hook(
            cmd,
            "pre_destroy",
            repo_name,
            clone_path,
            Some(pristine_path),
            Some(clone_path),
            Some(clone_name),
            None,
            false,
        )?;
    }

    run_plugin_hooks(
        "pre_destroy",
        repo_name,
        Some(pristine_path),
        Some(clone_path),
        Some(clone_name),
        None,
    );
    Ok(())
}

/// Run post_destroy hook if configured. Call after clone removed. cwd = clones_dir.
pub fn run_post_destroy(config: &Config, repo_name: &str, clones_dir: &Path) -> Result<()> {
    let command = config
        .hooks_for_repo(repo_name)
        .and_then(|h| h.post_destroy.as_deref());

    if let Some(cmd) = command {
        run_hook(
            cmd,
            "post_destroy",
            repo_name,
            clones_dir,
            None,
            None,
            None,
            None,
            false,
        )?;
    }

    run_plugin_hooks("post_destroy", repo_name, None, None, None, None);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_run_hook_noop_succeeds() {
        let temp = tempfile::tempdir().unwrap();
        let result = run_hook(
            "true",
            "post_clone",
            "my-repo",
            temp.path(),
            Some(temp.path().as_ref()),
            None,
            None,
            None,
            true,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_run_hook_env_and_cwd() {
        let temp = tempfile::tempdir().unwrap();
        // Write REPOMAN_EVENT and REPOMAN_REPO into marker file to prove env and cwd
        let cmd = format!("echo \"$REPOMAN_EVENT\" > marker && echo \"$REPOMAN_REPO\" >> marker");
        let result = run_hook(
            &cmd,
            "post_sync",
            "test-repo",
            temp.path(),
            Some(temp.path().as_ref()),
            None,
            None,
            None,
            true,
        );
        assert!(result.is_ok());
        let content = fs::read_to_string(temp.path().join("marker")).unwrap();
        assert!(content.contains("post_sync"));
        assert!(content.contains("test-repo"));
    }

    #[test]
    fn test_run_hook_fail_on_error_returns_err() {
        let temp = tempfile::tempdir().unwrap();
        let result = run_hook(
            "false",
            "post_clone",
            "x",
            temp.path(),
            None,
            None,
            None,
            None,
            true,
        );
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("post_clone"));
    }

    #[test]
    fn test_run_hook_fail_non_fatal_returns_ok() {
        let temp = tempfile::tempdir().unwrap();
        let result = run_hook(
            "false",
            "post_sync",
            "x",
            temp.path(),
            None,
            None,
            None,
            None,
            false,
        );
        assert!(result.is_ok());
    }
}
