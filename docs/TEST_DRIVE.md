# Repoman v0.3.6 Test Drive

A complete hands-on walkthrough of every Repoman feature. Follow this guide
from top to bottom on a fresh install, or jump to any section for a specific
capability.

---

## Table of Contents

- [Prerequisites](#prerequisites)
- [1. Adding Repositories to the Vault](#1-adding-repositories-to-the-vault)
- [2. Listing Repositories](#2-listing-repositories)
- [3. JSON Output and Piping](#3-json-output-and-piping)
- [4. Initializing Pristines](#4-initializing-pristines)
- [5. Creating Clones (Workspaces)](#5-creating-clones-workspaces)
- [6. Lazy Init Shortcut](#6-lazy-init-shortcut)
- [7. Syncing Pristines](#7-syncing-pristines)
- [8. Status Inspection](#8-status-inspection)
- [9. Open (Path Lookup)](#9-open-path-lookup)
- [10. Aliases](#10-aliases)
- [11. Update (Sync + Fast-Forward)](#11-update-sync--fast-forward)
- [12. Destroying Clones and Pristines](#12-destroying-clones-and-pristines)
- [13. Removing a Repository](#13-removing-a-repository)
- [14. Garbage Collection](#14-garbage-collection)
- [15. Export and Import](#15-export-and-import)
- [16. Shell Completions](#16-shell-completions)
- [17. Background Agent](#17-background-agent)
- [18. Agent Heartbeat Configuration](#18-agent-heartbeat-configuration)
- [19. TUI Dashboard](#19-tui-dashboard)
- [20. Lua Plugins](#20-lua-plugins)
- [21. Configuration Management](#21-configuration-management)
- [22. Health Checks (Doctor)](#22-health-checks-doctor)
- [23. Renaming Repositories](#23-renaming-repositories)
- [24. Man Pages](#24-man-pages)
- [25. Configuration File Reference](#25-configuration-file-reference)
- [26. MCP Server (LLM Agent Integration)](#26-mcp-server-llm-agent-integration)
- [Complete Workflow Example](#complete-workflow-example)
- [Directory Structure](#directory-structure)
- [Troubleshooting](#troubleshooting)

---

## Prerequisites

1. **Build and install repoman:**
   ```bash
   cargo build --release
   cargo install --path .
   ```

2. **Load your SSH key** (for private repos):
   ```bash
   ssh-add ~/.ssh/id_ed25519
   ```

3. **Verify the install:**
   ```bash
   repoman --version
   ```

---

## 1. Adding Repositories to the Vault

The vault is Repoman's registry of repository URLs. Adding a repo does not
clone anything -- it just records the URL for later use.

### Add by URL

```bash
repoman add https://github.com/neovim/neovim.git
repoman add git@github.com:torvalds/linux.git
```

### Auto-detect from current directory

```bash
cd ~/my-project
repoman add
# Detects the remote URL from the local .git config
```

### Verify

```bash
repoman list
```

---

## 2. Listing Repositories

### Summary view (default)

```bash
repoman list
```

Output:

```
NAME                 PRISTINE     CLONES   LAST SYNC
----------------------------------------------------------------
neovim               no           0        never
linux                no           0        never
```

### Verbose view

```bash
repoman list -v
```

Shows full details per repository: URL, added date, pristine status, clone
list, and last sync timestamp.

---

## 3. JSON Output and Piping

Both `list` and `status` support machine-readable JSON output via the
`--json` flag. This makes Repoman composable with tools like `jq`, scripts,
and other automation.

### List all repos as JSON

```bash
repoman list --json
```

### Pipe to jq

```bash
# Get names of all repos with pristines
repoman list --json | jq '[.[] | select(.pristine_exists == true) | .name]'

# Count total clones across all repos
repoman list --json | jq '[.[].clone_count] | add'

# Find repos that have never been synced
repoman list --json | jq '[.[] | select(.last_sync == null) | .name]'
```

### Status as JSON

```bash
repoman status neovim --json
repoman status neovim --json | jq '.clones[] | {name, branch, dirty_files}'
```

---

## 4. Initializing Pristines

A pristine is a bare reference clone that serves as the local source of truth.
Clones are created from pristines, not from the remote, making workspace
creation near-instant.

### Initialize a single repo

```bash
repoman init neovim
```

### Initialize all vaulted repos

```bash
repoman init
```

### Verify

```bash
repoman list
# PRISTINE column should now show "yes"

ls ~/.repoman/pristines/
# Bare git repositories appear here
```

---

## 5. Creating Clones (Workspaces)

Clones are lightweight working copies created from pristines. They use git
alternates to share object storage with the pristine, so they consume minimal
extra disk space.

### Auto-generated name

```bash
repoman clone neovim
# Creates ~/.repoman/clones/neovim-a7f3x2/
```

### Custom name

```bash
repoman clone neovim feature-work
# Creates ~/.repoman/clones/neovim-feature-work/
```

### Specific branch

```bash
repoman clone neovim bugfix -b release-0.10
# Creates clone checked out on the "release-0.10" branch
```

### Work in the clone

```bash
cd $(repoman open neovim-feature-work)
git checkout -b my-feature
# ... make changes ...
git commit -am "Implement feature"
git push origin my-feature
```

---

## 6. Lazy Init Shortcut

If you clone a repo that has not been initialized yet, Repoman will
automatically create the pristine first. No need to run `init` separately.

```bash
repoman add https://github.com/BurntSushi/ripgrep.git
repoman clone ripgrep my-workspace
# Pristine not found -- initializing from vault...
# Clone created: ~/.repoman/clones/ripgrep-my-workspace/
```

This is equivalent to running `repoman init ripgrep` followed by
`repoman clone ripgrep my-workspace`, but in a single command.

---

## 7. Syncing Pristines

Fetch the latest changes from the remote origin into your pristines.

### Sync a single repo

```bash
repoman sync neovim
```

### Sync all repos

```bash
repoman sync
```

### Verify

```bash
repoman list
# LAST SYNC column shows the timestamp
```

---

## 8. Status Inspection

Deep inspection of a single repository: clone branches, dirty file count,
ahead/behind tracking, and alternates health check.

```bash
repoman status neovim
```

Output:

```
Repository: neovim
  URL: https://github.com/neovim/neovim.git
  Pristine: yes
  Branches: main, release-0.10
  Latest tag: v0.10.4
  Last sync: 2026-02-15 14:30:00 UTC (manual)
  Sync interval: 3600s
  Clones (2):
    feature-work on main (3 dirty) [+2/-0]
    bugfix on release-0.10
```

### JSON status

```bash
repoman status neovim --json
```

---

## 9. Open (Path Lookup)

Print the filesystem path for any pristine or clone. Designed for shell
integration with `cd`:

```bash
# Open a pristine
cd $(repoman open neovim)

# Open a clone by suffix
cd $(repoman open feature-work)

# Open a clone by full directory name
cd $(repoman open neovim-feature-work)
```

The search order is: pristine names, then clone suffixes in metadata, then
full clone directory names.

---

## 10. Aliases

Create short names for repositories. Aliases resolve transparently in every
command.

### Create an alias

```bash
repoman alias neovim nv
```

### Use the alias everywhere

```bash
repoman status nv
repoman clone nv quick-test
repoman sync nv
repoman update nv
```

### List all aliases

```bash
repoman alias
```

Output:

```
Aliases:
  nv -> neovim
```

### Remove an alias

```bash
repoman alias neovim nv --remove
```

---

## 11. Update (Sync + Fast-Forward)

The `update` command is a convenience that syncs the pristine from the remote
and then fast-forwards all clones on their current branches.

### Update a single repo

```bash
repoman update neovim
```

Output:

```
Updating neovim...
Syncing pristine 'neovim'...
  Clone feature-work (main) fast-forwarded
  Clone bugfix (release-0.10) already up-to-date
```

### Update all repos

```bash
repoman update
```

Clones that have diverged from upstream will not be modified -- you will see a
message indicating that a manual merge is required.

---

## 12. Destroying Clones and Pristines

### Destroy a single clone

```bash
# By clone suffix
repoman destroy feature-work

# Or by full directory name
repoman destroy neovim-feature-work
```

### Destroy a pristine (keeps vault entry)

```bash
repoman destroy neovim
# Removes ~/.repoman/pristines/neovim/
# Vault entry remains -- you can re-init later
```

### Destroy all clones for a repo

```bash
repoman destroy --all-clones neovim
```

### Destroy all pristines across all repos

```bash
repoman destroy --all-pristines
# Vault entries are preserved
```

### Destroy stale clones by age

```bash
repoman destroy --stale 14
# Removes clones with HEAD commits older than 14 days
```

---

## 13. Removing a Repository

Fully unregister a repository: destroys all clones, pristine, metadata,
aliases, and the vault entry itself.

```bash
repoman remove neovim
```

This is the nuclear option. Use `destroy` if you only want to remove specific
clones or pristines while keeping the vault entry.

```bash
# Works with aliases too
repoman remove nv
```

---

## 14. Garbage Collection

Automated cleanup of stale clones and compaction of pristine object storage.

### Preview what would be cleaned

```bash
repoman gc --dry-run
```

### Run cleanup (default: 30-day threshold)

```bash
repoman gc
```

### Custom age threshold

```bash
repoman gc --days 7
```

GC does two things:
1. Finds and removes clones whose HEAD commit is older than the threshold.
2. Runs `git gc --auto` on every pristine bare repo.

---

## 15. Export and Import

Transfer your vault between machines or share it with teammates.

### Export vault to YAML

```bash
repoman export > my-repos.yaml
```

Output format:

```yaml
repositories:
- name: neovim
  url: https://github.com/neovim/neovim.git
  aliases:
  - nv
- name: ripgrep
  url: https://github.com/BurntSushi/ripgrep.git
```

### Import from YAML

```bash
repoman import my-repos.yaml
# Imported 2 repositories
```

Import skips repositories that already exist in the vault. Aliases from the
export file are restored automatically.

### Workflow: syncing vault across machines

```bash
# On machine A
repoman export > ~/Dropbox/repoman-vault.yaml

# On machine B
repoman import ~/Dropbox/repoman-vault.yaml
repoman init
# Now you have the same repos ready to go
```

---

## 16. Shell Completions

Repoman can generate shell completions for tab-completion of commands,
subcommands, and flags.

### Bash

```bash
repoman completions bash > ~/.local/share/bash-completion/completions/repoman
# Or system-wide:
repoman completions bash | sudo tee /etc/bash_completion.d/repoman > /dev/null
```

Then restart your shell or run:

```bash
source ~/.local/share/bash-completion/completions/repoman
```

### Zsh

```bash
repoman completions zsh > ~/.zfunc/_repoman
```

Make sure `~/.zfunc` is in your `fpath`. Add to `~/.zshrc` before
`compinit`:

```zsh
fpath=(~/.zfunc $fpath)
autoload -Uz compinit && compinit
```

### Fish

```bash
repoman completions fish > ~/.config/fish/completions/repoman.fish
```

### PowerShell

```powershell
repoman completions powershell > $PROFILE\..\Completions\repoman.ps1
```

### Elvish

```bash
repoman completions elvish > ~/.config/elvish/lib/repoman.elv
```

After installing completions, typing `repoman <TAB>` will show available
subcommands, and `repoman clone <TAB>` will show flag options.

---

## 17. Background Agent

The agent runs as a background process, periodically checking for new tags
and auto-syncing pristines from their remotes.

### Start the agent

```bash
repoman agent start
```

### Check agent status

```bash
repoman agent status
```

Output:

```
Agent is running (PID: 12345)
Log file: ~/.repoman/logs/agent.log
```

### Watch agent activity

```bash
tail -f ~/.repoman/logs/agent.log
```

### Stop the agent

```bash
repoman agent stop
```

### How the agent works

- On each cycle, the agent checks every repo's metadata for its
  `sync_interval` (default: 3600 seconds / 1 hour).
- If a repo is due for a sync, it fetches from the remote and checks for
  new semver tags.
- When a new tag is detected, the `post_sync_on_new_tag` hook fires (if
  configured).
- The agent also runs a periodic heartbeat that fast-forwards clones to
  match updated pristines.
- The agent sleeps dynamically -- until the next repo is due, rather than
  using a fixed poll interval.

---

## 18. Agent Heartbeat Configuration

The agent periodically checks whether clones can be fast-forwarded to match
their pristine. The default heartbeat interval is 300 seconds (5 minutes).
You can customize this in `config.yaml`:

```yaml
agent_heartbeat_interval: 120   # check every 2 minutes
```

Per-repo sync intervals are stored in metadata and default to 3600 seconds
(1 hour). You can override this in the `repos` section of `config.yaml`:

```yaml
repos:
  my-critical-app:
    sync_interval: 600   # sync every 10 minutes
```

---

## 19. TUI Dashboard

Repoman includes an interactive terminal UI built with ratatui. It shows a
live overview of all your repos, their pristine status, branches, clones,
and agent state.

### Launch the dashboard

```bash
repoman dashboard
```

### Navigation

| Key       | Action              |
|-----------|---------------------|
| `j` / Down  | Move down in repo list |
| `k` / Up    | Move up in repo list   |
| `q` / Esc   | Quit dashboard         |

### Layout

- **Left pane**: Repo list with pristine indicators (`+` initialized,
  `-` not initialized).
- **Right pane**: Detail view for the selected repo -- URL, branches,
  latest tag, last sync time, and clone list.
- **Bottom bar**: Summary stats -- total repos, total clones, agent status.

---

## 20. Lua Plugins

Repoman supports Lua plugins for extending lifecycle behavior. Plugins are
`.lua` files placed in the plugins directory (`~/.config/repoman/plugins/` by
default).

### Installing a plugin

Copy any `.lua` file into the plugins directory:

```bash
cp my-plugin.lua ~/.config/repoman/plugins/
```

Plugins are automatically loaded on startup.

### Writing a plugin

Plugins use the `repoman` global table to register callbacks:

```lua
-- ~/.config/repoman/plugins/notify.lua

-- Register a callback for the post_clone event
repoman.on("post_clone", function(ctx)
    repoman.log("info", "Clone created for " .. ctx.repo)
    repoman.exec("notify-send 'Repoman' 'Clone created: " .. ctx.repo .. "'")
end)

-- Register for new tag detection
repoman.on("post_sync_on_new_tag", function(ctx)
    repoman.log("info", "New tag " .. ctx.new_tag .. " for " .. ctx.repo)
end)
```

### Plugin API

| Function                              | Description                              |
|---------------------------------------|------------------------------------------|
| `repoman.on(event, callback)`         | Register a callback for a lifecycle event |
| `repoman.log(level, message)`         | Log a message (debug, info, warn, error) |
| `repoman.exec(command)`               | Run a shell command, returns stdout      |
| `repoman.vault.list()`                | Returns a table of all repo names        |
| `repoman.vault.info(name)`            | Returns `{name, url}` for a repo or nil  |

### Supported events

`post_init_pristine`, `pre_clone`, `post_clone`, `post_sync`,
`post_sync_on_new_tag`, `pre_destroy`, `post_destroy`

### Callback context

Every callback receives a `ctx` table with these fields:

| Field            | Description                              |
|------------------|------------------------------------------|
| `ctx.repo`       | Repository name                          |
| `ctx.event`      | Event name (e.g. `"post_clone"`)         |
| `ctx.pristine_path` | Path to the pristine (if applicable) |
| `ctx.clone_path` | Path to the clone (if applicable)        |
| `ctx.clone_name` | Clone suffix (if applicable)             |
| `ctx.new_tag`    | New tag string (for `post_sync_on_new_tag`) |

### Example: auto-build after clone

```lua
-- ~/.config/repoman/plugins/auto-build.lua

repoman.on("post_clone", function(ctx)
    if ctx.clone_path then
        repoman.log("info", "Running build in " .. ctx.clone_path)
        repoman.exec("cd " .. ctx.clone_path .. " && make build")
    end
end)
```

---

## 21. Configuration Management

Repoman provides subcommands for inspecting and managing your configuration.

### Show effective config

```bash
repoman config
# or
repoman config show
```

### Print config file path

```bash
repoman config path
# Output: /home/user/.config/repoman/config.yaml
```

Useful for opening in your editor:

```bash
$EDITOR $(repoman config path)
```

### Validate config

```bash
repoman config validate
# Output: OK Configuration is valid
```

### Create default config

```bash
repoman config init
# Creates ~/.config/repoman/config.yaml with defaults
```

---

## 22. Health Checks (Doctor)

Run diagnostic checks on your repoman installation:

```bash
repoman doctor
```

Checks include:
- Config file existence and validity
- Data directory existence (vault, pristines, clones, plugins, logs)
- Vault integrity (metadata for each repo)
- Pristine validity (valid bare git repos)
- Clone health (directory exists, alternates point to valid paths)
- SSH agent availability
- Background agent status

Example output:

```
Repoman Health Check

  OK Config file: /home/user/.config/repoman/config.yaml
  OK vault_dir: /home/user/.repoman/vault
  OK pristines_dir: /home/user/.repoman/pristines
  OK clones_dir: /home/user/.repoman/clones
  OK plugins_dir: /home/user/.config/repoman/plugins
  OK logs_dir: /home/user/.repoman/logs
  OK 3 repositories in vault
  OK SSH agent available
  INFO Agent not running

  3 repos, 5 clones, 0 issues
  Everything looks good!
```

---

## 23. Renaming Repositories

Rename a vault entry and all associated data (metadata, pristine directory, aliases):

```bash
repoman rename old-name new-name
```

This updates the vault entry, moves metadata, renames the pristine directory,
and retargets any aliases that pointed to the old name.

```bash
# Example with alias
repoman alias my-repo mr
repoman rename mr better-name
# Alias 'mr' now points to 'better-name'
```

---

## 24. Man Pages

Generate a man page for repoman:

```bash
repoman man > /usr/local/share/man/man1/repoman.1
man repoman
```

Or view directly:

```bash
repoman man | man -l -
```

---

## 25. Configuration File Reference

Repoman reads its config from `~/.config/repoman/config.yaml`. All fields
are optional -- Repoman uses sensible defaults if no config file exists.

### Full example

```yaml
# Directory paths (tilde expansion supported)
vault_dir: ~/.repoman/vault
pristines_dir: ~/.repoman/pristines
clones_dir: ~/.repoman/clones
plugins_dir: ~/.config/repoman/plugins
logs_dir: ~/.repoman/logs

# Agent heartbeat interval in seconds (default: 300)
agent_heartbeat_interval: 300

# Default JSON output for list/status (default: false)
json_output: false

# Per-repo overrides
repos:
  my-web-app:
    hooks:
      post_init_pristine: "echo 'Pristine ready for my-web-app'"
      pre_clone: "echo 'About to create clone...'"
      post_clone: "npm ci && npm run build"
      post_sync: "./scripts/notify-team.sh"
      post_sync_on_new_tag: "./scripts/deploy-staging.sh"
      pre_destroy: "echo 'Cleaning up...'"
      post_destroy: "echo 'Clone removed'"
    build:
      command: "npm run build"
      pre_build: "npm ci"
      post_build: "npm test"
      working_dir: "frontend/"
    auth:
      ssh_key_path: "~/.ssh/deploy_key"
      token_env_var: "GITHUB_TOKEN"
    sync_interval: 1800
    default_branch: main
    auto_init: true
    clone_defaults:
      branch: develop
      shallow: false
    tags:
      - javascript
      - frontend

  my-rust-lib:
    hooks:
      post_clone: "cargo build --release"
      post_sync: "cargo test"
    sync_interval: 7200
    auth:
      ssh_key_path: "~/.ssh/id_ed25519"

  infra-repo:
    hooks:
      post_sync_on_new_tag: |
        echo "New release: $REPOMAN_NEW_TAG"
        ./deploy.sh --version "$REPOMAN_NEW_TAG"
```

### Hook environment variables

All hooks run via `sh -c "..."` with these environment variables set:

| Variable                | Always set | Description                    |
|-------------------------|------------|--------------------------------|
| `REPOMAN_REPO`          | Yes        | Repository name                |
| `REPOMAN_EVENT`         | Yes        | Hook event name                |
| `REPOMAN_PRISTINE_PATH` | When available | Path to pristine directory |
| `REPOMAN_CLONE_PATH`    | Clone hooks only | Path to clone directory   |
| `REPOMAN_CLONE_NAME`    | Clone hooks only | Clone suffix name         |
| `REPOMAN_NEW_TAG`       | Tag hooks only | New tag string              |

### Hook failure behavior

| Hook                  | On failure              |
|-----------------------|-------------------------|
| `post_init_pristine`  | Init fails (fatal)      |
| `pre_clone`           | Clone aborted (fatal)   |
| `post_clone`          | Clone fails (fatal)     |
| `post_sync`           | Logged, sync still succeeds |
| `post_sync_on_new_tag`| Logged, non-fatal       |
| `pre_destroy`         | Logged, non-fatal       |
| `post_destroy`        | Logged, non-fatal       |

---

## 26. MCP Server (LLM Agent Integration)

Repoman includes an MCP (Model Context Protocol) server that allows LLM
agents like Claude Code to manage repositories programmatically.

### Quick test

```bash
# Send an initialize request and see the response
echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test"}}}' | repoman mcp
```

### Setting up with Claude Code

Add to your project's `.mcp.json`:

```json
{
  "mcpServers": {
    "repoman": {
      "command": "repoman",
      "args": ["mcp"]
    }
  }
}
```

Or add globally via Claude Code settings:

```bash
claude mcp add repoman -- repoman mcp
```

### Available tools (13)

| Tool | Description |
|------|-------------|
| `vault_list` | List all repositories with status |
| `vault_add` | Add a repository by URL |
| `vault_remove` | Remove a repository and all data |
| `clone_create` | Create a clone from a pristine |
| `clone_destroy` | Destroy a clone |
| `sync` | Sync pristine(s) from remote |
| `status` | Detailed status for a repository |
| `open` | Get filesystem path for a pristine or clone |
| `update` | Sync + fast-forward all clones |
| `gc` | Garbage-collect stale clones |
| `agent_status` | Check background agent status |
| `export` | Export vault to YAML |
| `import` | Import repositories from YAML string |

### Available resources (4)

| URI | Description |
|-----|-------------|
| `vault://state` | Full vault contents |
| `vault://config` | Effective configuration |
| `vault://repo/{name}/metadata` | Per-repo metadata |
| `vault://repo/{name}/clones` | Clone list for a repo |

### Manual JSON-RPC testing

```bash
# List tools
echo '{"jsonrpc":"2.0","id":2,"method":"tools/list"}' | repoman mcp

# List repos
echo '{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"vault_list","arguments":{}}}' | repoman mcp

# Read vault state resource
echo '{"jsonrpc":"2.0","id":4,"method":"resources/read","params":{"uri":"vault://state"}}' | repoman mcp
```

---

## Complete Workflow Example

```bash
# 1. Add some repos
repoman add https://github.com/neovim/neovim.git
repoman add https://github.com/BurntSushi/ripgrep.git

# 2. Create aliases for convenience
repoman alias neovim nv
repoman alias ripgrep rg

# 3. Initialize pristines
repoman init

# 4. Check what we have
repoman list
repoman list --json | jq '.[].name'

# 5. Get detailed status
repoman status nv

# 6. Create workspaces
repoman clone nv bugfix -b release-0.10
repoman clone rg experiment

# 7. Work on the neovim clone
cd $(repoman open bugfix)
git checkout -b fix-issue-123
# ... make changes ...
git commit -am "Fix issue #123"
git push origin fix-issue-123

# 8. Update everything from upstream
repoman update

# 9. Clean up finished workspaces
repoman destroy bugfix

# 10. Export vault for backup
repoman export > ~/vault-backup.yaml

# 11. Garbage-collect old clones
repoman gc --days 14

# 12. Start the agent for background syncing
repoman agent start
repoman agent status

# 13. Launch the dashboard to see everything at a glance
repoman dashboard

# 14. When completely done with a repo
repoman remove rg
```

---

## Directory Structure

After using Repoman, `~/.repoman/` looks like this:

```
~/.repoman/
+-- vault/
|   +-- vault.json              # Master list of all vaulted repos + aliases
|   +-- neovim/
|   |   +-- metadata.json       # Per-repo metadata: clones, sync history, tags
|   +-- ripgrep/
|       +-- metadata.json
+-- pristines/
|   +-- neovim/                 # Bare git repo (reference clone)
|   +-- ripgrep/
+-- clones/
|   +-- neovim-bugfix/          # Working copy (uses git alternates)
|   +-- ripgrep-experiment/
+-- logs/
    +-- repoman.log             # Debug log (always written)
    +-- agent.log               # Agent stdout/stderr
    +-- agent.pid               # Agent PID file (when running)
```

Configuration and plugins live at:

```
~/.config/repoman/
+-- config.yaml
+-- plugins/
    +-- notify.lua              # Lua plugins (auto-loaded)
    +-- auto-build.lua
```

---

## Troubleshooting

### "Authentication failed" error

Your SSH key may not be loaded in the agent:

```bash
ssh-add ~/.ssh/id_ed25519
```

For persistent key management, consider using `keychain`:

```bash
eval $(keychain --eval --quiet id_ed25519)
```

For HTTPS repos, configure a credential helper:

```bash
git config --global credential.helper cache
```

See [ssh-authentication.md](ssh-authentication.md) for more options.

### "Pristine not found" error

Initialize it first, or use the lazy-init shortcut:

```bash
repoman init my-repo
# Or just clone directly (auto-inits):
repoman clone my-repo workspace
```

### "Repository already exists in vault"

It is already registered. Check with:

```bash
repoman list
```

### Agent won't start

Check if it is already running:

```bash
repoman agent status
```

Check the agent log for errors:

```bash
cat ~/.repoman/logs/agent.log
```

If the agent crashed and left a stale PID file, Repoman will detect that the
process is no longer running and clean up automatically.

### Debug mode

Enable verbose console logging for any command:

```bash
repoman --debug sync neovim
```

Debug logs are always written to `~/.repoman/logs/repoman.log` regardless
of this flag.
