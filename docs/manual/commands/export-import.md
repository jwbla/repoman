# repoman export / repoman import

Export and import vault data as YAML.

## Synopsis

```
repoman export
repoman import <path>
```

## Description

### export

Outputs the vault contents as a YAML document to stdout. The export includes each repository's name, URL, and aliases. It does not include pristines, clones, or metadata -- only the information needed to recreate the vault on another machine.

### import

Reads a YAML file at `<path>` and adds its repositories to the vault. Repositories that already exist in the vault are skipped with a message. Aliases defined in the import file are also created.

Import creates metadata for each new repository but does not initialize pristines. Run `repoman init` after importing to create them.

## YAML Format

```yaml
repositories:
  - name: my-repo
    url: https://github.com/user/my-repo.git
    aliases:
      - mr
  - name: other-repo
    url: git@github.com:user/other-repo.git
```

The `aliases` field is optional and may be omitted.

## Arguments

### export

No arguments. Output goes to stdout.

### import

| Argument | Required | Description |
|----------|----------|-------------|
| `path` | Yes | Path to the YAML file to import. |

## Examples

Export vault to a file:

```sh
repoman export > my-repos.yaml
```

Import on another machine:

```sh
repoman import my-repos.yaml
```

Round-trip:

```sh
repoman export > backup.yaml
# ... later, on a fresh install ...
repoman import backup.yaml
repoman init
```

Pipe between machines:

```sh
ssh workstation repoman export | repoman import /dev/stdin
```

## Tips

- Export/import is useful for backing up your vault, sharing repo lists with teammates, or migrating to a new machine.
- Duplicates are silently skipped during import, so it is safe to import the same file multiple times.
- The YAML format is simple enough to hand-edit. You can create an import file manually without ever running export.
