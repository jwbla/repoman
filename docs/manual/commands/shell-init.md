# repoman shell-init

Output shell completions and a wrapper function for eval.

## Synopsis

```
repoman shell-init <shell>
```

## Description

Outputs a shell script that sets up both tab completions and a shell wrapper function. The wrapper intercepts `repoman open` to automatically `cd` into the selected directory, instead of just printing the path.

Use this instead of `repoman completions` if you want the `cd` integration.

## Arguments

| Argument | Required | Description |
|----------|----------|-------------|
| `shell` | Yes | One of: `bash`, `zsh`, `fish`, `elvish`, `powershell`. |

## Examples

### Zsh

Add to `~/.zshrc`:

```sh
eval "$(repoman shell-init zsh)"
```

### Bash

Add to `~/.bashrc`:

```sh
eval "$(repoman shell-init bash)"
```

### Fish

Add to `~/.config/fish/config.fish`:

```sh
repoman shell-init fish | source
```

## Tips

- The wrapper function intercepts `repoman open` so that it `cd`s into the path automatically. All other commands pass through to the real `repoman` binary.
- You only need one of `shell-init` or `completions` -- `shell-init` includes completions plus the wrapper.
