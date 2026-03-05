# repoman mcp

Start an MCP (Model Context Protocol) server for LLM agent integration.

## Synopsis

```
repoman mcp
```

## Description

Launches an MCP server on stdin/stdout that exposes repoman operations as tools for LLM agents (Claude, etc.). The server implements the JSON-RPC-based MCP protocol and provides tools for vault management, cloning, syncing, and querying repository status.

This is intended for integration with AI coding assistants and automation pipelines, not for direct human use.

## Available Tools

| Tool | Description |
|------|-------------|
| `vault_list` | List all repositories in the vault |
| `vault_add` | Add a repository URL to the vault |
| `vault_remove` | Remove a repository from the vault |
| `clone_create` | Create a working copy from a pristine |
| `clone_destroy` | Destroy a clone |
| `sync` | Sync a pristine from its remote |
| `status` | Get detailed status for a repository |
| `open` | Get the filesystem path for a target |
| `update` | Sync and fast-forward clones |
| `gc` | Garbage-collect stale clones |
| `agent_status` | Check background agent status |
| `export` | Export vault to YAML |
| `import` | Import repositories from YAML |

## Examples

Configure in an MCP client (e.g., Claude Code):

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

## Tips

- The MCP server communicates via stdin/stdout using JSON-RPC. It is designed to be launched as a subprocess by an MCP client.
- See `docs/MCP_IDEAS.md` for the full design specification.
