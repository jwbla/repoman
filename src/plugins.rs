//! Lua plugin system. Plugins are .lua files in the plugins directory that register
//! callbacks for lifecycle events via `repoman.on(event, function)`.

use log::{debug, error, info, warn};
use mlua::prelude::*;
use std::path::Path;

use crate::error::{RepomanError, Result};
use crate::vault::Vault;

/// Context passed to plugin hook callbacks.
#[derive(Debug, Clone)]
pub struct HookContext {
    pub repo: String,
    pub event: String,
    pub pristine_path: Option<String>,
    pub clone_path: Option<String>,
    pub clone_name: Option<String>,
    pub new_tag: Option<String>,
}

/// Manages Lua plugin lifecycle.
pub struct PluginManager {
    lua: Lua,
    loaded: Vec<String>,
}

impl PluginManager {
    pub fn new(vault: Option<&Vault>) -> Result<Self> {
        let lua = Lua::new();

        // Set up the repoman API table
        {
            let globals = lua.globals();

            let repoman = lua.create_table().map_err(lua_err)?;

            // repoman.on(event, callback) — store callbacks in a registry table
            let hooks_table = lua.create_table().map_err(lua_err)?;
            lua.set_named_registry_value("repoman_hooks", hooks_table)
                .map_err(lua_err)?;

            let lua_ref = lua.clone();
            let on_fn = lua
                .create_function(move |_, (event, callback): (String, LuaFunction)| {
                    let hooks: LuaTable = lua_ref.named_registry_value("repoman_hooks").unwrap();
                    let existing: LuaTable = if let Ok(t) = hooks.get::<LuaTable>(event.as_str()) {
                        t
                    } else {
                        let t = lua_ref.create_table().unwrap();
                        hooks.set(event.as_str(), t.clone()).unwrap();
                        t
                    };
                    let len = existing.len().unwrap_or(0);
                    existing.set(len + 1, callback).unwrap();
                    Ok(())
                })
                .map_err(lua_err)?;
            repoman.set("on", on_fn).map_err(lua_err)?;

            // repoman.log(level, message)
            let log_fn = lua
                .create_function(|_, (level, message): (String, String)| {
                    match level.as_str() {
                        "debug" => debug!("plugin: {}", message),
                        "info" => info!("plugin: {}", message),
                        "warn" => warn!("plugin: {}", message),
                        "error" => error!("plugin: {}", message),
                        _ => info!("plugin: {}", message),
                    }
                    Ok(())
                })
                .map_err(lua_err)?;
            repoman.set("log", log_fn).map_err(lua_err)?;

            // repoman.exec(command) -> stdout string
            let exec_fn = lua
                .create_function(|_, command: String| {
                    let output = std::process::Command::new("sh")
                        .arg("-c")
                        .arg(&command)
                        .output();
                    match output {
                        Ok(out) => {
                            let stdout = String::from_utf8_lossy(&out.stdout).to_string();
                            Ok(stdout)
                        }
                        Err(e) => Err(LuaError::external(e)),
                    }
                })
                .map_err(lua_err)?;
            repoman.set("exec", exec_fn).map_err(lua_err)?;

            // repoman.vault table
            let vault_table = lua.create_table().map_err(lua_err)?;

            // repoman.vault.list() -> table of repo names
            let repo_names: Vec<String> = vault
                .map(|v| v.get_all_names().into_iter().map(String::from).collect())
                .unwrap_or_default();
            let names = repo_names.clone();
            let vault_list_fn = lua
                .create_function(move |lua, ()| {
                    let t = lua.create_table()?;
                    for (i, name) in names.iter().enumerate() {
                        t.set(i + 1, name.as_str())?;
                    }
                    Ok(t)
                })
                .map_err(lua_err)?;
            vault_table.set("list", vault_list_fn).map_err(lua_err)?;

            // repoman.vault.info(name) -> table with url, etc.
            let entries: Vec<(String, String)> = vault
                .map(|v| {
                    v.entries
                        .iter()
                        .map(|e| (e.name.clone(), e.url.clone()))
                        .collect()
                })
                .unwrap_or_default();
            let vault_info_fn = lua
                .create_function(move |lua, name: String| {
                    if let Some((_, url)) = entries.iter().find(|(n, _)| n == &name) {
                        let t = lua.create_table()?;
                        t.set("name", name.as_str())?;
                        t.set("url", url.as_str())?;
                        Ok(LuaValue::Table(t))
                    } else {
                        Ok(LuaValue::Nil)
                    }
                })
                .map_err(lua_err)?;
            vault_table.set("info", vault_info_fn).map_err(lua_err)?;

            repoman.set("vault", vault_table).map_err(lua_err)?;

            globals.set("repoman", repoman).map_err(lua_err)?;
        }

        Ok(Self {
            lua,
            loaded: Vec::new(),
        })
    }

    /// Load all .lua files from the plugins directory.
    pub fn load_plugins(&mut self, plugins_dir: &Path) -> Result<()> {
        if !plugins_dir.exists() {
            debug!(
                "plugins: directory does not exist: {}",
                plugins_dir.display()
            );
            return Ok(());
        }

        let entries = std::fs::read_dir(plugins_dir)?;
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "lua") {
                let name = path
                    .file_stem()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_default();
                debug!("plugins: loading '{}'", path.display());

                match self.lua.load(path.as_path()).exec() {
                    Ok(()) => {
                        info!("plugins: loaded '{}'", name);
                        self.loaded.push(name);
                    }
                    Err(e) => {
                        warn!("plugins: failed to load '{}': {}", path.display(), e);
                    }
                }
            }
        }

        Ok(())
    }

    /// Run all registered callbacks for a given event.
    pub fn run_hook(&self, event: &str, context: &HookContext) -> Result<()> {
        let hooks: LuaTable = self
            .lua
            .named_registry_value("repoman_hooks")
            .map_err(lua_err)?;

        let callbacks: LuaTable = match hooks.get::<LuaTable>(event) {
            Ok(t) => t,
            Err(_) => return Ok(()), // no callbacks for this event
        };

        // Build context table
        let ctx = self.lua.create_table().map_err(lua_err)?;
        ctx.set("repo", context.repo.as_str()).map_err(lua_err)?;
        ctx.set("event", context.event.as_str()).map_err(lua_err)?;
        if let Some(ref p) = context.pristine_path {
            ctx.set("pristine_path", p.as_str()).map_err(lua_err)?;
        }
        if let Some(ref p) = context.clone_path {
            ctx.set("clone_path", p.as_str()).map_err(lua_err)?;
        }
        if let Some(ref n) = context.clone_name {
            ctx.set("clone_name", n.as_str()).map_err(lua_err)?;
        }
        if let Some(ref t) = context.new_tag {
            ctx.set("new_tag", t.as_str()).map_err(lua_err)?;
        }

        for pair in callbacks.pairs::<LuaValue, LuaFunction>() {
            if let Ok((_, func)) = pair
                && let Err(e) = func.call::<()>(ctx.clone())
            {
                warn!("plugins: hook '{}' callback failed: {}", event, e);
            }
        }

        Ok(())
    }

    pub fn list_loaded(&self) -> &[String] {
        &self.loaded
    }
}

fn lua_err(e: LuaError) -> RepomanError {
    RepomanError::ConfigError(format!("Lua error: {}", e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plugin_manager_new() {
        let pm = PluginManager::new(None).unwrap();
        assert!(pm.list_loaded().is_empty());
    }

    #[test]
    fn test_plugin_load_and_hook() {
        let temp = tempfile::tempdir().unwrap();
        let plugin_path = temp.path().join("test.lua");
        std::fs::write(
            &plugin_path,
            r#"
            repoman.on("post_clone", function(ctx)
                repoman.log("info", "test hook fired for " .. ctx.repo)
            end)
            "#,
        )
        .unwrap();

        let mut pm = PluginManager::new(None).unwrap();
        pm.load_plugins(temp.path()).unwrap();
        assert_eq!(pm.list_loaded(), &["test"]);

        let ctx = HookContext {
            repo: "my-repo".to_string(),
            event: "post_clone".to_string(),
            pristine_path: None,
            clone_path: None,
            clone_name: None,
            new_tag: None,
        };
        pm.run_hook("post_clone", &ctx).unwrap();
    }

    #[test]
    fn test_plugin_exec() {
        let temp = tempfile::tempdir().unwrap();
        let plugin_path = temp.path().join("exec_test.lua");
        std::fs::write(
            &plugin_path,
            r#"
            local out = repoman.exec("echo hello")
            repoman.log("info", "exec output: " .. out)
            "#,
        )
        .unwrap();

        let mut pm = PluginManager::new(None).unwrap();
        pm.load_plugins(temp.path()).unwrap();
        assert_eq!(pm.list_loaded(), &["exec_test"]);
    }

    #[test]
    fn test_plugin_vault_list() {
        let mut vault = crate::vault::Vault::default();
        vault
            .add_entry("repo1".to_string(), "url1".to_string())
            .unwrap();
        vault
            .add_entry("repo2".to_string(), "url2".to_string())
            .unwrap();

        let temp = tempfile::tempdir().unwrap();
        let plugin_path = temp.path().join("vault_test.lua");
        std::fs::write(
            &plugin_path,
            r#"
            local repos = repoman.vault.list()
            repoman.log("info", "found " .. #repos .. " repos")
            "#,
        )
        .unwrap();

        let mut pm = PluginManager::new(Some(&vault)).unwrap();
        pm.load_plugins(temp.path()).unwrap();
        assert_eq!(pm.list_loaded(), &["vault_test"]);
    }
}
