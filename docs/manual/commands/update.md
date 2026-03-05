# repoman update

Sync a pristine from origin and fast-forward all its clones.

## Synopsis

```
repoman update [<name>]
```

## Description

Performs two steps in sequence:

1. Syncs the pristine from the remote origin (same as `repoman sync`).
2. For each clone of that repository, fetches from the local pristine and attempts a fast-forward merge on the current branch.

If `<name>` is omitted, all repositories with pristines are updated in parallel.

For each clone, the outcome is one of:

- **Fast-forwarded** -- the clone's branch was behind and has been moved forward.
- **Already up-to-date** -- the clone is at the same commit as the pristine.
- **Diverged** -- the clone has local commits that are not in the pristine. A manual merge or rebase is required.

Aliases are resolved transparently.

## Arguments

| Argument | Required | Description |
|----------|----------|-------------|
| `name` | No | Repository name or alias. Omit to update all. |

## Examples

Update a single repo and all its clones:

```sh
repoman update my-repo
```

Update everything:

```sh
repoman update
```

## Tips

- This is the preferred command for a daily "pull everything" workflow. It combines `sync` and clone fast-forward in one step.
- Clones on detached HEAD or branches without a remote tracking branch are skipped.
- If a clone has diverged, repoman will not modify it. You can manually rebase or merge inside the clone directory.
- The background agent performs a similar heartbeat update on its own schedule.
