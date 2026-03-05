# repoman refresh

Init missing pristines and sync existing ones in one pass.

## Synopsis

```
repoman refresh
```

## Description

Combines `repoman init` and `repoman sync` into a single parallel operation. Scans the vault for repositories that lack pristines and initializes them, while simultaneously syncing all existing pristines from their remotes.

This is the fastest way to bring all repositories up to date after a fresh install or after adding multiple repos to the vault.

## Examples

Refresh everything:

```sh
repoman refresh
```

Output:

```
Refreshing: 3 to init, 12 to sync...

Refresh complete: init 3/3, sync 12/12
```

## Tips

- Concurrency is bounded by the `max_parallel` config setting (default 8).
- Failed operations are reported individually but do not stop the rest of the batch.
- This is equivalent to running `repoman init && repoman sync` but faster because both phases run in parallel.
