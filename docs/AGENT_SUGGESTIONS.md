# Agent Suggestions

Ideas for expanding Repoman's background agent into a more capable,
production-ready automation layer. These are forward-looking suggestions --
none are implemented yet. Each section describes the motivation, a rough
design sketch, and open questions.

---

## Table of Contents

- [Webhook Listener Mode](#webhook-listener-mode)
- [systemd / launchd Service Generator](#systemd--launchd-service-generator)
- [Agent Metrics Dashboard](#agent-metrics-dashboard)
- [Graceful Shutdown](#graceful-shutdown)
- [Multi-Remote Sync](#multi-remote-sync)
- [Conflict Auto-Resolution Strategies](#conflict-auto-resolution-strategies)
- [Desktop and Webhook Notifications](#desktop-and-webhook-notifications)
- [Distributed Agent Coordination](#distributed-agent-coordination)
- [Clone Freshness Policies](#clone-freshness-policies)
- [Health Check HTTP Endpoint](#health-check-http-endpoint)
- [Auto-GC Scheduling](#auto-gc-scheduling)
- [Dependency Graph Awareness](#dependency-graph-awareness)
- [Bandwidth Throttling](#bandwidth-throttling)
- [Offline Mode](#offline-mode)

---

## Webhook Listener Mode

**Problem**: The agent currently polls remotes on a fixed interval. For repos
hosted on GitHub or GitLab, this means changes are detected up to
`sync_interval` seconds late, and idle repos waste network round-trips.

**Idea**: Add an optional HTTP server mode that listens for push/tag webhooks
from GitHub, GitLab, Bitbucket, or Gitea. When a webhook fires, the agent
syncs only the affected repo immediately.

**Sketch**:

```yaml
# config.yaml
agent:
  webhook:
    enabled: true
    listen: "0.0.0.0:9876"
    secret: "${REPOMAN_WEBHOOK_SECRET}"   # HMAC validation
    providers:
      - github
      - gitlab
```

```
POST /webhook/github -> parse payload -> identify repo -> instant sync
POST /webhook/gitlab -> parse payload -> identify repo -> instant sync
```

**Open questions**:
- Should the webhook server run inside the same agent process, or as a
  separate binary/mode?
- How to map incoming repo URLs to vault names (normalize URL formats)?
- Should polling be disabled entirely when webhooks are active, or kept as a
  fallback?

---

## systemd / launchd Service Generator

**Problem**: Users currently start the agent manually with
`repoman agent start`. There is no integration with system service managers,
so the agent does not survive reboots or user logouts.

**Idea**: Add a `repoman agent install` command that generates and installs a
systemd user unit (Linux) or launchd plist (macOS).

**Sketch**:

```bash
# Generate and enable the service
repoman agent install

# Equivalent to:
# Linux: creates ~/.config/systemd/user/repoman-agent.service, runs systemctl --user enable --now
# macOS: creates ~/Library/LaunchAgents/com.repoman.agent.plist, runs launchctl load
```

Generated systemd unit:

```ini
[Unit]
Description=Repoman Background Agent
After=network-online.target

[Service]
Type=simple
ExecStart=%h/.cargo/bin/repoman agent run
Restart=on-failure
RestartSec=30
Environment=SSH_AUTH_SOCK=%t/ssh-agent.socket

[Install]
WantedBy=default.target
```

**Open questions**:
- How to handle SSH agent socket forwarding in a systemd context?
- Should `repoman agent install` also set up log rotation (e.g.
  journald-native logging)?
- Windows: should we generate a Windows Service or a Task Scheduler entry?

---

## Agent Metrics Dashboard

**Problem**: The agent runs silently. Beyond tailing the log file, there is no
way to see sync success/failure rates, average sync duration, or how many
tags have been detected over time.

**Idea**: Track metrics in a lightweight SQLite or JSON-lines store and expose
them via `repoman agent metrics` or the TUI dashboard.

**Metrics to track**:
- Total syncs (successful / failed) per repo
- Average sync duration per repo
- Tags detected (with timestamps)
- Heartbeat fast-forward count
- Network errors / auth failures
- Uptime

**Sketch**:

```bash
repoman agent metrics
```

Output:

```
Agent uptime: 3d 7h 22m
Total syncs: 142 (138 ok, 4 failed)

REPO               SYNCS   FAILS   AVG TIME   LAST TAG        LAST SYNC
neovim             48      0       2.3s       v0.10.4         12 min ago
linux              47      4       18.7s      v6.12           1h ago
ripgrep            47      0       1.1s       14.1.1          3h ago
```

**Open questions**:
- SQLite vs. a simple JSON-lines append log?
- Should metrics be viewable in the TUI dashboard, or a separate command?
- Should there be a Prometheus-compatible `/metrics` endpoint?

---

## Graceful Shutdown

**Problem**: `repoman agent stop` sends SIGTERM, but the agent does not
explicitly handle signals. If SIGTERM arrives mid-sync, the pristine could be
left in an inconsistent state.

**Idea**: Install a signal handler that sets a shutdown flag. The agent
finishes the current sync cycle (or the current repo within a cycle) before
exiting cleanly. Optionally write a "last run" summary to the log.

**Sketch**:

```rust
// In agent loop
let shutdown = Arc::new(AtomicBool::new(false));
signal_hook::flag::register(SIGTERM, Arc::clone(&shutdown))?;
signal_hook::flag::register(SIGINT, Arc::clone(&shutdown))?;

loop {
    if shutdown.load(Ordering::Relaxed) {
        info!("agent: received shutdown signal, finishing current cycle");
        break;
    }
    // ... sync repos ...
}

// Cleanup: remove PID file, write summary
```

**Open questions**:
- Should there be a hard timeout (e.g. force-kill after 60s)?
- Should `repoman agent stop` wait for the graceful shutdown, or return
  immediately?

---

## Multi-Remote Sync

**Problem**: Some repos have mirrors on multiple hosting providers (e.g.
GitHub + internal GitLab). Currently, Repoman only syncs from a single origin.

**Idea**: Support multiple remotes per repo in metadata. The agent syncs from
each remote and merges refs. Users can configure a primary remote for pushes
and secondary remotes for read-only sync.

**Sketch**:

```yaml
repos:
  my-app:
    remotes:
      - url: git@github.com:org/my-app.git
        name: github
        primary: true
      - url: git@gitlab.internal.com:org/my-app.git
        name: gitlab
```

**Sync behavior**:
- Fetch from all remotes into the pristine under namespaced refs
  (`refs/remotes/github/*`, `refs/remotes/gitlab/*`).
- Report divergence between remotes as a warning.
- Clone commands use the primary remote's refs by default.

**Open questions**:
- How to handle tag conflicts across remotes?
- Should the agent detect when remotes have diverged and alert the user?

---

## Conflict Auto-Resolution Strategies

**Problem**: When `repoman update` attempts to fast-forward a clone and finds
it has diverged, it currently prints a message and skips. The user must
manually resolve the divergence.

**Idea**: Offer configurable strategies for handling diverged clones during
update/heartbeat:

| Strategy     | Behavior                                      |
|-------------|-----------------------------------------------|
| `skip`      | Current behavior. Do nothing.                 |
| `rebase`    | Attempt `git rebase` onto upstream.           |
| `stash`     | Stash local changes, fast-forward, pop stash. |
| `reset`     | Hard-reset to upstream (destructive).         |
| `notify`    | Skip, but send a notification.                |
| `branch`    | Create a backup branch, then reset.           |

**Sketch**:

```yaml
repos:
  my-app:
    clone_defaults:
      conflict_strategy: stash
```

**Open questions**:
- Should this be per-repo, per-clone, or global?
- Rebase and stash can fail. What is the fallback?
- How much risk is acceptable in an automated process?

---

## Desktop and Webhook Notifications

**Problem**: The agent detects new tags and sync events, but the only output
channel is a log file. Users do not learn about events unless they check
the log.

**Idea**: Support configurable notification backends:

- **Desktop**: `notify-send` (Linux), `osascript` (macOS),
  `BurntToast` (Windows)
- **Slack/Discord/Teams webhooks**: POST a message to a channel
- **Email**: via local `sendmail` or SMTP
- **Custom script**: user-provided command

**Sketch**:

```yaml
agent:
  notifications:
    on_new_tag:
      - type: desktop
      - type: slack
        webhook_url: https://hooks.slack.com/services/T.../B.../xxx
        template: "New tag {tag} for {repo}"
    on_sync_failure:
      - type: desktop
```

**Open questions**:
- Should notifications be deduped (e.g. only notify once per tag)?
- Should notification backends be implemented in core, or delegated to
  Lua plugins?
- Rate limiting to avoid notification storms during bulk init.

---

## Distributed Agent Coordination

**Problem**: If a user runs Repoman on multiple machines (e.g. laptop +
server), each agent independently syncs the same repos. This wastes bandwidth
and can lead to inconsistent state if the machines share storage.

**Idea**: Add a simple coordination protocol so agents on different machines
can claim repos and avoid duplicate work.

**Sketch**:
- Agents register themselves in a shared coordination file or lightweight
  service (e.g. a shared NFS path, Redis, or a tiny HTTP coordinator).
- Each repo is assigned to one agent at a time. Other agents skip it.
- If an agent goes down, its repos are automatically reassigned after a
  timeout.

**Open questions**:
- Is this a real problem for enough users to justify the complexity?
- What is the simplest coordination backend (file lock, SQLite WAL, etcd)?
- Should this be an opt-in mode for teams/orgs?

---

## Clone Freshness Policies

**Problem**: Stale clones accumulate silently. Users must manually run
`repoman gc` or `repoman destroy --stale`. There is no way to express "I want
clones for this repo to be automatically cleaned up after 7 days."

**Idea**: Per-repo freshness policies that the agent enforces automatically.

**Sketch**:

```yaml
repos:
  scratch-repo:
    clone_freshness:
      max_age_days: 7
      action: destroy           # or "warn"
  important-repo:
    clone_freshness:
      max_age_days: 30
      action: warn
```

**Behavior**: On each agent cycle, check clone ages against policies. Destroy
or warn as configured. Log all actions.

**Open questions**:
- Should freshness be measured from clone creation, last commit, or last
  `git status` activity?
- Should clones with uncommitted work be exempt from auto-destroy?
- Should there be a "protect" flag on individual clones?

---

## Health Check HTTP Endpoint

**Problem**: In server/CI environments, operators need a way to verify the
agent is running and healthy without SSH-ing into the machine.

**Idea**: Add an optional HTTP health check endpoint that the agent exposes.

**Sketch**:

```yaml
agent:
  health_check:
    enabled: true
    listen: "127.0.0.1:9877"
```

```
GET /health -> 200 OK {"status": "ok", "uptime": "3d 7h", "last_cycle": "2m ago"}
GET /repos   -> 200 OK [{"name": "neovim", "last_sync": "...", "status": "ok"}, ...]
```

**Open questions**:
- Should this share the same HTTP server as webhooks (if both are enabled)?
- Should the endpoint require authentication (API key, mTLS)?
- Is a simple TCP liveness check sufficient for most use cases?

---

## Auto-GC Scheduling

**Problem**: `repoman gc` must be run manually. Over time, pristine repos
accumulate loose objects and stale clones pile up.

**Idea**: Let the agent run GC on a configurable schedule.

**Sketch**:

```yaml
agent:
  gc:
    enabled: true
    interval: 86400     # run gc once per day
    stale_days: 30      # destroy clones older than 30 days
    pristine_gc: true   # run git gc --auto on pristines
```

**Behavior**: The agent tracks the last GC timestamp. When the interval has
elapsed, it runs the equivalent of `repoman gc --days <stale_days>`.

**Open questions**:
- Should GC run in the main agent loop or in a separate thread?
- Should there be a "quiet hours" window (e.g. don't GC during work hours)?
- How to prevent GC from interfering with active clones?

---

## Dependency Graph Awareness

**Problem**: Some repos depend on others (e.g. a library repo and an
application repo). When the library gets a new tag, the application's clones
should be updated too, or at least the user should be notified.

**Idea**: Define dependency edges between repos. When a dependency is synced
or tagged, trigger downstream actions.

**Sketch**:

```yaml
repos:
  my-library:
    hooks:
      post_sync_on_new_tag: "echo 'Library updated'"
  my-app:
    depends_on:
      - my-library
    hooks:
      on_dependency_update: "cargo update && cargo test"
```

**Behavior**: When `my-library` syncs and finds a new tag, the agent also
triggers `on_dependency_update` in `my-app`.

**Open questions**:
- How deep should dependency chains go?
- How to prevent circular dependency loops?
- Should dependencies be declared in config.yaml, or auto-detected from
  package manifests (Cargo.toml, package.json)?

---

## Bandwidth Throttling

**Problem**: On metered connections or shared networks, the agent's sync
activity can consume significant bandwidth, especially when syncing large
repos like the Linux kernel.

**Idea**: Add bandwidth limits to the agent's fetch operations.

**Sketch**:

```yaml
agent:
  bandwidth:
    max_download_kbps: 5000    # 5 MB/s limit
    max_concurrent_syncs: 2
    metered_mode: auto         # auto-detect metered connections (Linux/macOS)
```

**Open questions**:
- Can git2 be configured with transfer rate limits, or does this need to
  shell out to `git` with `http.lowSpeedLimit`?
- Should there be a "pause" mode for when the user is on a mobile hotspot?
- How to detect metered connections cross-platform?

---

## Offline Mode

**Problem**: When the network is unavailable (airplane, VPN down, flaky
Wi-Fi), the agent logs errors for every repo and retries on the next cycle.
There is no way to tell the agent "I know the network is down, stop trying."

**Idea**: Add an offline mode that suspends all network operations while still
allowing local operations (clone from pristine, destroy, gc, status).

**Sketch**:

```bash
# Manually enable offline mode
repoman agent offline

# The agent logs:
# "agent: offline mode enabled, skipping network operations"

# Re-enable
repoman agent online

# Auto-detect: if N consecutive syncs fail for all repos, enter offline mode
# and retry periodically at a reduced frequency
```

**Config**:

```yaml
agent:
  offline:
    auto_detect: true
    retry_interval: 600         # when offline, check connectivity every 10 min
    consecutive_failures: 3     # enter offline mode after 3 consecutive failures
```

**Open questions**:
- Should offline mode be per-repo or global?
- How to detect "network is back" reliably?
- Should the agent queue sync requests while offline and execute them on
  reconnect?
