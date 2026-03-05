# repoman destroy

Remove clones or pristines from disk.

## Synopsis

```
repoman destroy <target>
repoman destroy --all-clones <name>
repoman destroy --all-pristines
repoman destroy --stale <days>
```

## Description

Removes clones or pristines from the filesystem. The vault entry is always preserved (use `repoman remove` to fully unregister a repo).

**Single target:** If `<target>` matches a vault repo name with an existing pristine, the pristine is destroyed. If it matches a clone directory name or clone suffix, the clone is destroyed. Metadata is updated to reflect the removal.

**All clones for a repo:** `--all-clones <name>` removes every clone belonging to the named pristine.

**All pristines:** `--all-pristines` removes all pristine directories across the entire vault. Vault entries and metadata are preserved so you can re-init later.

**Stale clones:** `--stale <days>` finds clones whose HEAD commit is older than the specified number of days and destroys them.

Lifecycle hooks `pre_destroy` and `post_destroy` fire around clone destruction if configured. See [Hooks](../hooks.md).

You must provide exactly one of: a target, `--all-clones`, `--all-pristines`, or `--stale`.

## Arguments

| Argument | Required | Description |
|----------|----------|-------------|
| `target` | No | Name of a clone or pristine to destroy. |

## Flags

| Flag | Description |
|------|-------------|
| `--all-clones <name>` | Destroy all clones for the named pristine. |
| `--all-pristines` | Destroy all pristines (keeps vault entries). |
| `--stale <days>` | Destroy clones whose HEAD commit is older than N days. |

## Examples

Destroy a specific clone:

```sh
repoman destroy my-repo-feature-auth
```

Destroy a pristine (keeps vault entry):

```sh
repoman destroy my-repo
```

Destroy all clones for a repo:

```sh
repoman destroy --all-clones my-repo
```

Destroy all pristines:

```sh
repoman destroy --all-pristines
```

Clean up old clones:

```sh
repoman destroy --stale 14
```

## Tips

- Destroying a pristine while clones still reference it will break those clones (they depend on the pristine's objects via alternates). Destroy clones first, or use `repoman remove` to clean up everything at once.
- `--stale` is based on the HEAD commit date, not the clone creation date. A clone on an old branch will be considered stale even if recently created.
- For a dry-run preview of what would be cleaned up, use `repoman gc --dry-run --days <N>` instead.
