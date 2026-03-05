# Lua Plugins

Repoman supports Lua plugins for custom automation. Plugins are `.lua` files placed in the plugins directory that register callbacks for lifecycle events. They complement [shell hooks](hooks.md): hooks are per-repo, plugins are global.

## Plugin Location

Plugins are loaded from the directory specified by `plugins_dir` in `config.yaml`:

```yaml
plugins_dir: ~/.config/repoman/plugins
```

Default: `~/.config/repoman/plugins/`

All files with a `.lua` extension in this directory are loaded automatically at startup. The load order is determined by the filesystem (typically alphabetical). Plugins are loaded for all commands except `completions`, `config`, `doctor`, and `man`.

## API Reference

Plugins interact with repoman through the `repoman` global table.

### repoman.on(event, callback)

Register a callback function for a lifecycle event.

```lua
repoman.on("post_clone", function(ctx)
    repoman.log("info", "Clone created for " .. ctx.repo)
end)
```

**Parameters:**

- `event` (string) -- The event name. Same names as shell hooks: `post_init_pristine`, `pre_clone`, `post_clone`, `post_sync`, `post_sync_on_new_tag`, `pre_destroy`, `post_destroy`.
- `callback` (function) -- A function that receives a context table.

Multiple callbacks can be registered for the same event (even across different plugins). They run in registration order.

### repoman.log(level, message)

Write a message to repoman's log file.

```lua
repoman.log("info", "Plugin initialized")
repoman.log("debug", "Processing repo: " .. name)
repoman.log("warn", "Something unexpected happened")
repoman.log("error", "Critical failure")
```

**Parameters:**

- `level` (string) -- One of: `debug`, `info`, `warn`, `error`.
- `message` (string) -- The log message.

Messages appear in `~/.repoman/logs/repoman.log` prefixed with `plugin:`. Use `--debug` to also see them in the terminal.

### repoman.exec(command)

Execute a shell command and return its stdout as a string.

```lua
local output = repoman.exec("git --version")
repoman.log("info", "Git version: " .. output)
```

**Parameters:**

- `command` (string) -- Shell command to execute via `sh -c`.

**Returns:** stdout of the command as a string (lossy UTF-8 conversion).

### repoman.vault.list()

Return a table of all repository names in the vault.

```lua
local repos = repoman.vault.list()
for i, name in ipairs(repos) do
    repoman.log("info", "Repo: " .. name)
end
```

**Returns:** A Lua table (array) of repository name strings. Reflects the vault state at plugin load time.

### repoman.vault.info(name)

Return information about a specific repository.

```lua
local info = repoman.vault.info("my-repo")
if info then
    repoman.log("info", "URL: " .. info.url)
end
```

**Parameters:**

- `name` (string) -- Repository name.

**Returns:** A table with `name` and `url` fields, or `nil` if the repo is not found.

## Events

Plugins receive the same lifecycle events as shell hooks. After any configured shell hook runs for a given event and repo, all plugin callbacks registered for that event are invoked.

| Event | When it fires |
|-------|---------------|
| `post_init_pristine` | After a pristine is created by `repoman init` |
| `pre_clone` | Before a clone is created |
| `post_clone` | After a clone is created and metadata saved |
| `post_sync` | After a pristine is synced |
| `post_sync_on_new_tag` | After an agent sync detects a new tag |
| `pre_destroy` | Before a clone is removed |
| `post_destroy` | After a clone is removed |

### Context Object

Every callback receives a context table (`ctx`) with the following fields:

| Field | Type | Description |
|-------|------|-------------|
| `ctx.repo` | string | Repository name |
| `ctx.event` | string | Event name |
| `ctx.pristine_path` | string or nil | Absolute path to the pristine directory |
| `ctx.clone_path` | string or nil | Absolute path to the clone directory |
| `ctx.clone_name` | string or nil | Clone directory name (e.g., `my-repo-feature`) |
| `ctx.new_tag` | string or nil | New tag name (only for `post_sync_on_new_tag`) |

Which fields are populated depends on the event. For example, `post_clone` has both `pristine_path` and `clone_path`, while `post_sync` only has `pristine_path`.

## Example Plugins

The `examples/plugins/` directory contains ready-to-use plugins. Copy them to `~/.config/repoman/plugins/` to activate.

### tmux.lua -- Tmux session per clone

Creates a tmux session named after the clone on `post_clone` and kills it on `pre_destroy`.

```lua
repoman.on("post_clone", function(ctx)
    local session = ctx.clone_name:gsub("[%.%-]", "_")
    repoman.exec("tmux new-session -d -s " .. session .. " -c " .. ctx.clone_path)
    repoman.log("info", "tmux session '" .. session .. "' created")
end)

repoman.on("pre_destroy", function(ctx)
    local session = ctx.clone_name:gsub("[%.%-]", "_")
    repoman.exec("tmux kill-session -t " .. session .. " 2>/dev/null || true")
end)
```

### auto_deps.lua -- Auto-install dependencies

Detects the project type by checking for manifest files and installs dependencies after clone.

```lua
repoman.on("post_clone", function(ctx)
    local path = ctx.clone_path
    local checks = {
        { file = "package.json", cmd = "npm ci" },
        { file = "Cargo.toml",  cmd = "cargo fetch" },
        { file = "go.mod",      cmd = "go mod download" },
        { file = "requirements.txt", cmd = "pip install -r requirements.txt" },
        { file = "Gemfile",     cmd = "bundle install" },
        { file = "pyproject.toml", cmd = "pip install -e ." },
    }
    for _, check in ipairs(checks) do
        local f = io.open(path .. "/" .. check.file, "r")
        if f then
            f:close()
            repoman.log("info", "Detected " .. check.file .. ", running: " .. check.cmd)
            repoman.exec("cd " .. path .. " && " .. check.cmd)
            return
        end
    end
end)
```

### sync_report.lua -- Sync event log

Appends sync events to a report file for tracking sync history.

```lua
repoman.on("post_sync", function(ctx)
    local report = os.getenv("HOME") .. "/.repoman/logs/sync_report.log"
    local f = io.open(report, "a")
    if f then
        f:write(os.date("%Y-%m-%d %H:%M:%S") .. " | " .. ctx.repo .. " | synced\n")
        f:close()
    end
end)

repoman.on("post_sync_on_new_tag", function(ctx)
    local report = os.getenv("HOME") .. "/.repoman/logs/sync_report.log"
    local f = io.open(report, "a")
    if f then
        f:write(os.date("%Y-%m-%d %H:%M:%S") .. " | " .. ctx.repo
            .. " | new tag: " .. (ctx.new_tag or "unknown") .. "\n")
        f:close()
    end
end)
```

## Best Practices

- **Keep plugins focused.** One plugin per concern (tmux management, dependency installation, notifications) is easier to maintain than a monolithic script.
- **Guard against nil fields.** Not all context fields are set for every event. Check for `nil` before using `ctx.clone_path` or `ctx.new_tag`.
- **Use `repoman.log()` for diagnostics.** Plugin log messages go to `repoman.log` and are visible with `--debug`. Avoid `print()` which bypasses the logging system.
- **Avoid blocking commands in `repoman.exec()`.** Long-running commands delay repoman's response. For heavy tasks, background them: `repoman.exec("make &")`.
- **Plugins fire for all repos.** If a plugin should only act on specific repos, check `ctx.repo` in the callback and return early for repos you want to skip.

## Troubleshooting

**Plugin not loading:**
- Verify the file is in the plugins directory: `repoman config` shows `plugins_dir`.
- Check the file has a `.lua` extension.
- Run with `--debug` to see load errors in the terminal.

**Callback not firing:**
- Confirm the event name is spelled correctly (e.g., `post_clone` not `postClone`).
- Check the log for plugin load errors: `grep plugin ~/.repoman/logs/repoman.log`.

**`repoman.exec()` returns empty string:**
- The command may be writing to stderr instead of stdout. Redirect: `repoman.exec("my-cmd 2>&1")`.

**Plugin errors:**
- Plugin errors are caught and logged as warnings. A failing plugin does not break repoman.
- Plugins use Lua 5.4 (vendored via mlua, no system Lua required).
- Plugins run in the same process as repoman, so they share the same permissions and environment.
