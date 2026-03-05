# repoman clone

Create a lightweight working copy from a pristine.

## Synopsis

```
repoman clone <pristine> [<clone_name>] [-b <branch>]
```

## Description

Creates a new working copy (clone) from a pristine bare repository. The clone uses git's alternates mechanism to share object storage with the pristine, keeping disk usage minimal.

The clone is placed at `~/.repoman/clones/<pristine>-<clone_name>/`. If `<clone_name>` is omitted, a random 6-character alphanumeric suffix is generated.

If the pristine does not exist yet, repoman automatically initializes it from the vault (lazy init) before creating the clone.

Lifecycle hooks `pre_clone` and `post_clone` fire before and after clone creation if configured. See [Hooks](../hooks.md).

## Arguments

| Argument | Required | Description |
|----------|----------|-------------|
| `pristine` | Yes | Name of the pristine to clone from (the repo name in the vault). |
| `clone_name` | No | Suffix for the clone directory. Defaults to a random string. |

## Flags

| Flag | Description |
|------|-------------|
| `-b, --branch <branch>` | Check out this branch instead of the pristine's HEAD. The branch must exist in the pristine. |

## Examples

Create a clone with a random name:

```sh
repoman clone my-repo
# -> ~/.repoman/clones/my-repo-a3kx9f/
```

Create a named clone:

```sh
repoman clone my-repo feature-auth
# -> ~/.repoman/clones/my-repo-feature-auth/
```

Clone with a specific branch:

```sh
repoman clone my-repo hotfix -b release/2.0
```

Jump into the clone directory:

```sh
cd $(repoman open my-repo-feature-auth)
```

## Tips

- Clones share git objects with the pristine via the alternates file at `.git/objects/info/alternates`. Do not delete the pristine while clones reference it, or they will lose access to their objects. Use `repoman status` to check alternates health.
- If the branch specified with `-b` does not exist in the pristine, the command fails with an error. Run `repoman sync` first to fetch new branches from the remote.
- Clone names must be unique. Attempting to create a clone with a name that already exists returns an error.
