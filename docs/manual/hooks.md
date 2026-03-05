# Lifecycle Hooks

Hooks are shell commands that run at defined points in repoman's workflow. They let you automate tasks like installing dependencies after a clone, running builds after a sync, or backing up state before a destroy.

## Configuration

Hooks are defined in `~/.config/repoman/config.yaml` under `repos.<name>.hooks`:

```yaml
repos:
  my-app:
    hooks:
      post_clone: "npm ci && npm run build"
      post_sync: "./scripts/deploy.sh"
      post_sync_on_new_tag: "./scripts/release-notify.sh"
```

Each hook value is a shell command string. It is executed via `sh -c "<command>"`, so standard shell syntax (pipes, `&&`, variable expansion) works.

## Hook Points

There are 7 hook points, each firing at a specific moment in a command's lifecycle.

### post_init_pristine

**When:** After a pristine is successfully created by `repoman init`.
**Working directory:** The pristine directory.
**Failure behavior:** Fatal. If the hook fails, the init operation is considered failed.

### pre_clone

**When:** Before a clone is created by `repoman clone`, after the pristine has been verified/auto-initialized.
**Working directory:** The pristine directory.
**Failure behavior:** Fatal. If the hook fails, the clone is not created.

### post_clone

**When:** After a clone is created and metadata is saved.
**Working directory:** The clone directory.
**Failure behavior:** Fatal. If the hook fails, the clone command reports an error (but the clone directory remains on disk).

### post_sync

**When:** After a pristine is synced by `repoman sync` or `repoman update`.
**Working directory:** The pristine directory.
**Failure behavior:** Non-fatal. A warning is logged but the sync is considered successful.

### post_sync_on_new_tag

**When:** After a sync when the background agent detects a new tag on the remote. Only runs during agent-initiated syncs, not manual `repoman sync`.
**Working directory:** The pristine directory.
**Failure behavior:** Non-fatal. A warning is logged.

### pre_destroy

**When:** Before a clone is removed by `repoman destroy` or `repoman gc`.
**Working directory:** The clone directory (while it still exists).
**Failure behavior:** Non-fatal. A warning is logged but destruction proceeds.

### post_destroy

**When:** After a clone has been removed from disk.
**Working directory:** The clones directory (parent).
**Failure behavior:** Non-fatal. A warning is logged.

## Environment Variables

Every hook receives the following environment variables:

| Variable | Always set | Description |
|----------|-----------|-------------|
| `REPOMAN_REPO` | Yes | The canonical repository name |
| `REPOMAN_EVENT` | Yes | The hook event name (e.g., `post_clone`) |
| `REPOMAN_PRISTINE_PATH` | When applicable | Absolute path to the pristine directory |
| `REPOMAN_CLONE_PATH` | Clone hooks only | Absolute path to the clone directory |
| `REPOMAN_CLONE_NAME` | Clone hooks only | The full clone directory name |
| `REPOMAN_NEW_TAG` | `post_sync_on_new_tag` only | The new tag name |

## Which Hooks Get Which Variables

| Hook | REPO | EVENT | PRISTINE_PATH | CLONE_PATH | CLONE_NAME | NEW_TAG |
|------|------|-------|---------------|------------|------------|---------|
| post_init_pristine | Yes | Yes | Yes | -- | -- | -- |
| pre_clone | Yes | Yes | Yes | -- | -- | -- |
| post_clone | Yes | Yes | Yes | Yes | Yes | -- |
| post_sync | Yes | Yes | Yes | -- | -- | -- |
| post_sync_on_new_tag | Yes | Yes | Yes | -- | -- | Yes |
| pre_destroy | Yes | Yes | Yes | Yes | Yes | -- |
| post_destroy | Yes | Yes | -- | -- | -- | -- |

## Examples

### Install dependencies after clone

```yaml
repos:
  my-node-app:
    hooks:
      post_clone: "npm ci"
```

### Build after sync

```yaml
repos:
  my-rust-project:
    hooks:
      post_sync: "cargo build --release"
```

### Notify on new release tag

```yaml
repos:
  upstream-lib:
    hooks:
      post_sync_on_new_tag: |
        echo "New release: $REPOMAN_NEW_TAG" | mail -s "Release alert" team@example.com
```

### Backup state before destroying a clone

```yaml
repos:
  important-project:
    hooks:
      pre_destroy: "tar czf /tmp/backup-$REPOMAN_CLONE_NAME.tar.gz ."
```

### Chain multiple commands

```yaml
repos:
  full-stack-app:
    hooks:
      post_clone: "npm ci && npm run build && npm test"
```

## Tips

- Hooks are per-repo. There is no global hook that applies to all repos. If you want the same hook for many repos, define it on each one or use a [Lua plugin](plugins.md) which fires on all repos.
- Hook commands run in a subshell (`sh -c`), so they inherit repoman's environment plus the `REPOMAN_*` variables.
- Fatal hooks (post_init_pristine, pre_clone, post_clone) stop the operation on failure. Non-fatal hooks (post_sync, pre_destroy, post_destroy) log a warning and continue.
- Use `--debug` to see hook execution details in the console output.
