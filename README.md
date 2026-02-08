# Repoman

A git repository manager built around disposable workspaces. Repoman maintains a vault of repository URLs, creates space-efficient local reference clones (pristines), and lets you spin up and tear down working copies (clones) instantly.

## Why

- Keep a master list of repos without keeping them all on disk
- Create throwaway workspaces for feature branches, QA reviews, or LLM-assisted development
- Space-efficient: pristines are bare repos, clones use git alternates to share objects
- Background agent can auto-sync pristines and detect new tags

## How It Works

```
vault (URLs + metadata)
  -> pristine (bare reference clone)
    -> clone (disposable working copy)
```

1. **Add** a repo URL to the vault (or auto-detect from current directory)
2. **Init** creates a bare "pristine" clone
3. **Clone** creates a lightweight working copy from the pristine
4. **Destroy** the clone when you're done -- the pristine stays
5. **Sync** pulls latest changes into pristines from origin

## Usage

```sh
repoman add <git-url>              # add repo to vault
repoman add                        # auto-detect from current directory
repoman init [<name>]              # create pristine(s)
repoman clone <pristine>           # create working copy
repoman clone <pristine> myfix     # create named working copy
repoman clone <pristine> -b dev    # clone with specific branch
repoman sync [<pristine>]          # fetch latest from origin
repoman status <name>              # detailed repo inspection
repoman open <target>              # print path (for cd $(repoman open foo))
repoman alias <name> <alias>       # create alias for a repo
repoman alias                      # list all aliases
repoman update [<name>]            # sync pristine + fast-forward clones
repoman destroy <target>           # remove a clone or pristine
repoman destroy --all-clones <n>   # destroy all clones for a pristine
repoman destroy --all-pristines    # destroy all pristines (keeps vault)
repoman destroy --stale <days>     # destroy clones older than N days
repoman remove <name>              # fully unregister repo + delete all data
repoman gc --days 30               # garbage-collect stale clones
repoman gc --dry-run               # preview what gc would do
repoman list                       # summary table
repoman list -v                    # detailed view
repoman agent start|stop|status
repoman --version
repoman --debug <command>          # print debug logs to console
```


## Build

Requires Rust 1.85+ and system libraries for git2 (OpenSSL, libssh2).

### System dependencies

Arch Linux:
```sh
sudo pacman -S openssl libssh2
```

Debian/Ubuntu:
```sh
sudo apt install libssl-dev libssh2-1-dev pkg-config cmake
```

Fedora:
```sh
sudo dnf install openssl-devel libssh2-devel
```

macOS:
```sh
brew install openssl libssh2
```

### Compile

```sh
cargo build --release
```

### Run tests

```sh
cargo test
```

## Install

### cargo install (recommended)

```sh
cargo install --path .
```

Installs to `~/.cargo/bin/repoman`. Make sure `~/.cargo/bin` is in your PATH (rustup adds this by default).

Update after pulling changes:
```sh
cargo install --path . --force
```

### Manual

```sh
cargo build --release
cp target/release/repoman ~/.local/bin/   # or /usr/local/bin/
```

### Verify

```sh
repoman --version
```

## Configuration

Optional. Create `~/.config/repoman/config.yaml` to override default paths:

```yaml
vault_dir: ~/.repoman/vault
pristines_dir: ~/.repoman/pristines
clones_dir: ~/.repoman/clones
plugins_dir: ~/.repoman/plugins
logs_dir: ~/.repoman/logs
```

All directories are created automatically on first run.

## Logs

Debug logs are written to `~/.repoman/logs/repoman.log` on every run. Use `--debug` to also print them to the console.

## License

MIT
