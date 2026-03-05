# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build & Test Commands

```sh
cargo build                    # dev build
cargo build --release          # release build
cargo test                     # all tests (unit + integration)
cargo test vault               # run tests matching "vault"
cargo test --lib config        # run unit tests in config module only
cargo clippy                   # lint checks
cargo fmt --check              # formatting check
cargo install --path .         # install to ~/.cargo/bin/
```

Requires Rust 1.85+ (edition 2024). System deps: openssl, libssh2 (see README.md for per-distro packages).

## Version Policy

Semver starting at current version in Cargo.toml. Bump patch version on every change unless told otherwise. Version appears in `--version` and `after_help` output via `env!("CARGO_PKG_VERSION")`.

## Architecture

Repoman is a git repository manager for disposable workspaces. The data flow is: **vault** (URL registry) -> **pristine** (bare reference clone) -> **clone** (lightweight working copy via git alternates).

### Module Layout

- **`src/main.rs`** - CLI definition (clap derive), logging init, plugin init, command dispatch. Global flags: `--debug`, `--json`, `-y/--yes`.
- **`src/commands/`** - Thin command handlers that validate args and call into operations:
  - `add.rs`, `init.rs`, `clone_cmd.rs`, `sync.rs`, `update.rs`, `status.rs`, `list.rs`, `open.rs`, `alias.rs`, `destroy.rs`, `remove.rs`, `gc.rs`, `agent.rs`
  - `config_cmd.rs` - config show/path/validate/init subcommands
  - `doctor.rs` - health check diagnostics
  - `rename.rs` - vault entry rename
  - `export_import.rs` - vault export/import to YAML
- **`src/operations/`** - Business logic. All git2 interactions live here:
  - `add.rs`, `init.rs`, `clone_op.rs`, `sync.rs`, `update.rs`, `status.rs`, `list.rs`, `open.rs`, `alias.rs`, `destroy.rs`, `remove.rs`, `gc.rs`
  - `credentials.rs` - Centralized git2 credential callback setup
  - `rebase.rs` - Agent heartbeat: clone fast-forward and rebase
  - `export_import.rs` - Vault YAML serialization
- **`src/vault.rs`** - Vault CRUD, URL-to-name extraction, alias resolution, file locking (fs2)
- **`src/metadata.rs`** - Per-repo metadata CRUD, clone tracking, file locking (fs2)
- **`src/config.rs`** - Config loading from `~/.config/repoman/config.yaml` with tilde expansion. Per-repo overrides: hooks, build, sync_interval, auth, default_branch, clone_defaults, tags. Merge helpers: `effective_sync_interval()`, `effective_auth()`, `effective_default_branch()`, `json_enabled()`.
- **`src/agent.rs`** - Background agent with per-repo sync scheduling and heartbeat clone updates
- **`src/hooks.rs`** - Shell hook execution + Lua plugin dispatch via global OnceLock
- **`src/plugins.rs`** - Lua plugin manager (mlua runtime, API bindings: `repoman.on()`, `repoman.log()`, `repoman.exec()`, `repoman.vault.list()`, `repoman.vault.info()`)
- **`src/dashboard.rs`** - Interactive TUI (ratatui + crossterm)
- **`src/util.rs`** - Shared utilities: `relative_time()`, `confirm()`, `suggest_similar()`, `truncate()`
- **`src/error.rs`** - `RepomanError` enum via thiserror, `Result<T>` alias, auth error detection helpers

### Critical Pattern: git2 Credential Callbacks

Credential callbacks use a `Cell<u32>` attempt counter via centralized `operations/credentials.rs` to prevent infinite retry loops. The `Cell` **must be declared before** `RemoteCallbacks` so it outlives the callbacks (Rust drop order is reverse declaration order). Auth config is resolved via `config.effective_auth(repo_name, &metadata)` which merges config.yaml overrides onto metadata defaults.

### Error Handling

- Operations return `error::Result<T>` (alias for `Result<T, RepomanError>`)
- `main.rs::run()` returns `Box<dyn Error>` for the top-level dispatch
- Auth errors from git2 are detected via `error::is_auth_error()` and wrapped with SSH setup help text
- `RepoNotInVault` errors trigger fuzzy name suggestion via `util::suggest_similar()`

### Logging

Dual-output via `simplelog::CombinedLogger`: file logger always at debug level (`~/.repoman/logs/repoman.log`), terminal logger only when `--debug` flag is passed.

### Data Storage

All state under `~/.repoman/`:
- `vault/vault.json` - master repo list (with fs2 advisory locking)
- `vault/<repo>/metadata.json` - per-repo metadata (with fs2 advisory locking)
- `pristines/<repo>/` - bare git clones
- `clones/<repo>-<suffix>/` - working copies (use git alternates pointing at pristine objects)
- `logs/` - repoman.log, agent.log, agent.pid

Config and plugins under `~/.config/repoman/`:
- `config.yaml` - configuration file (optional)
- `plugins/*.lua` - Lua plugin scripts (auto-loaded at startup)

### Hooks + Plugins

Shell hooks are per-repo (configured in config.yaml `repos.<name>.hooks`). Lua plugins are global (all `.lua` files in plugins_dir). After each shell hook runs, `hooks.rs` dispatches to the plugin manager for the same event. The plugin manager is stored via `OnceLock<PluginManagerPtr>` with an unsafe Send+Sync wrapper (safe because only accessed from the main thread).
