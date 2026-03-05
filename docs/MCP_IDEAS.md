# MCP Ideas: Making Repoman Accessible to LLM Agents

The Model Context Protocol (MCP) is a standard for exposing tools and
resources to LLM agents over JSON-RPC (typically via stdio). Adding an MCP
server to Repoman would allow AI agents to manage git workspaces
programmatically -- creating clones, syncing repos, inspecting status, and
orchestrating multi-repo workflows without human intervention.

This document explores what that integration would look like.

---

## Table of Contents

- [MCP Server Architecture](#mcp-server-architecture)
- [Tools](#tools)
- [Resources](#resources)
- [LLM Agent Use Cases](#llm-agent-use-cases)
- [MCP Tool Definitions](#mcp-tool-definitions)
- [Integration Patterns](#integration-patterns)
- [Implementation Roadmap](#implementation-roadmap)

---

## MCP Server Architecture

The MCP server would run as a subprocess launched by the MCP client (e.g.
Claude Code, Cursor, or a custom agent). Communication happens over stdio
using JSON-RPC 2.0.

```
+-----------------+       stdio (JSON-RPC)       +------------------+
|  LLM Agent      | <-------------------------> |  repoman-mcp     |
|  (Claude Code,  |                              |  (MCP server)    |
|   custom agent) |                              |                  |
+-----------------+                              +--+---------------+
                                                    |
                                                    | calls into
                                                    v
                                              +-----+--------+
                                              | repoman core |
                                              | (lib crate)  |
                                              +--------------+
                                                    |
                                    +---------------+---------------+
                                    |               |               |
                                    v               v               v
                              vault.json     pristines/       clones/
```

### Key design decisions

1. **Separate binary**: `repoman-mcp` would be a thin binary in the same
   workspace that links against repoman's core library. This avoids
   duplicating business logic.

2. **Stateless per-request**: Each tool invocation is a self-contained
   operation. The MCP server loads config and vault from disk on each call,
   just like the CLI does.

3. **No long-running state**: The MCP server does not manage the agent. The
   agent continues to run independently. The MCP server can query agent
   status and start/stop it.

4. **Auth passthrough**: SSH keys and tokens from the user's environment are
   available to the MCP server because it runs as a child process of the
   agent host (e.g. Claude Code).

---

## Tools

MCP tools are callable functions that the LLM agent can invoke. Each tool
maps to one or more Repoman operations.

### vault_list

List all repositories in the vault with their status.

```
Input:  { verbose?: boolean }
Output: { repos: [{ name, url, has_pristine, clone_count, last_sync, aliases }] }
```

### vault_add

Add a repository URL to the vault.

```
Input:  { url: string }
Output: { name: string, url: string }
```

### vault_remove

Remove a repository from the vault and delete all its data.

```
Input:  { name: string }
Output: { removed: string, clones_destroyed: number, aliases_removed: string[] }
```

### clone_create

Create a new working copy from a pristine. Supports lazy init.

```
Input:  { repo: string, name?: string, branch?: string }
Output: { clone_name: string, path: string, branch: string }
```

### clone_destroy

Destroy a clone by name or suffix.

```
Input:  { target: string }
Output: { destroyed: string, path: string }
```

### sync

Sync one or all pristines from their remotes.

```
Input:  { repo?: string }
Output: { synced: string[], new_tags: { repo: string, tag: string }[] }
```

### status

Get detailed status for a repository.

```
Input:  { name: string }
Output: { name, url, pristine_exists, branches, clones: [{ name, branch, dirty_files, ahead, behind }], latest_tag, last_sync }
```

### open

Resolve a target name to a filesystem path.

```
Input:  { target: string }
Output: { path: string }
```

### update

Sync pristine and fast-forward all clones.

```
Input:  { repo?: string }
Output: { updated: string[], fast_forwarded: string[], diverged: string[] }
```

### gc

Run garbage collection.

```
Input:  { days?: number, dry_run?: boolean }
Output: { stale_clones: [{ repo, clone, days_old }], pristines_compacted: number }
```

### agent_status

Check the background agent's status.

```
Input:  {}
Output: { running: boolean, pid?: number, log_path: string }
```

### export

Export the vault to YAML.

```
Input:  {}
Output: { yaml: string }
```

### import

Import repositories from a YAML string.

```
Input:  { yaml: string }
Output: { imported: number, skipped: string[] }
```

---

## Resources

MCP resources are read-only data sources that the LLM can access for context
without invoking a tool.

### vault://state

The full vault state: all repos, aliases, and entry metadata.

```json
{
  "uri": "vault://state",
  "name": "Vault State",
  "mimeType": "application/json",
  "description": "Complete vault contents including all repos and aliases"
}
```

### vault://repo/{name}/metadata

Per-repo metadata including clone list, sync history, branches, and tags.

```json
{
  "uri": "vault://repo/neovim/metadata",
  "name": "neovim metadata",
  "mimeType": "application/json"
}
```

### vault://repo/{name}/clones

List of active clones with their paths and branch state.

```json
{
  "uri": "vault://repo/neovim/clones",
  "name": "neovim clones",
  "mimeType": "application/json"
}
```

### vault://config

The current Repoman configuration (paths, hooks, per-repo settings).

```json
{
  "uri": "vault://config",
  "name": "Repoman Configuration",
  "mimeType": "application/json"
}
```

---

## LLM Agent Use Cases

### Parallel Development Workspaces

An LLM agent working on a complex task can create multiple isolated clones
to explore different approaches simultaneously, then destroy the ones that
did not work out.

```
Agent: "I need to try three approaches to fix this bug."
  -> clone_create { repo: "my-app", name: "approach-1" }
  -> clone_create { repo: "my-app", name: "approach-2" }
  -> clone_create { repo: "my-app", name: "approach-3" }
  -> [work in each clone, evaluate results]
  -> clone_destroy { target: "approach-2" }
  -> clone_destroy { target: "approach-3" }
  -> "Approach 1 succeeded. Clone at /home/user/.repoman/clones/my-app-approach-1/"
```

### Automated Code Review

An LLM agent reviewing a PR can create a fresh clone, check out the PR
branch, run tests, inspect diffs, and then destroy the clone.

```
Agent: "Review PR #42 on my-app"
  -> clone_create { repo: "my-app", name: "review-pr-42", branch: "feature/new-api" }
  -> open { target: "review-pr-42" }
  -> [read files, run tests in the clone]
  -> clone_destroy { target: "review-pr-42" }
  -> "Review complete. 3 issues found."
```

### Dependency Auditing

An agent can clone multiple repos, inspect their dependency files, and
cross-reference versions.

```
Agent: "Audit all repos for outdated dependencies"
  -> vault_list {}
  -> [for each repo: clone_create, read package.json/Cargo.toml, clone_destroy]
  -> "Found 7 repos with outdated dependencies. Details: ..."
```

### Automated Migration

An agent performing a codebase-wide migration (e.g. upgrading a framework
version) can clone each affected repo, apply the migration, run tests, and
commit.

```
Agent: "Upgrade React 18 -> 19 across all frontend repos"
  -> vault_list {}
  -> [filter repos with React dependency]
  -> [for each: clone_create, modify files, run tests, commit, clone_destroy]
  -> "Migration complete. 4/5 repos succeeded. my-legacy-app failed tests."
```

### Multi-Repo Refactoring

When a change spans multiple repositories (e.g. updating a shared API
contract), an agent can create coordinated clones and ensure consistency.

```
Agent: "Rename UserService to AccountService across api-gateway and user-service repos"
  -> clone_create { repo: "api-gateway", name: "rename-refactor" }
  -> clone_create { repo: "user-service", name: "rename-refactor" }
  -> [apply changes in both clones, verify consistency]
  -> "Refactoring complete. Both repos updated consistently."
```

---

## MCP Tool Definitions

JSON Schema definitions for each tool, suitable for inclusion in an MCP
server manifest.

### vault_list

```json
{
  "name": "vault_list",
  "description": "List all repositories in the Repoman vault with their current status (pristine existence, clone count, last sync time, aliases).",
  "inputSchema": {
    "type": "object",
    "properties": {
      "verbose": {
        "type": "boolean",
        "description": "Include detailed clone and branch information",
        "default": false
      }
    }
  }
}
```

### vault_add

```json
{
  "name": "vault_add",
  "description": "Add a git repository URL to the Repoman vault. Does not clone the repo -- use clone_create for that.",
  "inputSchema": {
    "type": "object",
    "properties": {
      "url": {
        "type": "string",
        "description": "Git URL (HTTPS or SSH format)"
      }
    },
    "required": ["url"]
  }
}
```

### vault_remove

```json
{
  "name": "vault_remove",
  "description": "Remove a repository from the vault and delete ALL associated data (pristine, clones, metadata, aliases). This is destructive and irreversible.",
  "inputSchema": {
    "type": "object",
    "properties": {
      "name": {
        "type": "string",
        "description": "Repository name or alias"
      }
    },
    "required": ["name"]
  }
}
```

### clone_create

```json
{
  "name": "clone_create",
  "description": "Create a lightweight working copy from a pristine. If the pristine does not exist, it will be automatically initialized from the vault (lazy init). Returns the filesystem path of the new clone.",
  "inputSchema": {
    "type": "object",
    "properties": {
      "repo": {
        "type": "string",
        "description": "Repository name or alias"
      },
      "name": {
        "type": "string",
        "description": "Optional name for the clone suffix. If omitted, a random 6-character suffix is generated."
      },
      "branch": {
        "type": "string",
        "description": "Branch to check out. Defaults to the repo's HEAD branch."
      }
    },
    "required": ["repo"]
  }
}
```

### clone_destroy

```json
{
  "name": "clone_destroy",
  "description": "Destroy a clone by its suffix name or full directory name. The pristine and vault entry are preserved.",
  "inputSchema": {
    "type": "object",
    "properties": {
      "target": {
        "type": "string",
        "description": "Clone suffix, full clone directory name, or pristine name"
      }
    },
    "required": ["target"]
  }
}
```

### sync

```json
{
  "name": "sync",
  "description": "Fetch the latest changes from remote origins into pristine(s). If repo is specified, syncs only that repo. If omitted, syncs all repos with initialized pristines.",
  "inputSchema": {
    "type": "object",
    "properties": {
      "repo": {
        "type": "string",
        "description": "Repository name or alias. Omit to sync all."
      }
    }
  }
}
```

### status

```json
{
  "name": "status",
  "description": "Get detailed status for a repository: pristine state, branches, clone details (branch, dirty files, ahead/behind), latest tag, last sync, and alternates health.",
  "inputSchema": {
    "type": "object",
    "properties": {
      "name": {
        "type": "string",
        "description": "Repository name or alias"
      }
    },
    "required": ["name"]
  }
}
```

### open

```json
{
  "name": "open",
  "description": "Resolve a target name (pristine name, clone suffix, or full clone directory name) to its absolute filesystem path.",
  "inputSchema": {
    "type": "object",
    "properties": {
      "target": {
        "type": "string",
        "description": "Pristine name, clone suffix, or full clone directory name"
      }
    },
    "required": ["target"]
  }
}
```

### update

```json
{
  "name": "update",
  "description": "Sync a pristine from its remote and then fast-forward all of its clones. Clones that have diverged are skipped with a warning.",
  "inputSchema": {
    "type": "object",
    "properties": {
      "repo": {
        "type": "string",
        "description": "Repository name or alias. Omit to update all."
      }
    }
  }
}
```

### gc

```json
{
  "name": "gc",
  "description": "Garbage-collect stale clones (HEAD commit older than threshold) and run git gc --auto on pristines.",
  "inputSchema": {
    "type": "object",
    "properties": {
      "days": {
        "type": "integer",
        "description": "Age threshold in days for stale clone detection",
        "default": 30
      },
      "dry_run": {
        "type": "boolean",
        "description": "Preview what would be cleaned without making changes",
        "default": false
      }
    }
  }
}
```

### agent_status

```json
{
  "name": "agent_status",
  "description": "Check whether the Repoman background agent is running.",
  "inputSchema": {
    "type": "object",
    "properties": {}
  }
}
```

### export

```json
{
  "name": "export",
  "description": "Export the entire vault (repos, URLs, aliases) as a YAML string for backup or transfer.",
  "inputSchema": {
    "type": "object",
    "properties": {}
  }
}
```

---

## Integration Patterns

### Claude Code Integration

The most natural integration point. Users would add `repoman-mcp` to their
Claude Code MCP server config:

```json
{
  "mcpServers": {
    "repoman": {
      "command": "repoman-mcp",
      "args": [],
      "env": {}
    }
  }
}
```

Claude Code would then be able to:
- Create disposable workspaces for each task
- Spin up multiple clones for parallel exploration
- Clean up after itself when a task is complete
- Query repo status before deciding on an approach

### Agentic CI

A CI pipeline could use Repoman via MCP to manage workspaces for parallel
test execution:

```
1. Agent receives "test PR #42 against 3 target branches"
2. clone_create { repo: "my-app", name: "test-main",    branch: "main" }
3. clone_create { repo: "my-app", name: "test-release",  branch: "release" }
4. clone_create { repo: "my-app", name: "test-develop",  branch: "develop" }
5. [cherry-pick PR commits into each clone, run tests in parallel]
6. [report results]
7. clone_destroy for all three clones
```

### Self-Healing Repos

An agent monitoring repo health can detect and fix common issues:

```
1. status { name: "my-app" }
2. [detect: alternates_ok = false, some clones have broken alternates]
3. [for each broken clone: destroy and recreate]
4. "Repaired 2 clones with broken alternates for my-app"
```

### Multi-Repo Coordination

An agent managing a microservices architecture can use Repoman to keep all
service repos in sync and detect version drift:

```
1. vault_list { verbose: true }
2. [for each service repo: check latest_tag]
3. [compare deployed versions with latest tags]
4. "3 services are behind their latest tags: api-gateway (v2.1.0 -> v2.3.0), ..."
5. [optionally: sync and update affected repos]
```

---

## Implementation Roadmap

### Phase 1: Library Extraction

**Goal**: Make Repoman's core operations callable as a Rust library, not just
via CLI.

**Tasks**:
- Extract `src/operations/` into a `repoman-core` library crate.
- Ensure all operations return structured data (not just print to stdout).
- The CLI binary becomes a thin wrapper that calls `repoman-core` and
  formats output.
- Keep backward compatibility: the CLI behaves identically.

**Estimated effort**: Medium. Most operations already return `Result<T>` with
structured types. The main work is removing `println!` calls from the
operations layer and returning data instead.

### Phase 2: MCP Server Binary

**Goal**: Create `repoman-mcp` that speaks MCP over stdio.

**Tasks**:
- Add a `repoman-mcp` binary to the Cargo workspace.
- Implement the MCP JSON-RPC protocol (or use an existing Rust MCP SDK).
- Map each tool to a `repoman-core` function call.
- Implement resource handlers for vault state and metadata.
- Add error handling that returns structured MCP errors.

**Estimated effort**: Medium. The protocol is straightforward JSON-RPC. The
main complexity is in serializing outputs correctly and handling edge cases.

### Phase 3: Testing and Documentation

**Goal**: Ensure the MCP server works correctly with real MCP clients.

**Tasks**:
- Integration tests using a mock MCP client.
- Test with Claude Code as a real client.
- Write user-facing documentation for setup and configuration.
- Add the MCP server to CI (build, test, release).

**Estimated effort**: Low-medium.

### Phase 4: Advanced Features

**Goal**: Add capabilities that are uniquely valuable for LLM agents.

**Tasks**:
- **Batch operations**: Allow creating/destroying multiple clones in a
  single tool call for efficiency.
- **Workspace sessions**: Track which clones were created by which agent
  session, enabling automatic cleanup when a session ends.
- **File content access**: Expose file contents from clones as MCP resources,
  so the agent can read code without needing filesystem access.
- **Git operations**: Add tools for commit, branch, diff, and log operations
  within clones, enabling the agent to work entirely through MCP.

**Estimated effort**: High. These are new features that extend beyond the
current CLI surface.

### Phase 5: Ecosystem Integration

**Goal**: Make Repoman MCP a first-class citizen in the AI tooling ecosystem.

**Tasks**:
- Publish to MCP server registries.
- Integration guides for Claude Code, Cursor, Windsurf, and other MCP
  clients.
- Lua plugin hooks that fire on MCP tool invocations.
- Metrics and observability for MCP usage patterns.

**Estimated effort**: Ongoing.
