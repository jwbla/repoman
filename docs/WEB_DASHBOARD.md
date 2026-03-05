# Web Dashboard Design Document

## 1. Vision & Motivation

### Why a Web Dashboard?

Repoman already has a terminal-based TUI dashboard (ratatui), which works well for
quick keyboard-driven inspection from a local terminal session. A localhost web
dashboard solves a different set of problems:

**Remote access.** SSH into a dev server, start the agent, and check repo status
from a browser on your laptop without needing a terminal session open. This is
the single most compelling reason -- the agent already runs as a background
daemon, and a web UI lets you observe and control it without attaching to a
terminal.

**Richer information density.** HTML can render tables, timelines, collapsible
sections, and color-coded status badges far more effectively than a 80x24
terminal. The TUI is deliberately minimal; the web dashboard can be comprehensive
without feeling cramped.

**Trigger actions without CLI knowledge.** "Sync this repo" or "create a clone"
should be a single button click. This lowers the barrier for occasional use --
you don't need to remember `repoman clone myrepo --branch feature/x`.

**Agent observability.** The agent runs silently in the background. A web
dashboard gives it a face: uptime, last poll cycle, next scheduled sync, error
log tail, repos with upstream conflicts. This turns the agent from a "fire and
forget" daemon into something you can actually monitor.

**Shareability (future).** If repoman ever grows multi-user or team features,
a web UI is the natural surface. Even for single-user, you might want to show
a coworker your repo setup by sharing a localhost URL over a tunnel.

### What the TUI Dashboard Is Not Being Replaced By

The TUI remains the right tool for quick terminal checks (`repoman dashboard`),
keyboard-heavy workflows, and environments where a browser is unavailable. The
web dashboard is additive, not a replacement.

### Precedent from Similar Tools

Several developer tools have proven that a localhost web dashboard is the right
pattern for background services:

**Syncthing** is the gold standard for this pattern. It embeds a full web UI in
its Go binary, binds to `127.0.0.1:8384` by default, uses an API key for auth
when exposed beyond localhost, and provides real-time sync status, folder
management, and device configuration -- all through server-rendered HTML with
minimal JavaScript. Syncthing's approach is exactly what repoman should emulate:
simple, self-contained, and secure by default.

**Grafana** demonstrates how dashboards, timelines, and status panels should look
for monitoring data. The sync history timeline and agent status panels should
borrow from Grafana's visual language.

**Portainer** shows how container management maps to a web UI -- listing
resources, viewing details, and triggering actions (start/stop/remove). Repoman's
vault/pristine/clone hierarchy maps cleanly to the same pattern.

**pgAdmin** and **Adminer** prove that single-binary or embedded web UIs for
local services are well-accepted by developers. Nobody wants to install a
separate frontend app for a CLI tool.

**Gitea** is relevant as a self-hosted Git UI, though repoman's scope is much
narrower. The repo list and detail views in Gitea are a useful reference for
information architecture.

The common thread: all of these embed the web server in the main binary, bind to
localhost by default, use server-rendered HTML (or minimal JS), and treat
security as opt-in escalation (localhost trust by default, auth tokens when
exposed to the network).


## 2. Architecture Overview

### Recommendation: Embed in the Agent Process

The web server should run inside the existing `repoman agent run` process, not as
a separate subcommand. Reasons:

1. **The agent is already a long-lived daemon.** Adding an HTTP listener to it is
   natural -- the web UI is fundamentally "a window into the agent."

2. **Shared state.** The agent already loads vault, metadata, and config on each
   poll cycle. The web server can read the same data without IPC or file locking
   races.

3. **Single process to manage.** Users already know `repoman agent start/stop`.
   Adding a separate `repoman web` process means two daemons to manage, two PID
   files, two log files.

4. **WebSocket for live updates.** The agent loop knows when it starts a sync,
   detects a new tag, or encounters an error. With the web server in-process,
   it can push these events to connected WebSocket clients with zero
   serialization overhead.

The agent's `run_agent_loop` currently uses `tokio::time::sleep` between poll
cycles. Adding an axum server is a matter of `tokio::select!` between the poll
loop and the HTTP server -- both are async and run on the same tokio runtime.

A `--no-web` flag on `repoman agent run` (or `agent.web.enabled: false` in
config) provides an opt-out for users who want a headless agent.

### Alternative Considered: Separate `repoman web` Subcommand

This was rejected for the reasons above, but it has one advantage: you could run
the web UI without the agent's sync loop. This is a minor benefit -- the web UI
without the agent is just a read-only viewer of vault state, which `repoman list
--json` already provides. If a read-only mode is desired later, the agent could
accept a `--web-only` flag that skips the sync loop.

### Technology Stack

**HTTP framework: axum.** Repoman already depends on tokio. Axum is the standard
Rust HTTP framework in the tokio ecosystem -- it is fast, well-documented, and
has first-class support for WebSockets, middleware, and state extraction. There
is no reason to consider actix-web (different runtime), warp (less ergonomic), or
rocket (heavier).

**HTML rendering: askama.** Compile-time Jinja2-style templates that produce
type-checked HTML. Errors are caught at compile time, not at runtime. Askama
integrates well with axum via `askama_axum`. Maud (macro-based HTML) was
considered but is harder to read for non-trivial layouts and makes it difficult
for contributors who think in HTML rather than Rust macros.

**Interactivity: htmx.** htmx lets the server return HTML fragments that replace
parts of the page, giving SPA-like interactivity without a JavaScript build step,
a JS framework, a bundler, or client-side state management. This is a
perfect fit for repoman's dashboard:

- Click "Sync" -> POST /api/repos/{name}/sync -> server returns updated status
  HTML fragment -> htmx swaps it into the page.
- WebSocket pushes an agent status update -> htmx swaps the status badge.
- No React, no npm, no node_modules, no webpack. The entire frontend is HTML
  templates and a single 14KB htmx script.

This is the Syncthing model: the server owns all the state and rendering logic,
the browser is a thin display layer.

**Styling: Tailwind CSS via CDN.** Including Tailwind via CDN (`<script
src="https://cdn.tailwindcss.com">`) avoids a build step entirely. For
offline/air-gapped use, the CDN script can be embedded in the binary alongside
htmx. In practice, most developers have internet access and the CDN approach
keeps the binary smaller. A `play cdn` build and an `embed` build could be
offered via a Cargo feature flag.

Alternative: Pico CSS or Simple.css, which are classless CSS frameworks that
style semantic HTML with zero configuration. These produce clean, readable pages
with no class attributes needed. Worth considering for the MVP if Tailwind CDN
feels heavyweight.

**Static asset embedding: rust-embed.** Embeds templates, CSS, JS, and any
static assets directly into the compiled binary. No external files to ship or
lose. The `include_str!` / `include_bytes!` macros work for individual files,
but rust-embed handles directories and provides content-type detection and
ETag headers for caching.

**WebSocket: axum's built-in support.** Axum has native WebSocket upgrade
support. No additional dependencies needed.


## 3. Feature Set (Prioritized)

### Phase 1: MVP (Must-Have)

These features make the web dashboard immediately useful as an agent monitor
and repo management interface.

#### Dashboard Home Page
- Summary statistics: total repos, total clones, pristines initialized,
  agent uptime
- Agent status panel: running/stopped, PID, uptime, next sync due, last
  poll cycle result
- Repository table with columns: name, pristine status (yes/no), clone
  count, last sync (relative time), latest tag, upstream conflicts flag
- Sortable and filterable (htmx-powered search box that filters server-side)
- Each repo name links to the detail page

#### Repository Detail Page
- Full repo information: URL, added date, default branch, sync interval
- Pristine status: exists/missing, path, created date, branches list
- Clones table: name, path, branch, dirty files, ahead/behind, upstream
  conflicts, created date
- Sync history: last sync timestamp, sync type (auto/manual)
- Latest tag
- Alternates health check status
- Action buttons (see below)

#### Actions from UI
- **Sync repo**: POST button that triggers `operations::sync_pristine()`
- **Create clone**: Form with optional branch and clone name, triggers
  `operations::create_clone()`
- **Agent start/stop**: Buttons to start/stop the agent (when web server
  runs independently, or to restart the agent loop)

#### Agent Status (Real-Time)
- WebSocket connection that pushes: agent state changes, sync start/complete
  events, new tag detections, error notifications
- Status badge in the header that shows connected/disconnected

#### Config Viewer
- Read-only display of the effective configuration
- Shows config file path and whether it was loaded or using defaults

### Phase 2: Nice-to-Have

These features round out the dashboard into a full management interface.

#### Clone Management
- Destroy clone from UI (with confirmation modal)
- Open clone path (copy-to-clipboard button)

#### Log Viewer
- Tail `~/.repoman/logs/repoman.log` in the browser
- Auto-scroll with pause button
- Filter by level (debug/info/warn/error)
- Streamed via WebSocket or SSE

#### Vault Management
- Add repo to vault (URL input form)
- Remove repo from vault (with confirmation)
- Rename repo
- Manage aliases

#### Export/Import
- Export vault as YAML (download button)
- Import from uploaded YAML file

#### Sync History Timeline
- Visual timeline showing sync events per repo over the last 24h/7d/30d
- Color-coded: green for success, red for failure, blue for new tag

#### Plugin Management
- List loaded plugins
- Enable/disable plugins (requires config file write)
- View plugin details (name, description, hooks registered)

#### Theme Toggle
- Dark/light mode (stored in localStorage, sent as cookie for SSR)
- System preference detection via `prefers-color-scheme`

### Phase 3: Future

These features are speculative and would require significant new infrastructure.

#### Multi-User Support
- User accounts with role-based access (admin, viewer)
- Per-user API tokens
- Audit log of actions

#### REST API for External Integrations
- Formal versioned REST API (v1) with OpenAPI spec
- Webhook receiver for GitHub/GitLab push events (trigger sync on push)
- Outgoing webhooks (notify external services on new tag, sync failure)

#### Metrics & Monitoring
- Prometheus `/metrics` endpoint: repos_total, clones_total, sync_duration,
  sync_errors_total, agent_uptime_seconds
- Integrates with existing Grafana/Prometheus stacks

#### Webhook Receiver
- Endpoint that accepts GitHub/GitLab webhook payloads
- On push event, triggers sync for the matching repo
- HMAC signature verification for security


## 4. UI Mockups

### Dashboard Home Page

```
+------------------------------------------------------------------+
|  REPOMAN                                    Agent: Running (PID   |
|                                             1234) | Uptime: 2d   |
|                                             3h | Next sync: 12m  |
+------------------------------------------------------------------+
|                                                                   |
|  Overview          12 repos | 28 clones | 11 pristines | 1 error |
|                                                                   |
|  +--------------------------------------------------------------+|
|  | Search: [___________________]              Sort: [Last Sync v]||
|  +--------------------------------------------------------------+|
|  | NAME            PRISTINE  CLONES  LAST SYNC     TAG     WARN ||
|  |--------------------------------------------------------------||
|  | linux-kernel       *        3     12 min ago    v6.8       -- ||
|  | repoman            *        2     2 hours ago   v0.3.0     -- ||
|  | tokio              *        1     5 hours ago   v1.37      -- ||
|  | axum               *        0     1 day ago     v0.7.9     -- ||
|  | serde              -        0     never         --         -- ||
|  | my-project         *        4     3 min ago     --    CONFLICT||
|  | ...                                                           ||
|  +--------------------------------------------------------------+|
|                                                                   |
|  [Sync All]                                                       |
+------------------------------------------------------------------+
```

### Repository Detail Page

```
+------------------------------------------------------------------+
|  REPOMAN  >  repoman                                              |
+------------------------------------------------------------------+
|                                                                   |
|  URL:      https://github.com/user/repoman.git                   |
|  Added:    2024-11-15                                             |
|  Branch:   main                                                   |
|  Tag:      v0.3.0                                                |
|  Interval: 3600s (1 hour)                                         |
|  Pristine: ~/.repoman/pristines/repoman                          |
|  Last Sync: 2 hours ago (manual)                                 |
|  Alternates: OK                                                   |
|                                                                   |
|  [Sync Now]  [Create Clone]                                       |
|                                                                   |
|  Branches: main, develop, feature/web-dashboard                  |
|                                                                   |
|  Clones (2):                                                      |
|  +--------------------------------------------------------------+|
|  | NAME         BRANCH    DIRTY   AHEAD/BEHIND   CREATED        ||
|  |--------------------------------------------------------------||
|  | repoman-abc  main        0       +0/-0        2024-12-01     ||
|  | repoman-xyz  develop     3       +2/-1        2025-01-15     ||
|  +--------------------------------------------------------------+|
|  | [Destroy]    [Destroy]                                        ||
|  +--------------------------------------------------------------+|
|                                                                   |
|  Sync History:                                                    |
|  | 2025-02-15 10:30  auto   OK                                   |
|  | 2025-02-15 09:30  auto   OK                                   |
|  | 2025-02-14 18:00  manual OK                                   |
+------------------------------------------------------------------+
```

### Agent Status Panel (Header Component)

```
+------------------------------------------------------------------+
|  [*] Agent Running        PID: 48291                              |
|      Uptime: 2 days, 3 hours, 14 minutes                        |
|      Last poll: 12 minutes ago (all OK)                          |
|      Next poll: in 48 minutes                                     |
|      Repos monitored: 11                                          |
|      Errors this session: 0                                       |
|                                                                   |
|      [Stop Agent]  [Force Sync All]                              |
+------------------------------------------------------------------+
```

### Create Clone Modal

```
+------------------------------------------+
|  Create Clone: repoman                    |
|                                           |
|  Branch:  [main____________v]             |
|  Name:    [repoman-________]  (optional)  |
|                                           |
|  [Cancel]              [Create Clone]     |
+------------------------------------------+
```


## 5. Security Concerns

Security is the most critical section of this document. A web server on localhost
is a fundamentally different attack surface than a CLI tool. The design must be
secure by default.

### 5.1 Bind Address: Localhost Only

**Default: `127.0.0.1:9876`.** The server MUST bind to `127.0.0.1`, never
`0.0.0.0`, unless the user explicitly opts in. This is the single most important
security decision.

Binding to `0.0.0.0` exposes the dashboard to the local network and potentially
the internet. On shared servers, other users on the same machine cannot connect
to `127.0.0.1` services, but they can connect to `0.0.0.0` services.

The port `9876` is arbitrary but should be outside the common range (8080, 3000,
8000) to avoid conflicts. It should be configurable.

**IPv6:** Also bind to `[::1]` (IPv6 localhost) if available. Axum's
`TcpListener` supports dual-stack.

### 5.2 No Authentication by Default (Localhost Trust Model)

When bound to `127.0.0.1`, no authentication is required. This is the Syncthing
model and the standard for localhost development tools. The reasoning:

- Any process running as the same user can already run `repoman` commands
  directly.
- Any process running as the same user can read `~/.repoman/vault/vault.json`
  directly.
- The web UI does not expose any capabilities that the CLI does not already
  provide.
- Adding mandatory auth for localhost creates friction without meaningful
  security gain.

However, a browser-based CSRF attack could cause a page on the internet to make
requests to `localhost:9876`. See Section 5.4.

### 5.3 Mandatory Authentication When Not Localhost

If the bind address is changed to anything other than `127.0.0.1` or `[::1]`,
the server MUST refuse to start unless an API key is configured. This prevents
accidental exposure without auth.

```yaml
# config.yaml
web:
  bind_address: "0.0.0.0"  # exposed to network
  port: 9876
  api_key: "rpn_a1b2c3d4e5f6..."  # REQUIRED when not localhost
```

Startup should print a clear warning:
```
WARNING: Web dashboard bound to 0.0.0.0:9876 (network-accessible).
API key authentication is enabled. Keep your API key secret.
Consider enabling TLS (web.tls) for encrypted connections.
```

The API key is sent as a `Bearer` token in the `Authorization` header, or as a
cookie set after a login page. For browser use, the login page + cookie approach
is more practical.

### 5.4 CSRF Protection

This is the most subtle security concern for localhost web servers. A malicious
website can instruct the browser to make POST requests to `http://localhost:9876`
using JavaScript. The browser will include cookies, making the request appear
legitimate.

**Mitigations:**

1. **Origin/Referer header validation.** All state-changing requests (POST, PUT,
   DELETE) must check that the `Origin` or `Referer` header matches the expected
   host (`localhost:9876` or `127.0.0.1:9876`). Reject requests with foreign
   origins. This is simple, effective, and how Syncthing handles it.

2. **CSRF tokens.** For form submissions, include a hidden CSRF token that is
   validated server-side. With htmx, this can be injected into all requests via
   `hx-headers` on the `<body>` tag.

3. **SameSite cookies.** If session cookies are used (for API key auth), set
   `SameSite=Strict` to prevent cross-origin cookie inclusion.

4. **Custom header requirement.** Require a custom header (e.g.,
   `X-Repoman-Request: true`) on all API requests. Browsers cannot set custom
   headers in cross-origin requests without a CORS preflight, which will fail
   because the server does not send CORS headers.

**Recommendation:** Use all four. The Origin check and custom header are the
primary defenses. CSRF tokens and SameSite cookies are defense-in-depth.

### 5.5 CORS Policy

The server MUST NOT send `Access-Control-Allow-Origin` headers. This prevents
any cross-origin JavaScript from making requests to the dashboard. Without CORS
headers, browsers will block cross-origin fetch/XHR responses (though simple
POST requests can still be sent -- hence the CSRF protections above).

### 5.6 Command Injection Prevention

The web UI accepts user input in several places: clone names, branch names, repo
URLs. All of these flow through repoman's existing Rust operations layer, which
uses git2 (a C library) rather than shelling out to `git`. There is no path from
web input to `Command::new("sh").arg("-c").arg(user_input)`.

The one exception is lifecycle hooks, which are shell commands defined in
`config.yaml`. These are never user-editable through the web UI. The web UI
can display hooks but not create or modify them.

**Rules:**
- Never construct shell commands from web input.
- All repo operations go through `operations::*`, which use git2.
- Clone names and branch names are validated against a whitelist pattern
  (alphanumeric, hyphens, underscores, dots, slashes).
- Repo URLs are validated by `vault::extract_repo_name()` before use.

### 5.7 File Path Traversal Prevention

The log viewer and config viewer display file contents. These must only serve
files within known safe directories:
- `~/.repoman/logs/` (log files)
- `~/.config/repoman/` (config files)
- `~/.repoman/vault/*/metadata.json` (metadata files)

The server must resolve paths and verify they fall within these directories
after symlink resolution (using `std::fs::canonicalize`). Never serve arbitrary
files based on URL path components.

### 5.8 TLS for Non-Localhost Bindings

When bound to a non-localhost address, the dashboard should support TLS:

```yaml
web:
  bind_address: "0.0.0.0"
  tls:
    cert_path: "/path/to/cert.pem"
    key_path: "/path/to/key.pem"
```

For localhost use, TLS is unnecessary (traffic never leaves the machine). For
network-exposed deployments, TLS prevents credential sniffing and MITM attacks.

Axum supports TLS via `axum-server` with `rustls` (pure Rust, no OpenSSL
dependency for TLS specifically).

### 5.9 Rate Limiting

Apply rate limiting to prevent abuse:
- API endpoints: 60 requests per minute per IP
- Login/auth endpoints: 5 attempts per minute per IP
- WebSocket connections: 5 concurrent per IP

For localhost, rate limiting is optional but provides defense against runaway
scripts.

### 5.10 Comparison to Syncthing's Security Model

Syncthing's approach is the closest analogue and worth studying:

| Aspect | Syncthing | Repoman (proposed) |
|--------|-----------|-------------------|
| Default bind | 127.0.0.1:8384 | 127.0.0.1:9876 |
| Auth on localhost | API key in config (optional GUI password) | None (CSRF protection only) |
| Auth on network | API key required | API key required, refuse to start without it |
| CSRF protection | API key header on all requests | Origin check + custom header + CSRF tokens |
| TLS | Optional (auto-generated cert) | Optional (user-provided cert) |
| CORS | None | None |

Repoman's approach is slightly more relaxed on localhost (no API key by default,
relying on CSRF mitigations instead) but stricter on network exposure (refuses to
start without auth). This is appropriate for a single-user developer tool.


## 6. Technology Stack Summary

| Component | Choice | Rationale |
|-----------|--------|-----------|
| HTTP framework | **axum 0.8+** | Tokio-native, already have tokio dependency |
| Templates | **askama** (+ askama_axum) | Compile-time checked, Jinja2 syntax, type-safe |
| Interactivity | **htmx 2.x** | No JS build step, server-rendered fragments |
| Styling | **Pico CSS** (MVP), **Tailwind CDN** (later) | Classless CSS for fast MVP, upgrade path to Tailwind |
| WebSocket | **axum built-in** | Native support, no extra deps |
| Asset embedding | **rust-embed** | Embeds static files in binary |
| TLS | **axum-server + rustls** | Pure Rust, optional feature gate |
| Serialization | **serde_json** (already used) | JSON API responses |

### New Dependencies

```toml
# Cargo.toml additions
axum = { version = "0.8", features = ["ws"] }
askama = "0.12"
askama_axum = "0.4"
rust-embed = "8"
tower = "0.5"                    # middleware (rate limiting, logging)
tower-http = { version = "0.6", features = ["fs", "cors", "trace"] }

# Optional, behind feature flag
axum-server = { version = "0.7", features = ["tls-rustls"], optional = true }
```

### Cargo Feature Flags

```toml
[features]
default = ["web"]
web = ["axum", "askama", "askama_axum", "rust-embed", "tower", "tower-http"]
web-tls = ["web", "axum-server"]
```

Users who want a minimal binary without the web dashboard can build with
`--no-default-features`. This keeps the web stack fully optional.


## 7. Implementation Plan

### Phase 1: Foundation (Estimated: 2-3 days)

**Goal:** Agent serves a basic web page showing repo list and agent status.

1. **Add dependencies.** Add axum, askama, rust-embed, tower-http to Cargo.toml
   behind a `web` feature flag.

2. **Create module structure.**
   ```
   src/web/
     mod.rs          -- Server startup, router, shared state
     handlers.rs     -- Route handlers (index, repo detail, actions)
     templates.rs    -- Askama template structs
     state.rs        -- AppState (Arc<Config>, broadcast channel)
     middleware.rs   -- CSRF check, request logging
   templates/
     base.html       -- Layout with htmx + CSS
     index.html      -- Dashboard home page
     repo_detail.html -- Repo detail page
     components/
       repo_row.html   -- Single repo row (htmx partial)
       agent_status.html -- Agent status panel (htmx partial)
   static/
     htmx.min.js     -- htmx 2.x (vendored, ~14KB)
     pico.min.css    -- Pico CSS (vendored, ~10KB)
   ```

3. **Embed HTTP server in agent loop.** Modify `agent::run_agent_loop` to
   `tokio::select!` between the sync loop and the axum server:

   ```rust
   pub async fn run_agent_loop(config: &Config) -> Result<()> {
       let (tx, _rx) = tokio::sync::broadcast::channel::<AgentEvent>(64);
       let state = AppState::new(config.clone(), tx.clone());

       let web_server = web::serve(state.clone());
       let sync_loop = run_sync_loop(config, tx);

       tokio::select! {
           result = web_server => { /* log error */ },
           result = sync_loop => { /* log error */ },
       }
       Ok(())
   }
   ```

4. **Implement core routes.**
   - `GET /` -- Dashboard home page (full HTML)
   - `GET /repos` -- Repo list fragment (htmx partial for search/filter)
   - `GET /repos/{name}` -- Repo detail page
   - `GET /api/agent/status` -- Agent status JSON
   - `GET /api/repos` -- Repo list JSON (reuses `operations::list_all_repos`)
   - `GET /api/repos/{name}` -- Repo detail JSON (reuses
     `operations::get_detailed_status`)

5. **AppState.** Wrap `Config` in `Arc` and pass as axum state. The web handlers
   call into `operations::*` and `vault::Vault::load()` the same way commands
   do. No new data access patterns needed.

6. **CSRF middleware.** Implement `Origin` header check as a tower middleware
   layer applied to all non-GET routes.

7. **Config.** Add `web` section to config struct:
   ```rust
   #[derive(Debug, Clone, Deserialize, Serialize)]
   pub struct WebConfig {
       pub enabled: Option<bool>,      // default: true
       pub bind_address: Option<String>, // default: "127.0.0.1"
       pub port: Option<u16>,          // default: 9876
       pub api_key: Option<String>,    // default: None
   }
   ```

### Phase 2: Interactivity (Estimated: 2-3 days)

**Goal:** Users can trigger actions and see live updates.

1. **Sync trigger.** `POST /api/repos/{name}/sync` endpoint. Returns an htmx
   fragment with updated repo status. The handler calls
   `operations::sync_pristine()` (async). Show a spinner via htmx's
   `hx-indicator`.

2. **Clone creation.** `POST /api/repos/{name}/clone` endpoint. Accepts
   `branch` and `clone_name` form params. Calls `operations::create_clone()`.
   Returns updated clone table fragment.

3. **WebSocket for live events.** `GET /ws` upgrade endpoint. Agent loop sends
   `AgentEvent` variants (SyncStarted, SyncCompleted, NewTagDetected, Error)
   through a `tokio::sync::broadcast` channel. WebSocket handler subscribes
   and forwards as JSON messages. Client-side htmx-ws extension or a small
   custom JS snippet receives events and swaps status elements.

4. **Repo detail page.** Full detail view with all metadata fields, clone
   table with ahead/behind/dirty status, and action buttons.

5. **Error handling in UI.** Failed operations show inline error messages
   (htmx `hx-target` with error div). Toast notifications for transient
   errors.

### Phase 3: Polish (Estimated: 3-4 days)

**Goal:** Feature parity with common dashboard operations.

1. **Log viewer.** `GET /logs` page with `GET /api/logs/tail?lines=100`
   endpoint. Uses SSE (Server-Sent Events) for live tailing -- simpler than
   WebSocket for one-way streaming. Reads from
   `~/.repoman/logs/repoman.log`. Path validated against config directory.

2. **Config viewer.** `GET /config` page showing effective config as formatted
   YAML. Read-only in the web UI (editing config via web is a security risk
   and complexity explosion).

3. **Clone destruction.** `DELETE /api/repos/{name}/clones/{clone}` endpoint
   with confirmation (htmx `hx-confirm` attribute for browser confirm dialog).

4. **Export/Import.** `GET /api/export` returns YAML. `POST /api/import`
   accepts uploaded YAML file.

5. **Theme toggle.** Dark/light mode. Pico CSS supports `data-theme="dark"`.
   Toggle button stores preference in localStorage and sets a cookie so the
   server can render the correct theme on first load.

6. **Vault management.** Add repo form, remove repo button, alias management.
   Each action is a form POST that returns an updated page fragment.

### Phase 4: Production Readiness (Estimated: 2-3 days)

**Goal:** Safe for non-localhost deployments.

1. **API key authentication.** Login page, session cookie, middleware that
   checks auth on all routes. API key generated with
   `repoman web generate-key`.

2. **TLS support.** Behind `web-tls` feature flag. Config fields for cert/key
   paths. Uses `axum-server` with `rustls`.

3. **Rate limiting.** Tower middleware for request rate limiting. Uses
   `tower::limit::RateLimitLayer` or a simple token bucket.

4. **Metrics endpoint.** `GET /metrics` in Prometheus format. Exposes
   repos_total, clones_total, syncs_total, sync_errors_total,
   agent_uptime_seconds.


## 8. API Design

### REST Endpoints

All endpoints return HTML by default (for htmx consumption). Endpoints under
`/api/` return JSON. The same handler logic is used; only the response format
differs.

#### Read Operations (GET)

| Endpoint | Response | Description |
|----------|----------|-------------|
| `GET /` | HTML | Dashboard home page |
| `GET /repos/{name}` | HTML | Repo detail page |
| `GET /config` | HTML | Config viewer page |
| `GET /logs` | HTML | Log viewer page |
| `GET /api/repos` | JSON | List all repos with status |
| `GET /api/repos/{name}` | JSON | Detailed repo status |
| `GET /api/agent/status` | JSON | Agent status |
| `GET /api/config` | JSON | Effective config |
| `GET /api/logs/tail?lines=N` | JSON | Last N log lines |
| `GET /api/export` | YAML | Export vault |
| `GET /ws` | WebSocket | Live event stream |

#### Write Operations (POST/DELETE)

| Endpoint | Body | Description |
|----------|------|-------------|
| `POST /api/repos/{name}/sync` | -- | Trigger sync for repo |
| `POST /api/repos/{name}/clone` | `{ branch?, name? }` | Create clone |
| `POST /api/repos` | `{ url }` | Add repo to vault |
| `POST /api/repos/{name}/init` | -- | Initialize pristine |
| `DELETE /api/repos/{name}` | -- | Remove repo from vault |
| `DELETE /api/repos/{name}/clones/{clone}` | -- | Destroy clone |
| `POST /api/agent/stop` | -- | Stop agent |
| `POST /api/import` | YAML file | Import vault |

#### htmx Fragment Endpoints

These return HTML fragments (not full pages) for htmx partial updates:

| Endpoint | Returns |
|----------|---------|
| `GET /fragments/repo-list?search=X&sort=Y` | Filtered/sorted repo table body |
| `GET /fragments/repo-row/{name}` | Single repo row (after sync/update) |
| `GET /fragments/agent-status` | Agent status panel |
| `GET /fragments/clone-table/{name}` | Clone table for a repo |

### WebSocket Message Format

Messages from server to client:

```json
{
  "event": "sync_started",
  "repo": "repoman",
  "timestamp": "2025-02-15T10:30:00Z"
}

{
  "event": "sync_completed",
  "repo": "repoman",
  "timestamp": "2025-02-15T10:30:05Z",
  "success": true
}

{
  "event": "new_tag",
  "repo": "repoman",
  "tag": "v0.3.1",
  "timestamp": "2025-02-15T10:30:05Z"
}

{
  "event": "sync_error",
  "repo": "serde",
  "error": "Authentication failed for 'serde'",
  "timestamp": "2025-02-15T10:30:10Z"
}

{
  "event": "agent_status",
  "running": true,
  "pid": 48291,
  "repos_monitored": 12,
  "next_poll_secs": 2880
}
```

Client sends no messages (the WebSocket is one-directional, server to client).
If bidirectional communication is needed later (e.g., "subscribe to specific
repos"), it can be added.


## 9. Configuration

### Config File Additions

```yaml
# ~/.config/repoman/config.yaml

# Existing fields unchanged...
vault_dir: ~/.repoman/vault
pristines_dir: ~/.repoman/pristines
clones_dir: ~/.repoman/clones
plugins_dir: ~/.config/repoman/plugins
logs_dir: ~/.repoman/logs

# New: web dashboard configuration
web:
  enabled: true                  # Set to false to run agent without web server
  bind_address: "127.0.0.1"     # Only change this if you understand the security implications
  port: 9876                     # Dashboard port
  api_key: ~                     # null = no auth (localhost only). Required for non-localhost.
  tls:
    cert_path: ~                 # Path to TLS certificate (PEM)
    key_path: ~                  # Path to TLS private key (PEM)
```

### Config Struct

```rust
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct WebConfig {
    #[serde(default = "default_web_enabled")]
    pub enabled: bool,
    #[serde(default = "default_bind_address")]
    pub bind_address: String,
    #[serde(default = "default_port")]
    pub port: u16,
    pub api_key: Option<String>,
    pub tls: Option<TlsConfig>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TlsConfig {
    #[serde(deserialize_with = "deserialize_path")]
    pub cert_path: PathBuf,
    #[serde(deserialize_with = "deserialize_path")]
    pub key_path: PathBuf,
}

fn default_web_enabled() -> bool { true }
fn default_bind_address() -> String { "127.0.0.1".to_string() }
fn default_port() -> u16 { 9876 }
```

The `Config` struct gets a new optional field:
```rust
pub struct Config {
    // ... existing fields ...
    #[serde(default)]
    pub web: Option<WebConfig>,
}
```

### CLI Flags

No new subcommand is needed. The agent already has `start`/`stop`/`status`/`run`.
The web server starts automatically with the agent.

Optional override flags on `repoman agent start`:
```
repoman agent start --web-port 8080    # Override port
repoman agent start --no-web           # Disable web server
```

The dashboard URL is printed when the agent starts:
```
$ repoman agent start
Agent started (PID: 48291)
Web dashboard: http://127.0.0.1:9876
```

### Agent Status Enhancement

`repoman agent status` should show the web server URL:
```
Agent is running (PID: 48291)
Web dashboard: http://127.0.0.1:9876
Log file: ~/.repoman/logs/agent.log
```


## 10. Open Questions & Design Decisions

### Q1: Should the web server be part of the agent or a separate subcommand?

**Decision: Part of the agent.** See Section 2 for full rationale. The agent is
the natural home because (a) it is already a long-lived process, (b) it has
the data the web UI needs, and (c) a single process is simpler to manage.

The `--no-web` flag and `web.enabled: false` config provide opt-out.

### Q2: htmx vs SPA (React/Svelte)?

**Decision: htmx.** This is a developer tool dashboard, not a consumer web app.
htmx provides more than enough interactivity for the use cases (click buttons,
see updated data, receive live events). The advantages of avoiding a JS build
system, a node_modules directory, and client-side state management are enormous
for a Rust project.

If the dashboard ever needs complex client-side interactions (drag-and-drop repo
ordering, interactive graphs), a small Svelte component can be added alongside
htmx without replacing it.

### Q3: Embed assets or serve from disk?

**Decision: Embed in the binary via rust-embed.** Repoman is distributed as a
single binary. Users should not need to install or locate a `static/` directory.
Embedding adds ~50-100KB to the binary (htmx + CSS + templates), which is
negligible.

For development, rust-embed supports a `debug-embed = false` mode that serves
from disk, enabling live-reload during template development.

### Q4: How to handle concurrent access to vault/metadata?

**Decision: Same as today -- file locking.** The vault and metadata already use
`fs2::FileExt::lock_exclusive()` for writes. The web server reads through the
same `Vault::load()` and `Metadata::load()` paths. Reads are not locked (they
read the entire file atomically via `read_to_string`). Writes (sync, clone
creation) acquire exclusive locks.

This is safe because:
- Reads see a consistent snapshot (JSON files are written atomically).
- Writes are serialized by file locks.
- The agent loop and web handlers run on the same tokio runtime, so they
  alternate naturally (the agent loop yields during `tokio::time::sleep`,
  which is when web requests are most likely processed).

If contention becomes an issue (unlikely for a single-user tool), an in-memory
cache with a `tokio::sync::RwLock` could be added in `AppState`.

### Q5: What port number?

**Decision: 9876.** Arbitrary but chosen to avoid conflicts with common
development servers (3000, 5000, 8000, 8080, 8384). Fully configurable via
`web.port` in config.

### Q6: Should the web UI be able to edit config.yaml?

**Decision: No.** Config editing through a web UI is a complexity and security
trap. It requires validation, backup, atomic writes, and introduces the risk of
malformed config bricking the tool. The web UI displays the effective config
(read-only). Users edit `~/.config/repoman/config.yaml` with their editor.

### Q7: Sync history storage?

The current metadata stores only the most recent sync (`last_sync: SyncInfo`).
For a sync history timeline, metadata would need a `sync_history:
Vec<SyncInfo>` field with a bounded size (e.g., last 100 entries). This is a
small metadata schema change that should be implemented regardless of the web
dashboard, as it is useful for CLI reporting too.

### Q8: How should long-running operations (sync) report progress?

Sync operations can take seconds to minutes for large repos. The web UI should:
1. Return immediately with a "syncing..." status (htmx swaps a spinner).
2. Push progress via WebSocket (`sync_started`, `sync_completed` events).
3. On completion, the client fetches the updated repo fragment.

This requires the sync operation to run in a spawned tokio task rather than
blocking the HTTP handler. The broadcast channel carries the completion event.

### Q9: Mobile responsiveness?

The dashboard should be usable on mobile devices (checking repo status from a
phone). Pico CSS and Tailwind both handle responsive layouts well. The main
table might need horizontal scrolling or a card layout on narrow screens. This
is a CSS concern, not an architectural one.

### Q10: Should there be a `repoman web` command for standalone use?

**Deferred.** If users want a read-only web viewer without the agent's sync
loop, it could be added as `repoman web` or `repoman agent start --web-only`.
This is a minor code change (skip the sync loop, run only the HTTP server)
and can be added in a future version based on demand.
