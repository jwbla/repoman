# repoman init

Create pristine bare clone(s) of vaulted repositories.

## Synopsis

```
repoman init [<vault_name>] [--depth <N>]
```

## Description

Creates a bare reference clone (called a "pristine") for a repository that has been added to the vault. The pristine is stored under `~/.repoman/pristines/<name>/` and serves as the local source for all working copies.

If `<vault_name>` is provided, only that repository is initialized. If omitted, all vaulted repositories that do not yet have pristines are initialized in parallel.

Pristines are bare repos -- they contain git objects and refs but no working tree. This makes them compact and fast to sync.

If a `post_init_pristine` hook is configured for the repo, it runs after the pristine is created. See [Hooks](../hooks.md).

## Arguments

| Argument | Required | Description |
|----------|----------|-------------|
| `vault_name` | No | Name of the repository to initialize. Omit to initialize all. |

## Flags

| Flag | Default | Description |
|------|---------|-------------|
| `--depth <N>` | Full clone | Create a shallow pristine with only `N` commits of history. Useful for large repositories where full history is not needed. |

## Examples

Initialize a single repo:

```sh
repoman init my-repo
```

Initialize all repos that lack pristines:

```sh
repoman init
```

Shallow clone with limited history:

```sh
repoman init my-repo --depth 1
```

## Tips

- If the pristine already exists, repoman returns an error. Use `repoman sync` to update an existing pristine.
- SSH authentication uses the ssh-agent by default. If your key is not loaded, run `ssh-add` first. See the auth error message for detailed setup instructions.
- You can skip `init` entirely -- `repoman clone` will auto-init the pristine if it is missing (lazy initialization).
- A progress bar is displayed during the clone showing receiving and indexing phases.
- If `--depth` is not specified but `clone_defaults.shallow: true` is set in the repo's config, depth 1 is used automatically.
