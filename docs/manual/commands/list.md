# repoman list

List all repositories in the vault.

## Synopsis

```
repoman list [-v] [--json]
```

## Description

Displays all repositories registered in the vault.

In default mode, a summary table is printed with columns: name, pristine status, clone count, and last sync time.

In verbose mode (`-v`), each repository is shown with full details including URL, add date, pristine path, branches, tags, and individual clone entries.

## Flags

| Flag | Description |
|------|-------------|
| `-v, --verbose` | Show detailed per-repo information instead of the summary table. |
| `--json` | Output in JSON format. |

## Examples

Summary table:

```sh
repoman list
```

```
NAME                 PRISTINE     CLONES   LAST SYNC
----------------------------------------------------------------
my-repo              yes          2        2026-02-15 10:30
other-repo           no           0        never
```

Verbose output:

```sh
repoman list -v
```

JSON output (useful for scripting):

```sh
repoman list --json
```

## Tips

- Repos appear in the order they were added.
- Names longer than 18 characters are truncated with `...` in the summary table. Use `-v` or `--json` to see full names.
- The `--json` flag is a global flag and can appear before or after the subcommand.
