# repoman gc

Garbage-collect stale clones and compact pristines.

## Synopsis

```
repoman gc [--days <N>] [--dry-run] [-y/--yes]
```

## Description

Performs two cleanup tasks:

1. **Stale clone removal:** Finds clones whose HEAD commit is older than `--days` (default 30) and removes them from disk. Metadata is updated to reflect the removal.

2. **Pristine compaction:** Runs `git gc --auto` inside each pristine directory to let git decide whether to repack objects and prune unreachable data.

## Flags

| Flag | Default | Description |
|------|---------|-------------|
| `--days <N>` | `30` | Threshold in days. Clones with HEAD commits older than this are considered stale. |
| `--dry-run` | Off | Show what would be done without making any changes. |
| `-y` / `--yes` | Off | Skip the confirmation prompt (also inherited from global `-y`). |

## Examples

Default GC (30-day threshold, will preview and ask for confirmation):

```sh
repoman gc
```

Skip confirmation:

```sh
repoman gc -y
```

Set a custom threshold:

```sh
repoman gc --days 7
```

Preview without deleting:

```sh
repoman gc --dry-run
```

```
[dry-run] Stale clones (2):
  my-repo-abc123 (my-repo) -- 45 days old
  other-repo-xyz (other-repo) -- 31 days old
[dry-run] Pristines GC'd: 3
```

## Tips

- The age is measured from the HEAD commit date, not the clone creation date. A freshly created clone of an old branch could be flagged as stale.
- `--dry-run` is useful for understanding what GC would do before committing. Combine with `--days` to experiment with different thresholds.
- Pristine GC uses `git gc --auto`, which only repacks when git's heuristics determine it is worthwhile. It is safe to run frequently.
- For more targeted cleanup, use `repoman destroy --stale <days>` (which removes stale clones but does not run pristine GC).
