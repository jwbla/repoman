# Repoman v0.3.6 User Manual

Repoman is a git repository manager built around disposable workspaces. It maintains a vault of repository URLs, creates space-efficient bare reference clones (pristines), and lets you spin up and tear down lightweight working copies (clones) instantly.

## Table of Contents

### Getting Started

- [Installation, First Run, and Quick Example](getting-started.md)

### Commands

| Command | Description |
|---------|-------------|
| [add](commands/add.md) | Add a repository to the vault |
| [init](commands/init.md) | Create pristine bare clone(s) |
| [clone](commands/clone.md) | Create a working copy from a pristine |
| [sync](commands/sync.md) | Fetch latest changes into pristine(s) |
| [update](commands/update.md) | Sync pristine and fast-forward all clones |
| [status](commands/status.md) | Show detailed repository status |
| [list](commands/list.md) | List all repositories |
| [open](commands/open.md) | Print filesystem path for a target |
| [alias](commands/alias.md) | Manage short names for repositories |
| [rename](commands/rename.md) | Rename a vault entry |
| [destroy](commands/destroy.md) | Remove clones or pristines from disk |
| [remove](commands/remove.md) | Fully unregister a repository |
| [gc](commands/gc.md) | Garbage-collect stale clones and repack |
| [refresh](commands/refresh.md) | Init missing pristines and sync existing ones |
| [agent](commands/agent.md) | Background sync agent |
| [config](commands/config.md) | View and manage configuration |
| [doctor](commands/doctor.md) | Run health checks |
| [completions](commands/completions.md) | Generate shell completions |
| [shell-init](commands/shell-init.md) | Shell completions + `cd` wrapper |
| [export / import](commands/export-import.md) | Export and import vault data |
| [dashboard](commands/dashboard.md) | Interactive TUI dashboard |
| [mcp](commands/mcp.md) | MCP server for LLM agent integration |
| man | Generate man page (stdout) |

### Reference

- [Configuration](configuration.md) -- Full `config.yaml` reference
- [Lifecycle Hooks](hooks.md) -- Shell hooks at key events
- [Lua Plugins](plugins.md) -- Plugin development guide
- [Architecture](architecture.md) -- Data flow and internals

## Global Flags

These flags apply to all commands:

| Flag | Description |
|------|-------------|
| `--debug` | Print debug-level logs to the console (always written to log file) |
| `--json` | Output in JSON format (applies to `list` and `status`) |
| `-y` / `--yes` | Skip confirmation prompts for destructive commands |
| `--version` | Print version and exit |
| `--help` | Print help and exit |
