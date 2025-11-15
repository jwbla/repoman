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
- [ ] Define `Metadata` struct matching spec (git_url as Vec<String> with element 0 being default, created_on, last_updated, default_branch, tracked_branches, clones array, readme, sync_interval, last_sync, build_config, hook_configs)
- [ ] Implement `Metadata::load(repo_name, config)` to read from `~/.repoman/vault/<repo-name>/metadata.json`
- [ ] Implement `Metadata::save(repo_name, config)` to write metadata.json
- [ ] Implement `Metadata::new(urls: Vec<String>)` to create initial metadata with current timestamp (urls[0] is default)

### 2.3 Phase 2 Unit Tests
- [ ] Write unit tests for vault operations (load, save, add_entry, get_all_names)
- [ ] Write unit tests for metadata operations (load, save, new)
- [ ] Write integration tests for vault and metadata interaction
- [ ] Ensure tests run in CI

## Phase 3: Add Command

### 3.1 URL Detection
- [ ] Create `src/operations/add.rs` module
- [ ] Implement `detect_current_repo_urls()` using git2 to get all remotes from current directory
- [ ] Determine default remote using git's actual default detection:
  - Check `branch.<current-branch>.remote` config (remote that current branch tracks)
  - If not set, check `remote.pushDefault` config
  - If not set, check if "origin" exists and use it
  - Otherwise use first remote alphabetically
- [ ] Return Vec<String> of all remote URLs with default as first element
- [ ] Handle case where current dir is not a git repo (return error)

### 3.2 Add Operation
- [ ] Implement `add_repo(url: Option<String>, config: &Config)` function
- [ ] If url is None, call `detect_current_repo_urls()` to get all remotes
- [ ] If url is Some, treat as single remote (Vec with one element)
- [ ] If multiple remotes detected, print console warning: "Multiple remotes detected. Adding all remotes with '{default_remote}' as default. You can change defaults later by editing metadata."
- [ ] Extract repo name from default URL (parse git URL to get repo name)
- [ ] Check if repo already exists in vault (prevent duplicates)
- [ ] Create vault entry and add to vault.json
- [ ] Create `~/.repoman/vault/<repo-name>/` directory
- [ ] Create initial metadata.json with `Metadata::new(urls)` where urls[0] is default
- [ ] Save vault.json and metadata.json

### 3.3 Add Command Handler
- [ ] Create `src/commands/add.rs` module
- [ ] Implement `handle_add()` function that calls `operations::add::add_repo()`
- [ ] Update `main.rs` to call `commands::add::handle_add()` in Add match arm
- [ ] Add error handling and user-friendly messages

### 3.4 Tag Detection (Optional Enhancement)
- [ ] Implement `get_latest_remote_tag(url)` using git2 to list remote tags from default remote
- [ ] Store latest tag in metadata when adding repo
- [ ] Handle repos with no tags gracefully

### 3.5 Phase 3 Unit Tests
- [ ] Create `tests/` directory structure
- [ ] Write unit tests for vault operations (load, save, add_entry)
- [ ] Write unit tests for metadata operations (load, save, new)
- [ ] Write unit tests for URL detection (single remote, multiple remotes, default detection via branch tracking, remote.pushDefault, origin fallback)
- [ ] Write unit tests for add operation (duplicate detection, metadata creation)
- [ ] Ensure tests run in CI (add test step to CI configuration)

## Phase 4: CI Infrastructure

### 4.1 CI Setup
- [ ] Create CI configuration file (e.g., `.github/workflows/ci.yml` or `.gitlab-ci.yml`)
- [ ] Configure CI to run on push and pull requests
- [ ] Set up Rust toolchain installation in CI
- [ ] Configure test execution: `cargo test`
- [ ] Configure linting: `cargo clippy` and `cargo fmt --check`
- [ ] Configure build verification: `cargo build --release`
- [ ] Add CI status badge to README (if applicable)

### 4.2 Phase 4 Unit Tests
- [ ] Verify CI pipeline runs successfully
- [ ] Ensure all existing tests pass in CI environment
- [ ] Document CI setup in project documentation

## Phase 5: Init Command (Parallel Implementation)

### 5.1 Init Operation
- [ ] Create `src/operations/init.rs` module
- [ ] Implement `init_pristine(repo_name: &str, config: &Config)` function
- [ ] Load metadata to get git URL (use default from git_url[0])
- [ ] Use git2 to create reference clone at `~/.repoman/pristines/<repo-name>/`
- [ ] Handle errors (invalid URL, network issues, etc.)
- [ ] Update metadata with pristine creation timestamp

### 5.2 Init Command Handler (Sequential First)
- [ ] Create `src/commands/init.rs` module
- [ ] Implement `handle_init()` function
- [ ] If vault_name provided, init single repo
- [ ] If no vault_name, get all vaulted repos and init each sequentially
- [ ] Update `main.rs` to call handler in Init match arm

### 5.3 Parallel Init Implementation
- [ ] Refactor `handle_init()` to use `tokio::task::spawn_blocking` for each repo
- [ ] Spawn one task per repo (parallel execution)
- [ ] Collect all task handles
- [ ] Wait for all tasks and report success/failure per repo
- [ ] Handle errors gracefully (some succeed, some fail)

### 5.4 Phase 5 Unit Tests
- [ ] Write unit tests for init operation (reference clone creation, error handling)
- [ ] Write unit tests for init command handler (single repo, all repos)
- [ ] Write integration tests for parallel init execution
- [ ] Ensure tests run in CI

## Phase 6: Clone Command

### 6.1 Clone Operation
- [ ] Create `src/operations/clone.rs` module
- [ ] Implement `clone_from_pristine(pristine_name: &str, clone_name: Option<String>, config: &Config)` function
- [ ] Check if pristine exists
- [ ] Generate clone name if not provided (format: `<pristine-name>-<timestamp>` or similar)
- [ ] Use git2 to create reference clone from pristine at `~/.repoman/clones/<pristine-name>-<clone-name>/` (space-efficient for large repos)
- [ ] Update metadata to add clone entry to clones array
- [ ] Save updated metadata

### 6.2 Clone Command Handler
- [ ] Create `src/commands/clone.rs` module
- [ ] Implement `handle_clone()` function
- [ ] Call `operations::clone::clone_from_pristine()`
- [ ] Update `main.rs` to call handler in Clone match arm
- [ ] Add validation (pristine must exist)

### 6.3 Phase 6 Unit Tests
- [ ] Write unit tests for clone operation (reference clone creation, name generation)
- [ ] Write unit tests for clone command handler
- [ ] Write integration tests verifying reference clone space efficiency
- [ ] Ensure tests run in CI

## Phase 7: List Command

### 7.1 List Operation
- [ ] Create `src/operations/list.rs` module
- [ ] Implement `list_all_repos(config: &Config)` function
- [ ] Load vault to get all vaulted repositories
- [ ] For each repo, load metadata to get:
  - Repo name and default git URL
  - Created date, last updated date
  - Whether pristine exists (check filesystem)
  - List of clones with their names, paths, and creation timestamps
  - Last sync timestamp (if available)
  - Default branch name
- [ ] Format output to show:
  - Vault entries (all repos)
  - Pristine status (initialized or not)
  - Clone instances with metadata (name, path, created date)
- [ ] Handle cases where metadata is missing or corrupted gracefully

### 7.2 List Command Handler
- [ ] Create `src/commands/list.rs` module
- [ ] Implement `handle_list()` function
- [ ] Call `operations::list::list_all_repos()`
- [ ] Format output in a readable way (table or tree structure)
- [ ] Support optional filtering (e.g., show only repos with pristines, show only repos with clones)
- [ ] Update `main.rs` to call handler in List match arm
- [ ] Add CLI argument parsing for list options (if needed)

### 7.3 Phase 7 Unit Tests
- [ ] Write unit tests for list operation (vault loading, metadata reading, status detection)
- [ ] Write unit tests for list command handler (output formatting, filtering)
- [ ] Write integration tests for list command with various repo states
- [ ] Ensure tests run in CI

## Phase 8: Destroy Command

### 8.1 Destroy Operation
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

### 8.2 Destroy Command Handler
- [ ] Create `src/commands/destroy.rs` module
- [ ] Implement `handle_destroy()` function
- [ ] Determine if target is a clone or pristine (check if exists in clones array vs pristines)
- [ ] Call appropriate destroy function
- [ ] Update `main.rs` to call handler in Destroy match arm
- [ ] Add validation and error messages

### 8.3 Phase 8 Unit Tests
- [ ] Write unit tests for destroy operations (clone and pristine destruction)
- [ ] Write unit tests for destroy command handler
- [ ] Write integration tests for destroy operations
- [ ] Ensure tests run in CI

## Phase 9: Sync Command (Parallel Implementation)

### 9.1 Sync Operation
- [ ] Create `src/operations/sync.rs` module
- [ ] Implement `sync_pristine(pristine_name: &str, config: &Config)` function
- [ ] Load metadata to get git URL (use default from git_url[0])
- [ ] Use git2 to fetch from origin and update pristine
- [ ] Update metadata with last_sync timestamp (manual)
- [ ] Handle errors (network, conflicts, etc.)

### 9.2 Sync Command Handler (Sequential First)
- [ ] Create `src/commands/sync.rs` module
- [ ] Implement `handle_sync()` function
- [ ] If pristine provided, sync single repo
- [ ] If no pristine, get all repos with pristines and sync each sequentially
- [ ] Update `main.rs` to call handler in Sync match arm

### 9.3 Parallel Sync Implementation
- [ ] Refactor `handle_sync()` to use `tokio::task::spawn_blocking` for each repo
- [ ] Spawn one task per repo (parallel execution)
- [ ] Collect all task handles
- [ ] Wait for all tasks and report success/failure per repo
- [ ] Show progress/output per repo as they complete

### 9.4 Phase 9 Unit Tests
- [ ] Write unit tests for sync operation (fetch, update, error handling)
- [ ] Write unit tests for sync command handler (single repo, all repos)
- [ ] Write integration tests for parallel sync execution
- [ ] Ensure tests run in CI

## Phase 10: Agent - Basic Infrastructure

### 10.1 Agent Process Management
- [ ] Create `src/agent.rs` module
- [ ] Implement `agent_pid_file_path(config)` to get path for PID file
- [ ] Implement `is_agent_running(config)` to check if agent process exists
- [ ] Implement `start_agent(config)` to spawn background process
- [ ] Implement `stop_agent(config)` to kill agent process
- [ ] Implement `get_agent_status(config)` to return running/stopped status

### 10.2 Agent Command Handler
- [ ] Create `src/commands/agent.rs` module
- [ ] Implement `handle_agent(action: &str, config: &Config)` function
- [ ] Handle "start" action (check if already running, spawn if not)
- [ ] Handle "stop" action (check if running, kill if so)
- [ ] Handle "status" action (show current status)
- [ ] Update `main.rs` to call handler in Agent match arm
- [ ] Add validation for action parameter

### 10.3 Agent Main Loop (Stub)
- [ ] Create agent entry point (separate binary or flag)
- [ ] Implement basic agent loop that runs continuously
- [ ] Add sleep interval (hardcoded for now)
- [ ] Add graceful shutdown on SIGTERM/SIGINT

### 10.4 Phase 10 Unit Tests
- [ ] Write unit tests for agent process management
- [ ] Write unit tests for agent command handler
- [ ] Write integration tests for agent lifecycle
- [ ] Ensure tests run in CI

## Phase 11: Agent - Periodic Polling

### 11.1 Tag Checking
- [ ] Implement `check_for_new_tag(repo_name: &str, config: &Config)` in operations
- [ ] Load metadata to get current version and git URL (use default from git_url[0])
- [ ] Use git2 to list remote tags (like `git ls-remote --tags`)
- [ ] Compare latest remote tag with stored version in metadata
- [ ] Return Option<String> for new version if found

### 11.2 Agent Polling Loop
- [ ] In agent main loop, get all vaulted repos
- [ ] For each repo, spawn `check_for_new_tag()` task (parallel)
- [ ] If new tag found, update metadata with new version
- [ ] Log new version discoveries
- [ ] Use per-repo sync_interval from metadata (or default)
- [ ] Sleep between polling cycles

### 11.3 Auto-Sync on New Version (Optional)
- [ ] Add option to auto-sync when new version detected
- [ ] Call `sync_pristine()` when new tag found
- [ ] Make this configurable per-repo in metadata

### 11.4 Phase 11 Unit Tests
- [ ] Write unit tests for tag checking operations
- [ ] Write unit tests for agent polling loop
- [ ] Write integration tests for periodic polling
- [ ] Ensure tests run in CI

## Phase 12: Code Organization & Polish

### 12.1 Extract Command Handlers
- [ ] Move all command match arms to separate handler functions in command modules
- [ ] Clean up `main.rs` to just parse and route
- [ ] Ensure all error handling is consistent

### 12.2 Error Handling Improvements
- [ ] Create custom error types in `src/error.rs`
- [ ] Use `thiserror` or `anyhow::Context` for better error messages
- [ ] Add context to all error returns
- [ ] Ensure user-friendly error messages

### 12.3 Logging
- [ ] Add `tracing` or `log` crate
- [ ] Replace `println!` with proper logging
- [ ] Add log levels (info, warn, error)
- [ ] Write agent logs to `~/.repoman/logs/`

### 12.4 Phase 12 Unit Tests
- [ ] Write unit tests for error handling improvements
- [ ] Write unit tests for logging functionality
- [ ] Ensure all tests pass with new error types
- [ ] Ensure tests run in CI

## Phase 13: Future Enhancements (Post-Beta)

### 13.1 Hooks System
- [ ] Define hook configuration in metadata
- [ ] Implement hook execution before/after operations
- [ ] Support async hooks

### 13.2 Plugin System
- [ ] Add Lua integration (mlua crate)
- [ ] Implement plugin discovery in `~/.repoman/plugins/`
- [ ] Add plugin command routing

### 13.3 Job Queue
- [ ] Implement job queue for agent
- [ ] Allow CLI to enqueue jobs instead of executing directly
- [ ] Agent processes queue in background

### 13.4 Additional Features
- [ ] Build configuration support
- [ ] Branch tracking
- [ ] README extraction
- [ ] Garbage collection for git repos

## Notes

- Each story should be completable in a single focused session
- Test each story before moving to the next
- **Unit tests must be written at the end of each phase and run in CI**
- Parallel implementations (init, sync) should be done after sequential versions work
- Agent can be basic for beta, full queue system can come later
- Focus on core operations first (add, init, clone, list, destroy, sync) before agent features
- **git_url is stored as Vec<String> with element 0 being the default remote**
- **Clone operations use reference clones for space efficiency**
- **Multiple remotes are supported with console warnings when detected**
- **List command provides visibility into vault status, pristines, and clones with metadata**

