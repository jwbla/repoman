# repoman completions

Generate shell completion scripts.

## Synopsis

```
repoman completions <shell>
```

## Description

Outputs a shell completion script to stdout for the specified shell. Redirect the output to the appropriate file for your shell to enable tab completion.

## Arguments

| Argument | Required | Description |
|----------|----------|-------------|
| `shell` | Yes | One of: `bash`, `zsh`, `fish`, `elvish`, `powershell`. |

## Examples

### Bash

```sh
repoman completions bash > ~/.local/share/bash-completion/completions/repoman
```

Or for system-wide:

```sh
repoman completions bash | sudo tee /etc/bash_completion.d/repoman > /dev/null
```

### Zsh

```sh
repoman completions zsh > ~/.zfunc/_repoman
```

Make sure `~/.zfunc` is in your `fpath` (add `fpath=(~/.zfunc $fpath)` to `~/.zshrc` before `compinit`).

### Fish

```sh
repoman completions fish > ~/.config/fish/completions/repoman.fish
```

### Elvish

```sh
repoman completions elvish > ~/.config/elvish/lib/repoman.elv
```

### PowerShell

```sh
repoman completions powershell >> $PROFILE
```

## Tips

- You only need to regenerate completions after upgrading repoman if new subcommands or flags were added.
- Restart your shell or source the completion file for changes to take effect.
