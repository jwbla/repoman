# repoman add

Add a repository to the vault.

## Synopsis

```
repoman add [<url>]
```

## Description

Registers a git repository URL in the vault. The vault is repoman's central registry of known repositories stored at `~/.repoman/vault/vault.json`.

If `<url>` is provided, it is used directly. If omitted, repoman detects the remote URL(s) from the git repository in the current working directory.

When detecting from the current directory:

1. The current branch's configured remote is checked first.
2. Then `remote.pushDefault` from git config.
3. Then `origin` if it exists.
4. Finally, the first remote alphabetically.

If multiple remotes are found, all are recorded in metadata with the default remote listed first.

After adding, the repo appears in `repoman list` but has no pristine yet. Run `repoman init` to create one.

## Arguments

| Argument | Required | Description |
|----------|----------|-------------|
| `url` | No | Git URL (HTTPS, SSH, or local path). If omitted, auto-detect from current directory. |

## Examples

Add by URL:

```sh
repoman add https://github.com/user/my-repo.git
repoman add git@github.com:user/my-repo.git
```

Auto-detect from current directory:

```sh
cd ~/projects/my-repo
repoman add
```

## Tips

- The repository name is extracted from the URL (last path component, minus `.git` suffix). For `https://github.com/user/my-repo.git`, the name is `my-repo`.
- Adding a duplicate name returns an error. If you need two repos with the same base name from different sources, rename one using `repoman alias`.
- Adding a repo does not clone anything. Use `repoman init` next, or `repoman clone` which auto-inits if needed.
