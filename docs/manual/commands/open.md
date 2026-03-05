# repoman open

Print the filesystem path for a pristine or clone.

## Synopsis

```
repoman open <target>
```

## Description

Resolves a target name to its filesystem path and prints it to stdout. This is designed for use with shell command substitution to navigate to repo directories.

The search order is:

1. **Pristine names** -- if `<target>` matches a repo name (or alias) and the pristine directory exists, its path is returned.
2. **Clone suffixes** -- if `<target>` matches a clone suffix in any repo's metadata, that clone's path is returned.
3. **Full clone directory names** -- if `<target>` matches a directory name under the clones directory, its path is returned.

Aliases are resolved transparently at step 1.

## Arguments

| Argument | Required | Description |
|----------|----------|-------------|
| `target` | Yes | Pristine name, clone suffix, or full clone directory name. |

## Examples

Navigate to a pristine:

```sh
cd $(repoman open my-repo)
```

Navigate to a clone by its suffix:

```sh
cd $(repoman open feature-auth)
```

Navigate to a clone by its full directory name:

```sh
cd $(repoman open my-repo-feature-auth)
```

Use with an alias:

```sh
repoman alias my-repo mr
cd $(repoman open mr)
```

## Tips

- Only the path is printed to stdout, with no extra formatting. This makes it safe for command substitution.
- If the target cannot be found as a pristine, clone suffix, or clone directory, the command exits with an error.
- Consider adding a shell function to your profile for convenience:

```sh
# ~/.bashrc or ~/.zshrc
rcd() { cd "$(repoman open "$1")" || return 1; }
```

Then: `rcd my-repo`
