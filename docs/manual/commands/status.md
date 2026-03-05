# repoman status

Show detailed status for a repository.

## Synopsis

```
repoman status <name> [--json]
```

## Description

Prints a detailed inspection of a repository, including:

- Remote URL
- Whether the pristine exists and its branches
- Latest tag tracked by the agent
- Last sync time and type (manual or auto)
- Sync interval
- List of clones with their current branch, dirty file count, and ahead/behind counts
- Alternates health check (warns if the pristine objects path referenced by clones is missing)

Aliases are resolved transparently.

## Arguments

| Argument | Required | Description |
|----------|----------|-------------|
| `name` | Yes | Repository name or alias. |

## Flags

| Flag | Description |
|------|-------------|
| `--json` | Output in JSON format instead of human-readable text. |

## Examples

Human-readable output:

```sh
repoman status my-repo
```

```
Repository: my-repo
  URL: https://github.com/user/my-repo.git
  Pristine: yes
  Branches: main, develop, release/2.0
  Latest tag: v2.1.0
  Last sync: 2026-02-15 10:30:00 UTC (manual)
  Sync interval: 3600s
  Clones (2):
    feature-auth on main (3 dirty) [+2/-0]
    hotfix on release/2.0
```

JSON output:

```sh
repoman status my-repo --json
```

## Tips

- The ahead/behind counts compare the clone's local branch to `origin/<branch>` (which points at the pristine). Run `repoman update` first to get fresh numbers.
- If alternates health check fails, it means a clone references a pristine objects directory that no longer exists. This typically happens when you destroy a pristine but leave its clones. The clones will malfunction. Destroy them or re-init the pristine.
- The `--json` flag is a global flag and can appear before or after the subcommand.
