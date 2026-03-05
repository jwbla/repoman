# repoman alias

Manage aliases for repository names.

## Synopsis

```
repoman alias                          # list all aliases
repoman alias <name> <alias>           # create alias
repoman alias <name> <alias> -r        # remove alias
```

## Description

Aliases are short names that map to canonical repository names. Once created, an alias can be used in place of the full name in any command (`clone`, `sync`, `status`, `update`, `open`, `remove`, etc.).

When called with no arguments, lists all defined aliases.

When called with a name and alias, creates a mapping from the alias to the repository name. The repository must already exist in the vault.

With `-r`, removes the specified alias.

Aliases are stored in `vault.json` alongside repository entries.

## Arguments

| Argument | Required | Description |
|----------|----------|-------------|
| `name` | No | The canonical repository name to alias. |
| `alias` | No | The alias to create (or remove with `-r`). |

## Flags

| Flag | Description |
|------|-------------|
| `-r, --remove` | Remove the alias instead of creating it. |

## Examples

Create an alias:

```sh
repoman alias my-long-repo-name mr
```

Now use it:

```sh
repoman clone mr
repoman sync mr
repoman status mr
cd $(repoman open mr)
```

List all aliases:

```sh
repoman alias
```

```
Aliases:
  mr -> my-long-repo-name
  lnx -> linux
```

Remove an alias:

```sh
repoman alias my-long-repo-name mr -r
```

## Tips

- Aliases are one-to-many: a single repo can have multiple aliases.
- When a repo is removed with `repoman remove`, all its aliases are automatically cleaned up.
- Alias names must not collide with existing repo names, though repoman does not enforce this at creation time. If an alias shadows a repo name, the repo name takes precedence in some lookup paths.
