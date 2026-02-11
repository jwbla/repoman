# Installing Repoman

## Quick Install

### From GitHub Releases (Tagged Versions)

Download a prebuilt artifact from GitHub Releases:
- Linux/macOS: `repoman-<OS>-x86_64-<tag>.tar.gz`
- Windows: `repoman-windows-x86_64-<tag>.zip` or `.msi` installer

### From GitHub Actions CI (Any Successful Run)

Use this when you want a build from a commit that is not yet tagged:
1. Open a successful CI run in GitHub Actions
2. Download the artifact for your OS
3. Extract and install:
   - Linux/macOS: unpack and move `repoman` into a PATH directory
   - Windows: run the `.msi` installer or use the `.zip` binary

### From Source (Local)

```bash
cargo install --path .
```

This builds a release binary and installs it to `~/.cargo/bin/repoman`.

Ensure `~/.cargo/bin` is in your PATH (usually added by rustup automatically):
```bash
echo $PATH | grep -q ".cargo/bin" || echo 'export PATH="$HOME/.cargo/bin:$PATH"' >> ~/.bashrc
```

Verify:
```bash
repoman --version
```

### Update/Reinstall

```bash
cargo install --path . --force
```

### Uninstall

```bash
cargo uninstall repoman
```

## Other Installation Methods

### From crates.io (Future)

Once published:
```bash
cargo install repoman
```

### Manual Binary Install

If you prefer not to use cargo install:

```bash
# Build release binary
cargo build --release

# Copy to a location in your PATH
cp target/release/repoman ~/.local/bin/
# or system-wide:
sudo cp target/release/repoman /usr/local/bin/
```

### Development (Symlink)

For active development, symlink so changes are immediately available:

```bash
cargo build --release
ln -sf "$(pwd)/target/release/repoman" ~/.local/bin/repoman
```

## Build Options

### Standard Release
```bash
cargo build --release
```

### Optimized for Your CPU
```bash
RUSTFLAGS="-C target-cpu=native" cargo build --release
```

### Static Binary (Portable)
```bash
rustup target add x86_64-unknown-linux-musl
cargo build --release --target x86_64-unknown-linux-musl
```

## Dependencies

Building requires:
- Rust 1.70+ (install via [rustup](https://rustup.rs))
- OpenSSL development headers
- libssh2 development headers

### Arch Linux
```bash
sudo pacman -S rust openssl libssh2
```

### Debian/Ubuntu
```bash
sudo apt install rustc cargo libssl-dev libssh2-1-dev pkg-config
```

### Fedora
```bash
sudo dnf install rust cargo openssl-devel libssh2-devel
```

### macOS
```bash
brew install rust openssl libssh2
```

## Post-Install Setup

### 1. Create directories
```bash
repoman list  # Creates ~/.repoman/ structure
```

### 2. SSH Authentication

If using SSH remotes, ensure your key is available:
```bash
ssh-add ~/.ssh/id_ed25519
```

See [docs/ssh-authentication.md](docs/ssh-authentication.md) for persistent solutions.

### 3. Optional Configuration

Create `~/.config/repoman/config.yaml` to customize paths:
```yaml
vault_dir: ~/.repoman/vault
pristines_dir: ~/.repoman/pristines
clones_dir: ~/.repoman/clones
plugins_dir: ~/.repoman/plugins
logs_dir: ~/.repoman/logs
```

## Troubleshooting

### "command not found: repoman"

Check if cargo bin is in PATH:
```bash
echo $PATH | tr ':' '\n' | grep cargo
```

Add if missing:
```bash
echo 'export PATH="$HOME/.cargo/bin:$PATH"' >> ~/.bashrc
source ~/.bashrc
```

### Build fails with OpenSSL/libssh2 errors

Install the development packages for your distro (see Dependencies above).

### Permission errors

`cargo install` doesn't require sudo - it installs to your home directory.
