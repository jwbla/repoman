# Repoman Implementation Roadmap

This roadmap breaks down the implementation into small, single-point stories that can be completed one at a time.

## Phase 1: Foundation (COMPLETED)
- [x] Project setup with dependencies (clap, serde, git2, tokio, etc.)
- [x] CLI structure with all commands (stubs)
- [x] Config module with optional YAML loading and defaults
- [x] Directory structure creation/checking

## Phase 2: Vault Management

### 2.1 Vault Data Structures
- [ ] Create `src/vault.rs` module
- [ ] Define `Vault` struct to represent `vault.json` (Vec of repo entries)
- [ ] Define `VaultEntry` struct with fields: name, url, added_date
- [ ] Implement `Vault::load()` to read from `~/.repoman/vault/vault.json`
- [ ] Implement `Vault::save()` to write to vault.json
- [ ] Implement `Vault::add_entry()` to add new repo to vault
- [ ] Implement `Vault::get_all_names()` to list all vaulted repo names

### 2.2 Metadata Data Structures
- [ ] Define `Metadata` struct matching spec (git_url, created_on, last_updated, default_branch, tracked_branches, clones array, readme, sync_interval, last_sync, build_config, hook_configs)
- [ ] Implement `Metadata::load(repo_name, config)` to read from `~/.repoman/vault/<repo-name>/metadata.json`
- [ ] Implement `Metadata::save(repo_name, config)` to write metadata.json
- [ ] Implement `Metadata::new(url)` to create initial metadata with current timestamp

## Phase 3: Add Command

### 3.1 URL Detection
- [ ] Create `src/operations/add.rs` module
- [ ] Implement `detect_current_repo_url()` using git2 to get origin remote from current directory
- [ ] Handle case where current dir is not a git repo (return error)

### 3.2 Add Operation
- [ ] Implement `add_repo(url: Option<String>, config: &Config)` function
- [ ] If url is None, call `detect_current_repo_url()`
- [ ] Extract repo name from URL (parse git URL to get repo name)
- [ ] Check if repo already exists in vault (prevent duplicates)
- [ ] Create vault entry and add to vault.json
- [ ] Create `~/.repoman/vault/<repo-name>/` directory
- [ ] Create initial metadata.json with `Metadata::new()`
- [ ] Save vault.json and metadata.json

### 3.3 Add Command Handler
- [ ] Create `src/commands/add.rs` module
- [ ] Implement `handle_add()` function that calls `operations::add::add_repo()`
- [ ] Update `main.rs` to call `commands::add::handle_add()` in Add match arm
- [ ] Add error handling and user-friendly messages

### 3.4 Tag Detection (Optional Enhancement)
- [ ] Implement `get_latest_remote_tag(url)` using git2 to list remote tags
- [ ] Store latest tag in metadata when adding repo
- [ ] Handle repos with no tags gracefully

## Phase 4: Init Command (Parallel Implementation)

### 4.1 Init Operation
- [ ] Create `src/operations/init.rs` module
- [ ] Implement `init_pristine(repo_name: &str, config: &Config)` function
- [ ] Load metadata to get git URL
- [ ] Use git2 to create reference clone at `~/.repoman/pristines/<repo-name>/`
- [ ] Handle errors (invalid URL, network issues, etc.)
- [ ] Update metadata with pristine creation timestamp

### 4.2 Init Command Handler (Sequential First)
- [ ] Create `src/commands/init.rs` module
- [ ] Implement `handle_init()` function
- [ ] If vault_name provided, init single repo
- [ ] If no vault_name, get all vaulted repos and init each sequentially
- [ ] Update `main.rs` to call handler in Init match arm

### 4.3 Parallel Init Implementation
- [ ] Refactor `handle_init()` to use `tokio::task::spawn_blocking` for each repo
- [ ] Spawn one task per repo (parallel execution)
- [ ] Collect all task handles
- [ ] Wait for all tasks and report success/failure per repo
- [ ] Handle errors gracefully (some succeed, some fail)

## Phase 5: Clone Command

### 5.1 Clone Operation
- [ ] Create `src/operations/clone.rs` module
- [ ] Implement `clone_from_pristine(pristine_name: &str, clone_name: Option<String>, config: &Config)` function
- [ ] Check if pristine exists
- [ ] Generate clone name if not provided (format: `<pristine-name>-<timestamp>` or similar)
- [ ] Copy pristine to `~/.repoman/clones/<pristine-name>-<clone-name>/`
- [ ] Update metadata to add clone entry to clones array
- [ ] Save updated metadata

### 5.2 Clone Command Handler
- [ ] Create `src/commands/clone.rs` module
- [ ] Implement `handle_clone()` function
- [ ] Call `operations::clone::clone_from_pristine()`
- [ ] Update `main.rs` to call handler in Clone match arm
- [ ] Add validation (pristine must exist)

## Phase 6: Destroy Command

### 6.1 Destroy Operation
- [ ] Create `src/operations/destroy.rs` module
- [ ] Implement `destroy_clone(clone_name: &str, config: &Config)` function
- [ ] Find clone in metadata (search clones array)
- [ ] Remove clone directory from filesystem
- [ ] Update metadata to remove clone entry
- [ ] Save updated metadata
- [ ] Implement `destroy_pristine(pristine_name: &str, config: &Config)` function
- [ ] Remove pristine directory from filesystem
- [ ] Keep repo in vault (don't remove from vault.json)
- [ ] Optionally clear pristine reference in metadata

### 6.2 Destroy Command Handler
- [ ] Create `src/commands/destroy.rs` module
- [ ] Implement `handle_destroy()` function
- [ ] Determine if target is a clone or pristine (check if exists in clones array vs pristines)
- [ ] Call appropriate destroy function
- [ ] Update `main.rs` to call handler in Destroy match arm
- [ ] Add validation and error messages

## Phase 7: Sync Command (Parallel Implementation)

### 7.1 Sync Operation
- [ ] Create `src/operations/sync.rs` module
- [ ] Implement `sync_pristine(pristine_name: &str, config: &Config)` function
- [ ] Load metadata to get git URL
- [ ] Use git2 to fetch from origin and update pristine
- [ ] Update metadata with last_sync timestamp (manual)
- [ ] Handle errors (network, conflicts, etc.)

### 7.2 Sync Command Handler (Sequential First)
- [ ] Create `src/commands/sync.rs` module
- [ ] Implement `handle_sync()` function
- [ ] If pristine provided, sync single repo
- [ ] If no pristine, get all repos with pristines and sync each sequentially
- [ ] Update `main.rs` to call handler in Sync match arm

### 7.3 Parallel Sync Implementation
- [ ] Refactor `handle_sync()` to use `tokio::task::spawn_blocking` for each repo
- [ ] Spawn one task per repo (parallel execution)
- [ ] Collect all task handles
- [ ] Wait for all tasks and report success/failure per repo
- [ ] Show progress/output per repo as they complete

## Phase 8: Agent - Basic Infrastructure

### 8.1 Agent Process Management
- [ ] Create `src/agent.rs` module
- [ ] Implement `agent_pid_file_path(config)` to get path for PID file
- [ ] Implement `is_agent_running(config)` to check if agent process exists
- [ ] Implement `start_agent(config)` to spawn background process
- [ ] Implement `stop_agent(config)` to kill agent process
- [ ] Implement `get_agent_status(config)` to return running/stopped status

### 8.2 Agent Command Handler
- [ ] Create `src/commands/agent.rs` module
- [ ] Implement `handle_agent(action: &str, config: &Config)` function
- [ ] Handle "start" action (check if already running, spawn if not)
- [ ] Handle "stop" action (check if running, kill if so)
- [ ] Handle "status" action (show current status)
- [ ] Update `main.rs` to call handler in Agent match arm
- [ ] Add validation for action parameter

### 8.3 Agent Main Loop (Stub)
- [ ] Create agent entry point (separate binary or flag)
- [ ] Implement basic agent loop that runs continuously
- [ ] Add sleep interval (hardcoded for now)
- [ ] Add graceful shutdown on SIGTERM/SIGINT

## Phase 9: Agent - Periodic Polling

### 9.1 Tag Checking
- [ ] Implement `check_for_new_tag(repo_name: &str, config: &Config)` in operations
- [ ] Load metadata to get current version and git URL
- [ ] Use git2 to list remote tags (like `git ls-remote --tags`)
- [ ] Compare latest remote tag with stored version in metadata
- [ ] Return Option<String> for new version if found

### 9.2 Agent Polling Loop
- [ ] In agent main loop, get all vaulted repos
- [ ] For each repo, spawn `check_for_new_tag()` task (parallel)
- [ ] If new tag found, update metadata with new version
- [ ] Log new version discoveries
- [ ] Use per-repo sync_interval from metadata (or default)
- [ ] Sleep between polling cycles

### 9.3 Auto-Sync on New Version (Optional)
- [ ] Add option to auto-sync when new version detected
- [ ] Call `sync_pristine()` when new tag found
- [ ] Make this configurable per-repo in metadata

## Phase 10: Code Organization & Polish

### 10.1 Extract Command Handlers
- [ ] Move all command match arms to separate handler functions in command modules
- [ ] Clean up `main.rs` to just parse and route
- [ ] Ensure all error handling is consistent

### 10.2 Error Handling Improvements
- [ ] Create custom error types in `src/error.rs`
- [ ] Use `thiserror` or `anyhow::Context` for better error messages
- [ ] Add context to all error returns
- [ ] Ensure user-friendly error messages

### 10.3 Logging
- [ ] Add `tracing` or `log` crate
- [ ] Replace `println!` with proper logging
- [ ] Add log levels (info, warn, error)
- [ ] Write agent logs to `~/.repoman/logs/`

## Phase 11: Future Enhancements (Post-Beta)

### 11.1 Hooks System
- [ ] Define hook configuration in metadata
- [ ] Implement hook execution before/after operations
- [ ] Support async hooks

### 11.2 Plugin System
- [ ] Add Lua integration (mlua crate)
- [ ] Implement plugin discovery in `~/.repoman/plugins/`
- [ ] Add plugin command routing

### 11.3 Job Queue
- [ ] Implement job queue for agent
- [ ] Allow CLI to enqueue jobs instead of executing directly
- [ ] Agent processes queue in background

### 11.4 Additional Features
- [ ] Build configuration support
- [ ] Branch tracking
- [ ] README extraction
- [ ] Garbage collection for git repos

## Notes

- Each story should be completable in a single focused session
- Test each story before moving to the next
- Parallel implementations (init, sync) should be done after sequential versions work
- Agent can be basic for beta, full queue system can come later
- Focus on core operations first (add, init, clone, destroy, sync) before agent features

