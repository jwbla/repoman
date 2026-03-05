# Getting Started

## Installation

### Prerequisites

Repoman requires Rust 1.85+ and system libraries for git2.

**Arch Linux:**

```sh
sudo pacman -S openssl libssh2
```

**Debian / Ubuntu:**

```sh
sudo apt install libssl-dev libssh2-1-dev pkg-config cmake
```

**Fedora:**

```sh
sudo dnf install openssl-devel libssh2-devel
```

**macOS:**

```sh
brew install openssl libssh2
```

### Install with cargo (recommended)

```sh
cargo install --path .
```

This places the `repoman` binary in `~/.cargo/bin/`. Make sure that directory is on your `PATH` (rustup adds it by default).

### Install from release binaries

Download assets from the GitHub Releases page:

- Linux / macOS: `repoman-<OS>-x86_64-<tag>.tar.gz`
- Windows: `repoman-windows-x86_64-<tag>.zip` or `.msi` installer

Extract and place `repoman` on your `PATH`.

### Build from source (manual)

```sh
cargo build --release
cp target/release/repoman ~/.local/bin/
```

### Verify

```sh
repoman --version
```

## First Run

On its first invocation, repoman creates its data directories automatically under `~/.repoman/`:

```
~/.repoman/
  vault/          # vault.json + per-repo metadata
  pristines/      # bare git clones
  clones/         # lightweight working copies
  logs/           # repoman.log, agent.log, agent.pid

~/.config/repoman/
  config.yaml     # configuration file (optional)
  plugins/        # Lua plugin scripts
```

No configuration file is required. If you want to customize paths or add hooks, see [Configuration](configuration.md).

## Quick Example

This walkthrough takes you from zero to a disposable working copy in four commands.

### 1. Add a repository to the vault

```sh
repoman add https://github.com/torvalds/linux.git
```

Or, if you are already inside a cloned repository:

```sh
cd ~/projects/linux
repoman add
```

Repoman detects the remote URL from the current directory.

### 2. Create a pristine (bare reference clone)

```sh
repoman init linux
```

This clones the repository as a bare repo under `~/.repoman/pristines/linux/`. Run `repoman init` with no arguments to initialize all vaulted repos that do not yet have pristines.

### 3. Create a working copy

```sh
repoman clone linux
```

This creates a lightweight clone at `~/.repoman/clones/linux-<random>/` using git alternates so disk usage stays minimal. You can name the clone explicitly:

```sh
repoman clone linux my-feature
```

This creates `~/.repoman/clones/linux-my-feature/`.

To check out a specific branch:

```sh
repoman clone linux bugfix -b stable
```

### 4. Work, then destroy

Do your work in the clone directory. When you are done:

```sh
repoman destroy linux-my-feature
```

The pristine remains, ready for the next clone.

### Keeping things fresh

Sync all pristines from their remotes:

```sh
repoman sync
```

Or sync and fast-forward all clones in one step:

```sh
repoman update
```

### See what you have

```sh
repoman list            # summary table
repoman list -v         # detailed view
repoman status linux    # deep inspection of one repo
repoman dashboard       # interactive TUI
```

### Navigate to a repo

```sh
cd $(repoman open linux)
```

## Next Steps

- Read the [command reference](README.md#commands) for full details on each subcommand.
- Set up [lifecycle hooks](hooks.md) to automate post-clone builds.
- Start the [background agent](commands/agent.md) for automatic syncing.
- Explore [Lua plugins](plugins.md) for custom automation.
