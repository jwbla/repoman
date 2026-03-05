# repoman sync

Fetch latest changes from origin into pristine(s).

## Synopsis

```
repoman sync [<pristine>]
```

## Description

Fetches all branches and tags from the remote origin into the specified pristine. This updates the local bare reference clone to match the remote state.

If `<pristine>` is provided, only that repository is synced. If omitted, all repositories with existing pristines are synced in parallel.

After fetching, repoman updates the sync timestamp in metadata and runs the `post_sync` hook if configured. See [Hooks](../hooks.md).

Sync only updates the pristine (bare repo). It does not touch working copy clones. To also fast-forward clones, use `repoman update` instead.

## Arguments

| Argument | Required | Description |
|----------|----------|-------------|
| `pristine` | No | Name of the pristine to sync. Omit to sync all. |

## Examples

Sync a single repo:

```sh
repoman sync my-repo
```

Sync all repos with pristines:

```sh
repoman sync
```

## Tips

- If the pristine does not exist, sync returns an error. Run `repoman init` first.
- Parallel sync reports a summary of successes and failures at the end.
- The background agent (`repoman agent start`) calls sync automatically based on each repo's `sync_interval`. See [Agent](agent.md).
- Use `repoman update` if you also want clones to be fast-forwarded after the sync.
