# Architecture

This document describes repoman's internal design, data flow, and storage layout.

## Core Concept: Three-Tier Model

Repoman organizes repositories in three tiers:

```
vault (URL registry + metadata)
  -> pristine (bare reference clone)
    -> clone (lightweight working copy)
```

### Vault

The vault is repoman's central registry. It stores:

- A list of repository entries (name, URL, add date)
- Alias mappings (short name -> canonical name)

Stored at: `~/.repoman/vault/vault.json`

Adding a repo to the vault does not clone anything. It just records the URL.

### Pristine

A pristine is a bare git clone of a vaulted repository. It contains all git objects (commits, trees, blobs) and refs (branches, tags) but no working tree.

Stored at: `~/.repoman/pristines/<name>/`

Pristines serve as the local reference for creating clones. They are synced from the remote origin periodically (manually or by the agent).

### Clone

A clone is a lightweight working copy created from a pristine. Instead of duplicating all git objects, clones use git's **alternates** mechanism to reference the pristine's object store.

Stored at: `~/.repoman/clones/<name>-<suffix>/`

Clones have a full working tree and can be used like any normal git checkout. They are intended to be disposable -- create one for a task, destroy it when done.

## Git Alternates

When repoman creates a clone, it:

1. Runs `git init` in a new directory.
2. Writes the pristine's objects path to `.git/objects/info/alternates`.
3. Adds the pristine as the `origin` remote.
4. Fetches refs from the pristine.
5. Checks out the requested branch.

Because of alternates, the clone does not duplicate object data. It shares the pristine's objects, making clone creation fast and disk-efficient. A typical clone adds only the working tree files and a small amount of git metadata.

The dependency chain means: **if you delete a pristine, its clones lose access to shared objects and will malfunction.** Always destroy clones before their pristine, or use `repoman remove` which handles the order automatically.

## Data Directory Layout

All state lives under `~/.repoman/` (customizable via config). Configuration and plugins live under `~/.config/repoman/`:

```
~/.repoman/
  vault/
    vault.json                    # master repo list + aliases
    .vault.lock                   # advisory lock file for concurrent writes
    <repo-name>/
      metadata.json               # per-repo metadata
      .metadata.lock              # advisory lock file for concurrent writes
  pristines/
    <repo-name>/                  # bare git repository
  clones/
    <repo-name>-<suffix>/         # working copy with alternates
  logs/
    repoman.log                   # main debug log (always written)
    agent.log                     # background agent output
    agent.pid                     # agent process ID file

~/.config/repoman/
  config.yaml                     # configuration file (optional)
  plugins/
    *.lua                         # Lua plugin scripts
```

## Metadata

Each repo has a metadata file at `~/.repoman/vault/<name>/metadata.json` containing:

| Field | Description |
|-------|-------------|
| `git_urls` | List of remote URLs (index 0 is the default) |
| `created_on` | When the repo was added to the vault |
| `last_updated` | Last metadata modification time |
| `default_branch` | Default branch name (if detected) |
| `tracked_branches` | List of branches being tracked |
| `clones` | Array of clone entries (name, path, created date, upstream_conflicts flag) |
| `sync_interval` | Seconds between agent syncs (default 3600) |
| `last_sync` | Timestamp and type of last sync (manual or auto) |
| `auth_config` | Per-repo auth settings (SSH key path, token env var) |
| `latest_tag` | Most recent tag detected by the agent |
| `pristine_created` | When the pristine was created |

## Module Layout

```
src/
  main.rs              # CLI definition (clap derive), logging, command dispatch
  commands/            # Thin command handlers -- validate args, call operations
    add.rs
    init.rs
    clone_cmd.rs
    sync.rs
    update.rs
    status.rs
    list.rs
    open.rs
    alias.rs
    destroy.rs
    remove.rs
    gc.rs
    agent.rs
    config_cmd.rs
    doctor.rs
    rename.rs
    export_import.rs
  operations/          # Business logic -- all git2 interactions live here
    add.rs
    init.rs
    clone_op.rs
    sync.rs
    update.rs
    status.rs
    list.rs
    open.rs
    alias.rs
    destroy.rs
    remove.rs
    gc.rs
    export_import.rs
    credentials.rs     # Centralized git2 credential callback setup
    rebase.rs          # Agent heartbeat: clone fast-forward and rebase
  vault.rs             # Vault CRUD, URL-to-name extraction, alias resolution
  metadata.rs          # Per-repo metadata CRUD, clone tracking
  config.rs            # Config loading from YAML, tilde expansion, per-repo overrides
  agent.rs             # Background agent (PID management, per-repo sync scheduling)
  hooks.rs             # Shell hook execution + Lua plugin dispatch
  plugins.rs           # Lua plugin manager (mlua runtime, API bindings)
  dashboard.rs         # Interactive TUI (ratatui + crossterm)
  util.rs              # Shared utility functions
  error.rs             # Error types (thiserror), auth error detection
```

### Commands vs Operations

The separation between `commands/` and `operations/` keeps the architecture clean:

- **Commands** handle CLI concerns: parsing arguments, formatting output, calling operations.
- **Operations** contain the business logic: loading vault/metadata, interacting with git2, managing the filesystem.

This makes operations testable in isolation and reusable (e.g., the agent calls operations directly without going through the command layer).

## Configuration

Config is loaded from `~/.config/repoman/config.yaml`. All path values support tilde expansion (`~/` is expanded to the user's home directory). If no config file exists, defaults are used with everything under `~/.repoman/`.

```yaml
vault_dir: ~/.repoman/vault
pristines_dir: ~/.repoman/pristines
clones_dir: ~/.repoman/clones
plugins_dir: ~/.config/repoman/plugins
logs_dir: ~/.repoman/logs
agent_heartbeat_interval: 300
json_output: false

repos:
  my-app:
    sync_interval: 1800
    default_branch: main
    hooks:
      post_clone: "npm ci"
    auth:
      ssh_key_path: ~/.ssh/id_deploy
```

Per-repo settings under `repos.<name>` include hooks, build commands, sync intervals, auth overrides, default branch, auto-init, clone defaults, and tags. Config values override metadata values when both are present.

## Credential Handling

Git authentication is managed through a centralized callback in `operations/credentials.rs`. The callback tries methods in order:

1. Explicit SSH key from per-repo auth config
2. SSH agent
3. Git default credentials
4. Token from an environment variable
5. Git credential helper

A `Cell<u32>` attempt counter prevents infinite retry loops. git2 will call the credential callback repeatedly on failure; the counter ensures only one attempt is made before returning an error. The `Cell` must be declared before the `RemoteCallbacks` struct due to Rust's drop order (reverse declaration order).

```rust
// CORRECT: Cell declared before RemoteCallbacks
let cred_attempts = Cell::new(0u32);
let mut callbacks = RemoteCallbacks::new();
setup_credentials(&mut callbacks, &cred_attempts, auth, "sync");
```

## Agent Architecture

The background agent (`repoman agent start`) spawns itself as a detached process running `repoman agent run`. It:

1. Loads the vault and iterates over repos with pristines.
2. Checks each repo's `sync_interval` against its `last_sync` timestamp. The interval is resolved from config override, then metadata, then the 3600-second default.
3. For due repos: checks for new tags, syncs the pristine, runs hooks (`post_sync_on_new_tag` if a new tag was detected).
4. On a separate heartbeat interval (default 300 seconds, configurable via `agent_heartbeat_interval`): fetches and fast-forwards (or rebases) clones from their pristine state.
5. Sleeps until the next repo is due, rather than polling on a fixed interval. The sleep duration is the minimum of time-until-next-sync and time-until-next-heartbeat.

The agent writes its PID to `~/.repoman/logs/agent.pid` and logs to `agent.log`.

## Hooks and Plugins

Repoman has two extensibility mechanisms that work together:

### Shell Hooks

Shell commands configured per-repo in `config.yaml` under `repos.<name>.hooks`. Seven hook points: `post_init_pristine`, `pre_clone`, `post_clone`, `post_sync`, `post_sync_on_new_tag`, `pre_destroy`, `post_destroy`. Hooks receive `REPOMAN_*` environment variables and run via `sh -c`. See [Hooks](hooks.md).

### Lua Plugins

Global `.lua` scripts in `~/.config/repoman/plugins/` that register callbacks for the same lifecycle events via `repoman.on(event, callback)`. Plugins fire for all repos (not per-repo). After shell hooks run for an event, plugin callbacks are invoked for the same event. See [Plugins](plugins.md).

The `hooks.rs` module orchestrates both: it runs the shell command if configured, then dispatches to the plugin manager for plugin callbacks.

## File Locking

Concurrent writes to `vault.json` and `metadata.json` are protected by `fs2` advisory file locks. When saving:

1. A lock file (`.vault.lock` or `.metadata.lock`) is created alongside the data file.
2. An exclusive lock is acquired via `lock_exclusive()`.
3. The data file is written.
4. The lock is released on drop when the lock file handle goes out of scope.

This prevents corruption when multiple repoman processes (e.g., the agent and a manual command) run simultaneously.

## Error Handling

Operations return `Result<T, RepomanError>` using a custom error enum built with `thiserror`. Error variants include:

- Vault/metadata load/save failures
- Repo not found / already exists
- Pristine not found / already exists
- Clone not found / already exists
- Authentication failures (with detailed SSH setup help text)
- Git operation failures
- Hook failures
- Agent state errors
- Configuration errors
- Branch not found / fast-forward failed

Auth errors from git2 are detected by inspecting error codes, classes, and message text via `is_auth_error()`, then wrapped with SSH setup instructions in `RepomanError::AuthenticationFailed`.

## Dependencies

Key crates:

| Crate | Purpose |
|-------|---------|
| `clap` | CLI argument parsing (derive mode) |
| `clap_complete` | Shell completion generation |
| `clap_mangen` | Man page generation |
| `git2` | Git operations (clone, fetch, branch, etc.) |
| `tokio` | Async runtime for parallel operations and agent |
| `serde` / `serde_json` / `serde_yml` | Serialization for vault, metadata, config |
| `chrono` | Timestamps |
| `semver` | Tag version comparison |
| `mlua` | Lua 5.4 plugin runtime |
| `ratatui` / `crossterm` | TUI dashboard |
| `simplelog` | Dual-output logging (file + terminal) |
| `thiserror` | Error type derivation |
| `fs2` | Advisory file locking |
| `colored` | Terminal color output |
| `dirs` | Home/config directory resolution |
