# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build & Test Commands

```sh
cargo build                    # dev build
cargo build --release          # release build
cargo test                     # all tests (unit + integration)
cargo test vault               # run tests matching "vault"
cargo test --lib config        # run unit tests in config module only
cargo install --path .         # install to ~/.cargo/bin/
```

Requires Rust 1.85+ (edition 2024). System deps: openssl, libssh2 (see README.md for per-distro packages).

## Version Policy

Semver starting at current version in Cargo.toml. Bump patch version on every change unless told otherwise. Version appears in `--version` and `after_help` output via `env!("CARGO_PKG_VERSION")`.

## Architecture

Repoman is a git repository manager for disposable workspaces. The data flow is: **vault** (URL registry) -> **pristine** (bare reference clone) -> **clone** (lightweight working copy via git alternates).

### Module Layout

- **`src/main.rs`** - CLI definition (clap derive), logging init, command dispatch
- **`src/commands/`** - Thin command handlers that validate args and call into operations. Each file maps 1:1 to a CLI subcommand (add, init, clone_cmd, sync, destroy, list, agent).
- **`src/operations/`** - Business logic for each command. All git2 interactions live here. Files mirror commands: add, init, sync, clone_op, destroy, list.
- **`src/vault.rs`** - Vault CRUD (loads/saves `~/.repoman/vault/vault.json`), URL-to-name extraction
- **`src/metadata.rs`** - Per-repo metadata (loads/saves `~/.repoman/vault/<repo>/metadata.json`). Tracks clones, sync history, branches, hooks, auth config.
- **`src/config.rs`** - Config loading from `~/.config/repoman/config.yaml` with tilde expansion, falls back to defaults under `~/.repoman/`
- **`src/agent.rs`** - Background agent (PID file management, poll loop that checks for new tags and auto-syncs)
- **`src/error.rs`** - `RepomanError` enum via thiserror, `Result<T>` alias, auth error detection helpers

### Critical Pattern: git2 Credential Callbacks

Credential callbacks in `operations/init.rs` and `operations/sync.rs` use a `Cell<u32>` attempt counter to prevent infinite retry loops (git2 will keep calling the callback forever on public HTTPS repos that don't need auth). The `Cell` **must be declared before** `RemoteCallbacks` so it outlives the callbacks (Rust drop order is reverse declaration order). This pattern is duplicated in `operations/sync.rs::check_for_new_tag` as well.

### Error Handling

- Operations return `error::Result<T>` (alias for `Result<T, RepomanError>`)
- `main.rs::run()` returns `Box<dyn Error>` for the top-level dispatch
- Auth errors from git2 are detected via `error::is_auth_error()` and wrapped with SSH setup help text

### Logging

Dual-output via `simplelog::CombinedLogger`: file logger always at debug level (`~/.repoman/logs/repoman.log`), terminal logger only when `--debug` flag is passed.

### Data Storage

All state under `~/.repoman/`:
- `vault/vault.json` - master repo list
- `vault/<repo>/metadata.json` - per-repo metadata
- `pristines/<repo>/` - bare git clones
- `clones/<repo>-<suffix>/` - working copies (use git alternates pointing at pristine objects)
- `logs/` - repoman.log, agent.log, agent.pid
