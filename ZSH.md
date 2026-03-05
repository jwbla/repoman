# Repoman shell setup (zsh)

Add to your `~/.zshrc`:

```zsh
eval "$(repoman shell-init zsh)"
```

This gives you:
- Tab completion for all repoman subcommands and arguments
- `repoman open <target>` will `cd` into the directory instead of printing the path
