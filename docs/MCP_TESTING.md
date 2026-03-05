# Testing the Repoman MCP Server

This guide covers how to set up and test `repoman mcp` with Claude Code and
Cursor.

---

## Quick Smoke Test

Before configuring any editor, verify the server works from your terminal:

```bash
# Build and install
cargo install --path .

# Send an initialize handshake
echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test"}}}' | repoman mcp
```

You should get back a JSON response containing `"protocolVersion":"2024-11-05"`
and `"serverInfo":{"name":"repoman"}`. If you see that, the server is working.

### More manual tests

```bash
# List all 13 tools
echo '{"jsonrpc":"2.0","id":2,"method":"tools/list"}' | repoman mcp

# Call vault_list
echo '{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"vault_list","arguments":{}}}' | repoman mcp

# Read vault state resource
echo '{"jsonrpc":"2.0","id":4,"method":"resources/read","params":{"uri":"vault://state"}}' | repoman mcp

# Check agent status
echo '{"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"name":"agent_status","arguments":{}}}' | repoman mcp
```

> **Tip:** Pipe through `python3 -m json.tool` for pretty-printed output.

---

## Claude Code Setup

### Option A: CLI command (recommended)

```bash
claude mcp add repoman -- repoman mcp
```

This adds repoman as a user-scope MCP server. To scope it to a specific
project instead:

```bash
claude mcp add --scope project repoman -- repoman mcp
```

### Option B: Project config file

Create `.mcp.json` in your project root:

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

Claude Code will prompt for approval the first time it encounters a
project-scope MCP server.

### Option C: Full path (if repoman isn't in PATH)

```bash
claude mcp add repoman -- /home/youruser/.cargo/bin/repoman mcp
```

Or in `.mcp.json`:

```json
{
  "mcpServers": {
    "repoman": {
      "command": "/home/youruser/.cargo/bin/repoman",
      "args": ["mcp"]
    }
  }
}
```

### Verifying in Claude Code

```bash
# List configured MCP servers
claude mcp list

# Check details for repoman
claude mcp get repoman
```

Once configured, start a Claude Code session. The repoman tools will appear
automatically. Try asking Claude:

- "List my repoman repositories"
- "What's the status of the neovim repo?"
- "Clone ripgrep for me"
- "Is the repoman agent running?"

### Removing

```bash
claude mcp remove repoman
```

---

## Cursor Setup

### Option A: Project config (recommended for teams)

Create `.cursor/mcp.json` in your project root:

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

### Option B: Global config (all projects)

Edit `~/.cursor/mcp.json`:

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

### Option C: Cursor UI

1. Open Cursor Settings
2. Go to **Features > MCP**
3. Click **+ Add New MCP Server**
4. Set name to `repoman`, command to `repoman`, args to `mcp`

### Verifying in Cursor

After adding the config, **restart Cursor**. The repoman tools should appear
in Cursor's MCP tool list. You can verify by asking the agent to list your
repositories or check a repo's status.

> **Note:** If `repoman` isn't on your PATH, use the full path to the binary
> (e.g., `/home/youruser/.cargo/bin/repoman`).

---

## Available Tools

Once connected, the MCP server exposes these 13 tools:

| Tool | What it does |
|------|-------------|
| `vault_list` | List all repositories with status |
| `vault_add` | Add a repository by URL |
| `vault_remove` | Remove a repository and all data |
| `clone_create` | Create a working clone from a pristine |
| `clone_destroy` | Destroy a clone |
| `sync` | Sync pristine(s) from remote |
| `status` | Detailed repo status (branches, clones, dirty state) |
| `open` | Get filesystem path for a pristine or clone |
| `update` | Sync pristine + fast-forward all clones |
| `gc` | Garbage-collect stale clones |
| `agent_status` | Check if the background agent is running |
| `export` | Export vault to YAML |
| `import` | Import repositories from YAML string |

## Available Resources

| URI | Description |
|-----|-------------|
| `vault://state` | Full vault (all entries and aliases) |
| `vault://config` | Effective repoman configuration |
| `vault://repo/{name}/metadata` | Per-repo metadata |
| `vault://repo/{name}/clones` | Clone list for a repo |

---

## Troubleshooting

### "command not found" or "Connection closed"

Make sure `repoman` is installed and on your PATH:

```bash
which repoman
repoman --version
```

If it's not on PATH, use the full path in your config.

### No output / hangs

The MCP server reads from stdin line by line. If you're testing manually,
make sure you send a complete JSON line followed by a newline. The server
exits when stdin closes (EOF).

### Debug logging

All MCP server activity is logged to `~/.repoman/logs/repoman.log`. To see
debug output on stderr as well:

```bash
repoman --debug mcp
```

### Windows (WSL)

If running in WSL, the server works as-is. On native Windows, stdio
transport should work but is untested. If you hit issues with Cursor on
Windows, try wrapping the command:

```json
{
  "mcpServers": {
    "repoman": {
      "command": "cmd",
      "args": ["/c", "repoman", "mcp"]
    }
  }
}
```
