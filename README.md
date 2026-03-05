# Repoman

A blazing fast 🦀 git repository manager built in Rust around disposable workspaces. Repoman maintains a vault of repository URLs, creates space-efficient local reference clones (pristines), and lets you spin up and tear down working copies (clones) instantly.

## Why

- Keep a master list of repos without keeping them all on disk
- Create throwaway workspaces for feature branches, QA reviews, or LLM-assisted development
- Space-efficient: pristines are bare repos, clones use git alternates to share objects
- Background agent can auto-sync pristines, detect new tags, and keep clones updated
- Lua plugin system for automating post-clone setup, notifications, and more
- Interactive TUI dashboard for quick repo overview

## How It Works

```
vault (URLs + metadata)
  -> pristine (bare reference clone)
    -> clone (disposable working copy)
```

1. **Add** a repo URL to the vault (or auto-detect from current directory)
2. **Clone** creates a pristine (if needed) and a lightweight working copy
3. **Destroy** the clone when you're done -- the pristine stays
4. **Sync** pulls latest changes into pristines from origin

## Usage

```sh
# Core workflow
repoman add <git-url>              # add repo to vault
repoman add                        # auto-detect from current directory
repoman clone <name>               # create working copy (auto-inits pristine)
repoman clone <name> myfix -b dev  # named clone on specific branch
repoman sync [<name>]              # fetch latest from origin
repoman destroy <target>           # remove a clone or pristine

# Inspection
repoman list                       # summary table
repoman list -v                    # detailed view
repoman list --json                # JSON output for scripting
repoman status <name>              # detailed repo inspection
repoman status <name> --json       # JSON status output
repoman open <target>              # print path (for cd $(repoman open foo))

# Management
repoman init [<name>]              # manually create pristine(s)
repoman update [<name>]            # sync pristine + fast-forward clones
repoman alias <name> <alias>       # create alias for a repo
repoman alias                      # list all aliases
repoman rename <old> <new>         # rename a vault entry
repoman destroy --all-clones <n>   # destroy all clones for a pristine
repoman destroy --all-pristines    # destroy all pristines (keeps vault)
repoman destroy --stale <days>     # destroy clones older than N days
repoman remove <name> [-y]         # fully unregister repo + delete all data
repoman gc --days 30               # garbage-collect stale clones + repack
repoman gc --dry-run               # preview what gc would do

# Export/import
repoman export                     # dump vault to YAML
repoman export > repos.yaml        # save to file
repoman import repos.yaml          # bulk-add from YAML

# Agent
repoman agent start|stop|status    # background sync agent

# Config & diagnostics
repoman config [show|path|validate|init]  # view/manage configuration
repoman doctor                     # run health checks
repoman completions bash           # generate shell completions
repoman man                        # generate man page
repoman dashboard                  # interactive TUI
repoman --version
repoman --json <command>           # JSON output (list, status)
repoman -y <command>               # skip confirmation prompts
repoman --debug <command>          # print debug logs to console
```


## Build

Requires Rust 1.85+ and system libraries for git2 (OpenSSL, libssh2).

### System dependencies

Arch Linux:
```sh
sudo pacman -S openssl libssh2
```

Debian/Ubuntu:
```sh
sudo apt install libssl-dev libssh2-1-dev pkg-config cmake
```

Fedora:
```sh
sudo dnf install openssl-devel libssh2-devel
```

macOS:
```sh
brew install openssl libssh2
```

### Compile

```sh
cargo build --release
```

### Run tests

```sh
cargo test
```

## Install

### From GitHub Releases (prebuilt)

Download assets from the Releases page for tagged versions:
- Linux/macOS: `repoman-<OS>-x86_64-<tag>.tar.gz`
- Windows: `repoman-windows-x86_64-<tag>.zip` or `.msi` installer

### From GitHub Actions CI artifacts (latest build)

If you want a build from an untagged commit:
1. Open the latest successful CI run
2. Download the artifacts bundle for your OS
3. Extract and run/install:
   - Linux/macOS: unpack the `.tar.gz` and place `repoman` on your PATH
   - Windows: use the `.zip` binary or run the `.msi` installer

### cargo install (recommended)

```sh
cargo install --path .
```

Installs to `~/.cargo/bin/repoman`. Make sure `~/.cargo/bin` is in your PATH (rustup adds this by default).

Update after pulling changes:
```sh
cargo install --path . --force
```

### Manual

```sh
cargo build --release
cp target/release/repoman ~/.local/bin/   # or /usr/local/bin/
```

### Verify

```sh
repoman --version
```

## Configuration

Optional. Create `~/.config/repoman/config.yaml` to override default paths and define per-repo settings:

```yaml
vault_dir: ~/.repoman/vault
pristines_dir: ~/.repoman/pristines
clones_dir: ~/.repoman/clones
plugins_dir: ~/.config/repoman/plugins
logs_dir: ~/.repoman/logs
agent_heartbeat_interval: 300  # seconds between clone update checks

repos:
  neovim:
    sync_interval: 1800
    default_branch: master
    auto_init: true
    build:
      command: "make CMAKE_BUILD_TYPE=Release"
    hooks:
      post_clone: "make deps"
      post_sync: "echo 'neovim updated'"
    auth:
      ssh_key_path: ~/.ssh/id_github
    clone_defaults:
      branch: master
    tags: [editor, daily-driver]
  my-app:
    hooks:
      post_clone: "npm ci && npm run build"
      post_sync: "./scripts/deploy.sh"
```

### Hooks

7 lifecycle hook points: `post_init_pristine`, `pre_clone`, `post_clone`, `post_sync`, `post_sync_on_new_tag`, `pre_destroy`, `post_destroy`. Hooks run as shell commands with `REPOMAN_REPO`, `REPOMAN_EVENT`, `REPOMAN_PRISTINE_PATH`, `REPOMAN_CLONE_PATH`, `REPOMAN_CLONE_NAME`, and `REPOMAN_NEW_TAG` env vars set.

### Lua Plugins

Place `.lua` files in `~/.config/repoman/plugins/`. Plugins register callbacks via the `repoman.on(event, fn)` API. See `examples/plugins/` for sample plugins (tmux session management, auto dependency install, sync reporting, Salesforce CLI integration).

## Logs

Debug logs are written to `~/.repoman/logs/repoman.log` on every run. Use `--debug` to also print them to the console.

## Documentation

See `docs/manual/` for the full user manual, `docs/TEST_DRIVE.md` for a guided walkthrough, `docs/MCP_IDEAS.md` for LLM agent integration concepts, and `docs/SLACK.md` for Slack notification integration.
