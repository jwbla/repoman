# Configuration

Repoman configuration is optional. All settings have sensible defaults. If you need to customize paths, define lifecycle hooks, or set per-repo options, create:

```
~/.config/repoman/config.yaml
```

## Full Reference

```yaml
# Directory paths (all support ~ expansion)
vault_dir: ~/.repoman/vault
pristines_dir: ~/.repoman/pristines
clones_dir: ~/.repoman/clones
plugins_dir: ~/.config/repoman/plugins
logs_dir: ~/.repoman/logs

# Agent heartbeat interval in seconds (how often the agent checks clones)
agent_heartbeat_interval: 300

# Default JSON output for list and status commands
json_output: false

# Per-repo configuration (key = repo name as shown in repoman list)
repos:
  my-app:
    hooks:
      post_init_pristine: "echo pristine ready"
      pre_clone: "echo about to clone"
      post_clone: "npm ci && npm run build"
      post_sync: "./scripts/deploy.sh"
      post_sync_on_new_tag: "./scripts/release.sh"
      pre_destroy: "./scripts/backup-state.sh"
      post_destroy: "echo clone removed"
    build:
      command: "npm run build"
      pre_build: "npm ci"
      post_build: "npm test"
      working_dir: "."
    auth:
      ssh_key_path: "~/.ssh/id_ed25519_work"
      token_env_var: "GITHUB_TOKEN"
    clone_defaults:
      branch: "develop"
      shallow: false
    tags:
      - "javascript"
      - "frontend"
    sync_interval: 1800
    default_branch: "main"
    auto_init: true
```

## Top-Level Keys

### vault_dir

**Default:** `~/.repoman/vault`

Directory where `vault.json` and per-repo metadata directories are stored.

### pristines_dir

**Default:** `~/.repoman/pristines`

Directory where bare reference clones are created.

### clones_dir

**Default:** `~/.repoman/clones`

Directory where working copy clones are created.

### plugins_dir

**Default:** `~/.config/repoman/plugins`

Directory where Lua plugin scripts (`.lua` files) are loaded from. See [Plugins](plugins.md).

### logs_dir

**Default:** `~/.repoman/logs`

Directory for log files: `repoman.log` (main debug log), `agent.log` (agent output), and `agent.pid` (agent PID file).

### agent_heartbeat_interval

**Type:** Integer (seconds)
**Default:** `300` (5 minutes)

How often the background agent checks clones for upstream updates and attempts fast-forward or rebase.

### json_output

**Type:** Boolean
**Default:** `false`

When `true`, `repoman list` and `repoman status` default to JSON output without needing the `--json` flag.

## Per-Repo Configuration (repos)

The `repos` map is keyed by repository name (as shown in `repoman list`). Each entry can contain the following sections.

### hooks

Shell commands to run at lifecycle events. See [Hooks](hooks.md) for full details.

| Key | When it runs | Fail behavior |
|-----|-------------|---------------|
| `post_init_pristine` | After pristine is created | Fatal (init fails) |
| `pre_clone` | Before clone is created | Fatal (clone fails) |
| `post_clone` | After clone is created | Fatal (clone fails) |
| `post_sync` | After pristine is synced | Non-fatal (warning) |
| `post_sync_on_new_tag` | After sync when agent detects a new tag | Non-fatal (warning) |
| `pre_destroy` | Before clone is removed | Non-fatal (warning) |
| `post_destroy` | After clone is removed | Non-fatal (warning) |

### build

Build commands associated with the repository. These are stored in metadata for reference.

| Key | Description |
|-----|-------------|
| `command` | Main build command |
| `pre_build` | Command to run before the build |
| `post_build` | Command to run after the build |
| `working_dir` | Working directory for build commands (relative to clone root) |

### auth

Per-repo authentication overrides.

| Key | Description |
|-----|-------------|
| `ssh_key_path` | Path to an SSH private key file (overrides ssh-agent) |
| `token_env_var` | Name of an environment variable containing an auth token (used for HTTPS) |

The credential callback tries methods in order: (1) explicit SSH key, (2) ssh-agent, (3) git default credentials, (4) token from env var, (5) git credential helper.

### clone_defaults

Default settings applied when creating new clones.

| Key | Description |
|-----|-------------|
| `branch` | Default branch to check out (overrides pristine HEAD) |
| `shallow` | Whether to create shallow clones (boolean) |

### tags

**Type:** List of strings

Arbitrary tags for categorizing repos. Currently stored in metadata for organizational purposes.

### sync_interval

**Type:** Integer (seconds)
**Default:** `3600` (1 hour, set in metadata)

How often the background agent should sync this repo. Lower values mean more frequent syncing.

### default_branch

**Type:** String

The default branch name for this repository. Stored in metadata for reference.

### auto_init

**Type:** Boolean

When `true`, the agent or other automated processes should automatically initialize the pristine after adding the repo to the vault.

## Path Expansion

All directory paths in config support `~` expansion (e.g., `~/custom/path` expands to your home directory). Absolute paths are used as-is.

## No Config Needed

If `~/.config/repoman/config.yaml` does not exist, repoman uses all defaults. The data directory (`~/.repoman/`) and its subdirectories are created automatically on first run.
