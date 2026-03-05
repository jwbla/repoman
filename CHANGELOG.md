# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.3.6] - 2026-02-23

### Added
- **Shallow clone support**: `repoman init --depth N` creates shallow pristines. Also respects `clone_defaults.shallow: true` in per-repo config (uses depth 1).
- **Confirmation prompt for gc**: `repoman gc` now previews what would be deleted and asks for confirmation. Use `-y` or `--yes` to skip.
- **Progress bars**: `init` and `sync` now show indicatif progress bars (receiving/indexing phases with byte counts) instead of raw percentage output.
- **E2E integration test**: full pipeline test (add → init → clone → sync → destroy) against a local bare git repo.
- **Unit tests**: added tests for rebase.rs, credentials.rs, agent loop iteration, and dashboard rendering (ratatui TestBackend).
- **clippy::pedantic**: enabled project-wide with targeted allows for noisy categories.

### Changed
- **Migrated serde_yaml → serde_yml**: replaced unmaintained `serde_yaml 0.9` with API-compatible `serde_yml 0.0.12`.
- **Lazy plugin loading**: `init_plugins()` now skips vault loading entirely when no `.lua` files exist in the plugins directory.
- **Agent refactor**: extracted `run_agent_iteration()` from the agent loop for testability.
- **`get_agent_status()` returns `String`** instead of `Result<String>` (it can never fail).

### Removed
- Dead code: `FastForwardFailed` error variant, `list_clones()`, `init_all_pristines()`, `find_clone_owner()`, unused `search_query` dashboard field.

## [0.3.5] - 2026-02-21

### Added
- **Dual-remote clone setup**: new clones set `origin` to the source URL (GitHub, etc.) and `pristine` to the local pristine path. `git push`/`git pull` now target the real remote by default. Upstream tracking is configured automatically. Existing clones continue to work unchanged — all internal operations (update, heartbeat, status) detect both layouts.
- **`repoman list` shows clone names**: the summary table now displays clone directory names indented under each repo, not just a count.
- **`repoman open` TUI picker**: running `repoman open` with no arguments launches an interactive picker (rendered on stderr) to select a pristine or clone to `cd` into. Supports arrow-key navigation and type-to-filter.
- Shell wrappers (`shell-init`) updated to intercept no-arg `repoman open` for seamless `cd`.
- `RepomanError::Other` variant for general-purpose error messages.

## [0.3.4] - 2026-02-20

### Added
- **`max_parallel` config option**: limits concurrency for bulk operations (default 8). Set in `config.yaml` as `max_parallel: <n>`.
- **`repoman refresh`**: combined init + sync in one parallel pass — initializes missing pristines and syncs existing ones together.
- **Shared `run_parallel` utility**: semaphore-bounded parallel execution replaces unbounded `spawn_blocking` loops in `sync --all`, `init --all`, and `update --all`.

### Changed
- **Bulk sync/init/update**: now bounded by `max_parallel` semaphore instead of spawning unlimited concurrent tasks.
- **Agent sync phase**: repos due for sync are now processed in parallel (tag-check + sync), with hooks still dispatched sequentially for thread safety.
- **Agent heartbeat phase**: clone updates across repos run in parallel instead of sequentially.
- **Within-repo clone updates**: `update` fast-forwards clones in parallel via `std::thread::scope`, chunked by `max_parallel`.

## [0.3.3] - 2026-02-20

### Changed
- **Agent heartbeat**: replaced rebase with merge for diverged clones. Merge is always attempted in a temporary copy, never in the clone itself. On success the merged copy is swapped into place; on failure the clone is left untouched and `upstream_conflicts` is set.
- **`repoman open`**: prints a warning to stderr when opening a clone that has `upstream_conflicts` flagged, advising the user to resolve manually.

### Added
- **`no_upstream_merge`** per-repo config option (`repos.<name>.no_upstream_merge: true`). When set, the agent detects whether a merge would succeed but does not apply it — useful for repos where you want manual control over merges.

## [0.3.2] - 2026-02-20

### Added
- **`repoman shell-init <shell>`**: outputs shell completions plus a wrapper function for `eval` usage. The wrapper intercepts `repoman open <target>` to `cd` directly into the directory instead of just printing the path. Supports bash, zsh, and fish wrappers; other shells get completions only. Usage: `eval "$(repoman shell-init zsh)"`

## [0.3.1] - 2026-02-15

### Added
- **MCP server**: `repoman mcp` starts a Model Context Protocol server over stdio for LLM agent integration (e.g. Claude Code). Exposes 13 tools (`vault_list`, `vault_add`, `vault_remove`, `clone_create`, `clone_destroy`, `sync`, `status`, `open`, `update`, `gc`, `agent_status`, `export`, `import`) and 4 resources (`vault://state`, `vault://config`, `vault://repo/{name}/metadata`, `vault://repo/{name}/clones`). Hand-rolled JSON-RPC 2.0 implementation with no external MCP SDK dependency.
- `import_vault_from_string()` for importing vault YAML from a string (used by MCP import tool)
- `Serialize` derive on `StaleClone` and `GcReport` structs for JSON output

### Dependencies
- Added `libc` 0.2 (already a transitive dependency)

## [0.3.0] - 2026-02-15

### Added
- **Lazy pristine init**: `repoman clone` auto-initializes the pristine if it doesn't exist yet, collapsing `init` + `clone` into a single step
- **Shell completions**: `repoman completions <shell>` generates completions for bash, zsh, fish, elvish, and powershell
- **Man page generation**: `repoman man` generates a man page via clap_mangen
- **JSON output**: `--json` global flag for `list` and `status` commands; outputs machine-readable JSON for scripting and piping
- **Export/import**: `repoman export` dumps vault to YAML; `repoman import <path>` bulk-adds repos from YAML (skips duplicates, preserves aliases)
- **Lua plugin system**: `.lua` files in `~/.config/repoman/plugins/` can register callbacks via `repoman.on(event, fn)` for all lifecycle hooks; API includes `repoman.log()`, `repoman.exec()`, `repoman.vault.list()`, `repoman.vault.info(name)`. Plugins fire after shell hooks for the same event.
- **4 sample plugins**: `tmux.lua` (auto-create/kill tmux sessions), `auto_deps.lua` (detect package manager and install deps), `sync_report.lua` (log sync events to file), `salesforce.lua` (Salesforce CLI integration demo)
- **TUI dashboard**: `repoman dashboard` launches an interactive terminal UI with repo list, detail pane, agent status, and keyboard navigation
- **Agent heartbeat**: configurable `agent_heartbeat_interval` (default 300s) that updates clones from pristine state; fast-forwards where possible, attempts rebase for diverged clones, sets `upstream_conflicts` flag on failure
- **Config management**: `repoman config show|path|validate|init` subcommands for viewing and managing configuration
- **Health checks**: `repoman doctor` runs diagnostic checks on config, directories, vault integrity, pristines, clones, alternates, SSH, and agent status
- **Rename**: `repoman rename <old> <new>` renames vault entries, metadata, pristines, and retargets aliases
- **Confirmation prompts**: destructive commands (`destroy`, `remove`) now prompt for confirmation; skip with `-y` per-command or `--yes` globally
- **Colored output**: `list` and `status` commands use colored terminal output (bold labels, green/red status, yellow tags/branches, cyan repo names)
- **Relative time display**: last sync times shown as "2h ago", "3 days ago", etc.
- **Fuzzy name suggestions**: misspelled repo names trigger "Did you mean '...'?" suggestions using Jaro-Winkler similarity
- **Expanded config**: `config.yaml` now supports `agent_heartbeat_interval`, `json_output`, and per-repo `build`, `sync_interval`, `auth`, `default_branch`, `auto_init`, `clone_defaults`, and `tags` fields. Config overrides metadata for sync_interval, auth, and default_branch.
- **File locking**: vault.json and metadata.json writes use fs2 advisory locks to prevent corruption under concurrent access
- **GC alternates repack**: `repoman gc` now runs `git repack -adl` on clones to optimize shared object storage
- **User manual**: comprehensive documentation under `docs/manual/` covering all commands, configuration, hooks, plugins, and architecture
- **Strategic docs**: `docs/AGENT_SUGGESTIONS.md` (future agent capabilities), `docs/MCP_IDEAS.md` (MCP server integration for LLM agents), `docs/SLACK.md` (step-by-step Slack notification integration guide)
- **TEST_DRIVE.md overhaul**: complete walkthrough covering all v0.3.0 features

### Changed
- Legacy `metadata::HookConfig`, `metadata::BuildConfig`, and `metadata::readme` fields removed (no backward compatibility needed pre-v1)
- `CloneEntry` now includes `upstream_conflicts: bool` field (defaults to false, backward compatible via `#[serde(default)]`)
- `RepoStatus`, `DetailedStatus`, and `CloneStatus` now derive `Serialize` for JSON output
- Agent subcommand uses enum dispatch (`AgentAction`) instead of raw string matching
- `thiserror` upgraded to 2.0

### Dependencies
- Added `clap_complete` 4.5, `clap_mangen`, `mlua` 0.10 (Lua 5.4 vendored), `ratatui` 0.29, `crossterm` 0.28, `colored`, `fs2`, `strsim`, `indicatif`
- Removed `anyhow` (replaced by thiserror throughout)

## [0.2.4] - 2026-02-11

### Added
- Per-repo lifecycle hooks in `config.yaml` under `repos.<name>.hooks`: `post_init_pristine`, `pre_clone`, `post_clone`, `post_sync`, `pre_destroy`, `post_destroy`, and optional `post_sync_on_new_tag` for the agent
- Hooks run as shell commands (`sh -c "..."`) with env vars set: `REPOMAN_REPO`, `REPOMAN_EVENT`, `REPOMAN_PRISTINE_PATH`, `REPOMAN_CLONE_PATH`, `REPOMAN_CLONE_NAME`, and `REPOMAN_NEW_TAG` (for post_sync_on_new_tag)
- Hooks are invoked at the appropriate lifecycle points in init, clone, sync, destroy, and agent (post_sync and post_sync_on_new_tag after sync)

## [0.2.3] - 2026-02-11

### Added
- CI now uploads release artifacts from every run for Linux/macOS (`.tar.gz`) and Windows (`.zip` + `.msi`)
- New tag-driven `Release` workflow publishes packaged binaries/installers to GitHub Releases on `v*` tags
- Windows installer generation via WiX (`cargo-wix`) in CI/release workflows

## [0.2.2] - 2026-02-08

### Added
- GitHub Actions CI workflow — builds, tests, and lints on Linux, macOS, and Windows

## [0.2.1] - 2026-02-08

### Added
- `repoman remove <name>` — fully unregister a repository: destroys all clones, pristine, metadata, aliases, and vault entry
- `repoman destroy --all-pristines` — bulk-destroy all pristines across all vault entries (keeps vault entries)
- `Vault::remove_aliases_for()` — removes all aliases pointing at a given canonical repo name

## [0.2.0] - 2026-02-08

### Added
- `repoman status <name>` — deep inspection of a repository: clone branches, dirty state, ahead/behind, alternates health check
- `repoman open <target>` — print filesystem path for a pristine or clone (designed for `cd $(repoman open foo)`)
- `repoman alias` — manage short aliases for repository names; aliases resolve transparently in all commands
- `repoman update [<name>]` — sync pristine from remote then fast-forward all clones; parallel bulk mode when no name given
- `repoman gc` — garbage-collect stale clones (HEAD older than N days) and run `git gc --auto` on pristines; supports `--dry-run`
- `repoman clone --branch <branch>` — check out a specific branch when creating a clone from a pristine
- `repoman destroy --all-clones <name>` — bulk-destroy all clones for a given pristine
- `repoman destroy --stale <days>` — destroy clones with HEAD commits older than N days
- Per-repo `auth_config` (SSH key path, token env var) now wired into credential callbacks for init, sync, and tag checks
- Semver-aware tag sorting in agent tag checks — proper `v1.2.3` ordering instead of alphabetical
- Configurable per-repo sync interval — agent sleeps until the next repo is due instead of a fixed 1-hour poll
- Shared credential callback helper (`operations::credentials`) eliminates duplicated auth code

### Changed
- `repoman destroy` target is now optional (must provide one of `target`, `--all-clones`, or `--stale`)

## [0.1.4] - 2025-xx-xx

Initial tracked release with add, init, clone, sync, destroy, list, and agent commands.
