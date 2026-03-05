# FUTURE.md

Comprehensive list of suggestions for repoman before and beyond v1.0.0.
Organized by priority and category.

---

## 1. Wiring Up Unconnected Features

These features have code written but are not fully connected to the runtime.

### 1.1 Config `repos` Fields Not Fully Consumed

Several `RepoConfig` fields are declared and parsed but not yet wired into
any operation:

| Field | Declared in | Status |
|-------|------------|--------|
| `build` | `config.rs` | Not used by any operation |
| `auto_init` | `config.rs` | Not used by any operation |
| `tags` | `config.rs` | Parsed but no CLI interface (see 3.3) |
| `clone_defaults.branch` | `config.rs` | Not read by clone_op.rs |

Fields that are now wired (as of v0.3.6): `sync_interval` (via
`effective_sync_interval()`), `auth` (via `effective_auth()`),
`default_branch` (via `effective_default_branch()`), `clone_defaults.shallow`
(read by `init_pristine()` for shallow clones).

**Fix:** Either wire each remaining field into the relevant operation or
remove the dead structs until they are needed.

### 1.2 Dashboard Missing Interactive Commands

The plan specified keybindings for `s` (sync), `u` (update), `i` (init),
`d` (destroy), `Enter` (detail view), `/` (search). Only `q`, `j/k`,
and arrow keys were implemented.

**Fix:** Implement the remaining keybindings or remove them from the title
bar text.

---

## 2. Architecture Improvements

### 2.1 AppContext / Runtime Struct

Operations repeatedly call `Vault::load(config)` and
`Metadata::load(name, config)` independently. A shared runtime context
could cache the vault and provide a cleaner API:

```rust
struct AppContext {
    config: Config,
    vault: Vault,        // loaded once
    plugins: PluginManager,
}
```

### 2.2 Clone-to-Repo Reverse Index

`destroy_clone()` does an O(n) scan of all repos to find which repo owns
a clone. With many repos this is slow. A reverse index (clone_name -> repo)
in the vault or a separate index file would make this O(1).

### 2.3 Consistent Data Format

Vault and metadata use JSON; config uses YAML; export uses YAML. Consider
standardizing on one format, or at least documenting the rationale.

### 2.4 Error Context with anyhow or error chains

Currently many errors lose context (e.g., `VaultLoadError(String)` discards
the original error type). Consider using `anyhow` for user-facing errors or
adding source chains to `RepomanError` variants.

---

## 3. New Feature Ideas

### 3.1 Worktree Support

Instead of cloning from pristine with alternates, support `git worktree`
for clones. Worktrees are a first-class git feature that provides the same
space savings with better tooling support. This would be opt-in per-repo
via config.

### 3.2 Template Clones

Allow defining clone templates that run a set of commands after cloning:

```yaml
repos:
  my-app:
    templates:
      dev:
        branch: develop
        post_clone: "npm ci && npm run dev"
      review:
        branch: main
        post_clone: "npm ci"
```

Usage: `repoman clone my-app --template dev`

### 3.3 Repo Groups / Tags

The `tags` field exists in RepoConfig but has no CLI interface. Add:

```
repoman list --tag frontend      # filter by tag
repoman sync --tag daily-driver  # sync only tagged repos
repoman init --tag work          # init all repos with tag
```

### 3.4 Bulk Operations via Piping

Support reading repo names from stdin for scriptability:

```bash
repoman list --names-only | grep "^lib-" | xargs -I{} repoman sync {}
```

Or natively: `repoman sync --filter "lib-*"`

### 3.5 `repoman watch` / Live Mode

Watch a pristine for upstream changes and notify immediately (using
polling or webhooks). Lighter than the full agent.

### 3.6 Diff Between Pristine States

Show what changed between the last two syncs of a pristine:

```
repoman diff neovim          # what changed since last sync
repoman diff neovim --tags   # diff between last two tags
```

### 3.7 Clone from Specific Tag or Commit

```
repoman clone neovim bugfix --tag v0.10.0
repoman clone neovim investigate --commit abc123
```

### 3.8 Export to Git Bundle

For offline transfer: `repoman export-bundle neovim > neovim.bundle`

### 3.9 Disk Usage Command

```
repoman du                    # total disk usage
repoman du neovim             # per-repo breakdown
repoman du --sort size        # sorted by size
```

### 3.10 MCP Server Mode

Expose repoman operations as MCP tools for LLM agent integration. This is
extensively described in `docs/MCP_IDEAS.md`. The key tools would be:
`vault_list`, `clone_create`, `clone_destroy`, `sync`, `status`, `open`.

---

## 4. Simplification Opportunities

### 4.1 Flatten the destroy_clone Logic

`operations/destroy.rs::destroy_clone()` has two separate code paths for
finding a clone (by suffix in metadata, and by directory name in clones/).
The second path (lines 59-108) is complex with nested Options and manual
string splitting. Consolidate into a single lookup that tries metadata
first, then falls back to filesystem.

### 4.2 Deduplicate Transfer Progress Callbacks

`init.rs` and `sync.rs` have nearly identical indicatif progress bar
closures (~30 lines each). Extract into a shared helper in `util.rs` or
in `credentials.rs`.

### 4.3 Reduce Clone Name Collision Risk

`generate_clone_suffix()` creates a 6-char random string. With 36^6 = ~2B
possibilities this is unlikely to collide, but checking for existence is
cheap. The function should loop until it finds a name that doesn't exist
in the clones directory.

### 4.4 Replace `HashMap<String, String>` Aliases with BiMap

Vault aliases are a `HashMap<String, String>` (alias -> canonical). Reverse
lookups (canonical -> all aliases) require scanning the entire map.
`bimap` crate would make both directions O(1).

### 4.5 Simplify Hook Runner Signatures

`hooks::run_hook()` takes 9 parameters. This is a code smell. Bundle the
contextual parameters into a `HookContext` struct (similar to what
`plugins.rs` already has):

```rust
pub struct HookContext<'a> {
    pub command: &'a str,
    pub event: &'a str,
    pub repo_name: &'a str,
    pub cwd: &'a Path,
    pub pristine_path: Option<&'a Path>,
    pub clone_path: Option<&'a Path>,
    pub clone_name: Option<&'a str>,
    pub new_tag: Option<&'a str>,
    pub fail_on_error: bool,
}
```

This also aligns the shell hook and plugin hook interfaces.

---

## 5. v1.0.0 Blockers (Suggested)

Before calling this v1.0.0, I'd recommend addressing:

1. **Wire up or remove unused config fields** (1.1) — dead config is confusing
2. **Dashboard keybindings** (1.2) — advertised but non-functional

Everything else is nice-to-have for v1.x.y and beyond.

---

## Priority Summary

| Priority | Category | Items |
|----------|----------|-------|
| **Should fix** | Wiring | 1.1, 1.2 |
| **Should fix** | Architecture | 2.1-2.4 |
| **Nice to have** | Features | 3.1-3.10 |
| **Nice to have** | Simplification | 4.1-4.5 |
