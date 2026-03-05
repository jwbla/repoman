# repoman rename

Rename a vault entry and all associated data.

## Synopsis

```
repoman rename <old> <new>
```

## Description

Renames a repository in the vault. This updates the vault entry, moves the metadata directory, renames the pristine directory, and retargets any aliases that pointed to the old name.

The operation performs these steps in order:

1. Resolves `<old>` through aliases to find the canonical name.
2. Verifies the canonical name exists in the vault.
3. Verifies `<new>` does not already exist in the vault.
4. Saves existing metadata under the new name.
5. Removes the old metadata directory (`~/.repoman/vault/<old>/`).
6. Renames the pristine directory (`~/.repoman/pristines/<old>/` to `~/.repoman/pristines/<new>/`).
7. Updates the vault entry's name field.
8. Retargets all aliases that pointed to `<old>` so they now point to `<new>`.
9. Saves the updated vault.

Existing clones are **not** renamed or moved on disk. Their paths in metadata remain as they were. Clone directories under `~/.repoman/clones/` still use the old name prefix. This is intentional -- clones are disposable and can be recreated.

## Arguments

| Argument | Required | Description |
|----------|----------|-------------|
| `old` | Yes | Current repository name (or an alias that resolves to it). |
| `new` | Yes | New name for the repository. |

## Examples

Rename a repository:

```sh
repoman rename my-old-name my-new-name
```

```
Renamed 'my-old-name' to 'my-new-name'
```

Rename using an alias:

```sh
repoman alias my-repo mr
repoman rename mr better-name
```

```
Renamed 'my-repo' to 'better-name'
```

The alias `mr` now points to `better-name`.

## Tips

- If the new name conflicts with an existing vault entry, the operation fails. Remove or rename the conflicting entry first.
- Aliases are preserved and retargeted automatically. You do not need to recreate them.
- Existing clones continue to work after a rename because they reference the pristine by path (which is updated), not by name. However, clone directory names on disk will still reflect the old name. Destroy and recreate clones if you want matching names.
