# SSH Authentication for Repoman

Repoman uses your existing SSH configuration for authenticating with remote git repositories. If your SSH key has a passphrase, you'll need to ensure it's available to the SSH agent.

## The Problem

When using a passphrase-protected SSH key, each git operation (init, sync, agent polling) would normally prompt for your passphrase. This is inconvenient and doesn't work well for background operations like the agent.

## Solutions

### Option 1: SSH Agent (Per Terminal Session)

Load your key once per terminal session:

```bash
eval "$(ssh-agent -s)"
ssh-add ~/.ssh/id_ed25519
```

Add to your `~/.bashrc` or `~/.zshrc` to start the agent automatically:

```bash
if [ -z "$SSH_AUTH_SOCK" ]; then
    eval "$(ssh-agent -s)"
fi
```

**Drawback**: You still need to run `ssh-add` and enter your passphrase once per terminal.

### Option 2: Keychain (Recommended)

[Keychain](https://www.funtoo.org/Keychain) manages ssh-agent and gpg-agent across sessions. You only enter your passphrase once per boot.

**Install:**
```bash
# Arch Linux
sudo pacman -S keychain

# Debian/Ubuntu
sudo apt install keychain

# Fedora
sudo dnf install keychain

# macOS
brew install keychain
```

**Configure** (add to `~/.bashrc` or `~/.zshrc`):
```bash
eval $(keychain --eval --quiet id_ed25519)
```

When you open your first terminal after boot, keychain prompts for your passphrase once. All subsequent terminals and background processes use the cached key.

### Option 3: GNOME Keyring (Desktop Integration)

If you use GNOME, gnome-keyring can automatically unlock your SSH key when you log in.

```bash
# Arch Linux
sudo pacman -S gnome-keyring seahorse

# Enable SSH component
# Add to ~/.pam_environment or configure via settings
```

Your SSH key passphrase is unlocked automatically when you log into your desktop session.

### Option 4: KDE Wallet (KWallet)

For KDE Plasma users:

```bash
sudo pacman -S ksshaskpass
```

Configure SSH to use ksshaskpass for passphrase prompts, and KWallet will cache them.

### Option 5: Systemd User Service

Create a persistent ssh-agent as a systemd user service:

```bash
# ~/.config/systemd/user/ssh-agent.service
[Unit]
Description=SSH key agent

[Service]
Type=simple
Environment=SSH_AUTH_SOCK=%t/ssh-agent.socket
ExecStart=/usr/bin/ssh-agent -D -a $SSH_AUTH_SOCK

[Install]
WantedBy=default.target
```

Enable and start:
```bash
systemctl --user enable ssh-agent
systemctl --user start ssh-agent
```

Add to your shell config:
```bash
export SSH_AUTH_SOCK="$XDG_RUNTIME_DIR/ssh-agent.socket"
```

### Option 6: Passwordless SSH Key (Less Secure)

Generate a new key without a passphrase:

```bash
ssh-keygen -t ed25519 -f ~/.ssh/id_ed25519_repoman -N ""
```

Add to your git host (GitHub, GitLab, etc.) and configure repoman to use it.

**Note**: This is less secure as anyone with access to your machine can use the key.

## For the Repoman Agent

The repoman agent runs in the background and needs non-interactive access to your SSH keys. Recommended setup:

1. Use **keychain** and ensure it's initialized before starting the agent
2. Or use **GNOME Keyring/KWallet** with desktop session integration
3. Or use a **dedicated passwordless key** for repoman only

## HTTPS Alternative

For repositories using HTTPS instead of SSH:

```bash
# Cache credentials for 1 hour
git config --global credential.helper 'cache --timeout=3600'

# Or store permanently (less secure)
git config --global credential.helper store
```

## Troubleshooting

### "SSH authentication failed" error

1. Check if your key is loaded: `ssh-add -l`
2. If empty, load your key: `ssh-add ~/.ssh/id_ed25519`
3. Test SSH access: `ssh -T git@github.com`

### Agent can't authenticate

Ensure the agent inherits your SSH_AUTH_SOCK environment:

```bash
# Check current value
echo $SSH_AUTH_SOCK

# The agent needs this same value when it starts
```

## TODO

- [ ] Investigate per-repo SSH key configuration in metadata
- [ ] Add support for SSH key path override via config
- [ ] Consider integrating with system keyring directly
