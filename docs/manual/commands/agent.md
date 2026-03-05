# repoman agent

Manage the background sync agent.

## Synopsis

```
repoman agent start
repoman agent stop
repoman agent status
```

## Description

The agent is a background process that periodically syncs pristines from their remotes and checks for new tags. It runs as a detached child process of repoman.

### start

Spawns the agent in the background. The agent's PID is written to `~/.repoman/logs/agent.pid` and its output goes to `~/.repoman/logs/agent.log`. Returns an error if the agent is already running.

### stop

Sends SIGTERM to the running agent and removes the PID file. Returns an error if the agent is not running.

### status

Reports whether the agent is running, its PID, and the log file path.

## Agent Behavior

Once started, the agent runs a continuous poll loop:

1. For each vaulted repo with an existing pristine, it checks the per-repo `sync_interval` from metadata (default: 3600 seconds / 1 hour).
2. If a repo is due for a sync (based on time since last sync), the agent:
   - Checks the remote for new tags (semver-aware sorting to find the latest).
   - Updates `latest_tag` in metadata if a new tag is found.
   - Fetches all branches and tags into the pristine.
   - Runs `post_sync_on_new_tag` hook if a new tag was detected.
3. On a separate heartbeat interval (default: 300 seconds / 5 minutes), the agent attempts to fast-forward or rebase each clone from its pristine.
4. The agent sleeps until the next repo is due, rather than polling on a fixed interval.

The heartbeat update for clones is best-effort:

- Clones that are behind are fast-forwarded.
- Clones that have diverged are rebased in a temporary copy. If the rebase succeeds, the copy replaces the original. If it fails, the `upstream_conflicts` flag is set in metadata.

## Configuration

The agent respects these settings from `config.yaml`:

| Key | Default | Description |
|-----|---------|-------------|
| `agent_heartbeat_interval` | `300` | Seconds between clone heartbeat updates. |

Per-repo sync intervals are set in metadata (`sync_interval`, default 3600 seconds).

## Arguments

| Argument | Required | Description |
|----------|----------|-------------|
| `action` | Yes | One of: `start`, `stop`, `status`. |

## Examples

Start the agent:

```sh
repoman agent start
```

Check status:

```sh
repoman agent status
```

```
Agent is running (PID: 12345)
Log file: /home/user/.repoman/logs/agent.log
```

Stop the agent:

```sh
repoman agent stop
```

## Tips

- The agent uses `repoman agent run` internally as the actual long-running process. Do not call `run` directly; it is an implementation detail.
- Agent logs go to `~/.repoman/logs/agent.log`, separate from the main repoman log.
- If the agent crashes, the stale PID file is automatically cleaned up the next time you check status or start.
- Configure per-repo sync intervals in metadata or via config. Repos that should sync less frequently (e.g., large monorepos) can have a higher `sync_interval`.
