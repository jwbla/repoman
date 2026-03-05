# repoman remove

Fully unregister a repository and delete all its data.

## Synopsis

```
repoman remove <name>
```

## Description

Completely removes a repository from repoman. This deletes:

1. All clones belonging to the repository
2. The pristine directory
3. The metadata directory (`~/.repoman/vault/<name>/`)
4. All aliases pointing to the repository
5. The vault entry itself

This is a destructive, irreversible operation. After removal, the repository is no longer known to repoman.

Aliases are resolved transparently, so you can remove a repo using any of its aliases.

Removal is best-effort for filesystem operations: if a clone or pristine directory cannot be deleted (e.g., permissions), a warning is printed but the vault entry is still removed.

## Arguments

| Argument | Required | Description |
|----------|----------|-------------|
| `name` | Yes | Repository name or alias. |

## Examples

Remove a repo by name:

```sh
repoman remove my-repo
```

Remove using an alias:

```sh
repoman remove mr
```

## Tips

- If you only want to free disk space without losing the vault entry, use `repoman destroy` instead. That preserves the vault entry so you can re-init later.
- There is no confirmation prompt. The command executes immediately.
- If the repo has no pristine or clones on disk (e.g., it was only added but never initialized), the command still succeeds and cleans up the vault entry and metadata.
