# Repoman Test Drive

A hands-on guide to trying out all repoman features.

## Prerequisites

1. Build repoman:
   ```bash
   cargo build --release
   ```

2. Ensure SSH agent has your key loaded (for remote repos):
   ```bash
   ssh-add ~/.ssh/id_ed25519
   ```

3. Optional: Add to PATH for convenience:
   ```bash
   export PATH="$PWD/target/release:$PATH"
   ```

## 1. Adding Repositories to the Vault

### Add from current directory
```bash
# Navigate to any git repo
cd ~/some-project

# Add it to the vault (auto-detects remotes)
repoman add
```

### Add by URL
```bash
repoman add https://github.com/rust-lang/rust.git
repoman add git@github.com:torvalds/linux.git
```

### Verify
```bash
repoman list
```

## 2. Listing Repositories

### Summary view (default)
```bash
repoman list
```
Output:
```
NAME                 PRISTINE     CLONES   LAST SYNC
----------------------------------------------------------------
repoman              ✗            0        never
rust                 ✗            0        never
```

### Verbose view
```bash
repoman list -v
```
Output:
```
Repository Details:

  repoman
    URL: git@github.com:user/repoman.git
    Added: 2024-01-15 10:30
    Pristine: ✗ not initialized
    Clones: none
```

## 3. Initializing Pristines

A pristine is a bare clone that serves as your local "source of truth".

### Initialize a single repo
```bash
repoman init repoman
```

### Initialize all vaulted repos
```bash
repoman init
```

### Verify
```bash
repoman list
# Should show ✓ under PRISTINE column

ls ~/.repoman/pristines/
# Should show the bare repo directories
```

## 4. Creating Clones (Workspaces)

Clones are lightweight working copies created from pristines.

### Create a clone with auto-generated name
```bash
repoman clone repoman
# Creates: ~/.repoman/clones/repoman-abc123/
```

### Create a clone with custom name
```bash
repoman clone repoman feature-branch
# Creates: ~/.repoman/clones/repoman-feature-branch/
```

### Create a clone on a specific branch
```bash
repoman clone repoman bugfix -b develop
# Creates clone checked out on "develop" branch
```

### Verify
```bash
repoman list -v
# Shows clones under each repo

ls ~/.repoman/clones/
```

### Work in the clone
```bash
cd ~/.repoman/clones/repoman-abc123/
git checkout -b my-feature
# ... make changes ...
git push origin my-feature
```

## 5. Syncing Pristines

Update pristines from their remote origins.

### Sync a single repo
```bash
repoman sync repoman
```

### Sync all repos (parallel)
```bash
repoman sync
```

### Verify
```bash
repoman list
# LAST SYNC column should show recent timestamp
```

## 6. Status

Detailed inspection of a single repository: clone branches, dirty state, ahead/behind counts, alternates health.

```bash
repoman status repoman
```

## 7. Open

Print the filesystem path for a pristine or clone. Designed for shell integration:

```bash
cd $(repoman open repoman)

# Or open a specific clone
cd $(repoman open repoman-feature-branch)
```

## 8. Aliases

Create short names for repositories. Aliases resolve transparently in all commands.

### Create an alias
```bash
repoman alias repoman rm
```

### Use the alias
```bash
repoman status rm       # same as: repoman status repoman
repoman clone rm myfix  # same as: repoman clone repoman myfix
```

### List all aliases
```bash
repoman alias
```

### Remove an alias
```bash
repoman alias repoman rm --remove
```

## 9. Update

Sync pristine from remote, then fast-forward all clones to match.

### Update a single repo
```bash
repoman update repoman
```

### Update all repos (parallel)
```bash
repoman update
```

## 10. Destroying Clones and Pristines

### Destroy a single clone
```bash
# By clone suffix
repoman destroy abc123

# Or by full directory name
repoman destroy repoman-abc123
```

### Destroy a pristine (keeps vault entry)
```bash
repoman destroy repoman
# Removes ~/.repoman/pristines/repoman/
# Vault entry remains, can re-init later
```

### Destroy all clones for a repo
```bash
repoman destroy --all-clones repoman
```

### Destroy all pristines
```bash
repoman destroy --all-pristines
# Removes all pristine directories, vault entries kept
```

### Destroy stale clones
```bash
repoman destroy --stale 14
# Removes clones with HEAD older than 14 days
```

### Verify
```bash
repoman list
# Pristine shows ✗, clones removed
```

## 11. Removing a Repository

Fully unregister a repository: destroys all clones, pristine, metadata, aliases, and vault entry.

```bash
repoman remove repoman
```

This is the nuclear option. Use `destroy` if you only want to remove clones or pristines.

```bash
# Works with aliases too
repoman remove rm   # removes the repo that "rm" points to
```

## 12. Garbage Collection

Clean up stale clones and compact pristines.

### Preview what would be cleaned
```bash
repoman gc --dry-run
```

### Run cleanup (default: clones older than 30 days)
```bash
repoman gc
```

### Custom age threshold
```bash
repoman gc --days 7
```

## 13. Background Agent

The agent periodically checks for new tags and auto-syncs.

### Start the agent
```bash
repoman agent start
```

### Check status
```bash
repoman agent status
# Output: Agent is running (PID: 12345)
#         Log file: ~/.repoman/logs/agent.log
```

### View agent logs
```bash
tail -f ~/.repoman/logs/agent.log
```

### Stop the agent
```bash
repoman agent stop
```

## Complete Workflow Example

```bash
# 1. Add a repo
repoman add https://github.com/neovim/neovim.git

# 2. Create an alias
repoman alias neovim nv

# 3. Initialize the pristine
repoman init nv

# 4. Check status
repoman status nv

# 5. Create a workspace
repoman clone nv bugfix

# 6. Work on it
cd $(repoman open nv-bugfix)
git checkout -b fix-issue-123
# ... make changes ...
git commit -am "Fix issue #123"
git push origin fix-issue-123

# 7. Clean up when done
repoman destroy bugfix

# 8. Later, sync + fast-forward all clones
repoman update nv

# 9. Create fresh workspace for next task
repoman clone nv new-feature

# 10. When done with the repo entirely
repoman remove nv
```

## Directory Structure

After using repoman, your `~/.repoman/` will look like:

```
~/.repoman/
├── vault/
│   ├── vault.json              # List of all vaulted repos
│   ├── repoman/
│   │   └── metadata.json       # Repo metadata, clone list, etc.
│   └── neovim/
│       └── metadata.json
├── pristines/
│   ├── repoman/                # Bare git repo
│   └── neovim/                 # Bare git repo
├── clones/
│   ├── repoman-abc123/         # Working copy
│   └── neovim-bugfix/          # Working copy
├── plugins/                    # (future: Lua plugins)
└── logs/
    ├── repoman.log             # Debug log
    └── agent.log               # Agent output
```

## Configuration (Optional)

Create `~/.config/repoman/config.yaml` to customize paths:

```yaml
vault_dir: ~/my-repoman/vault
pristines_dir: ~/my-repoman/pristines
clones_dir: ~/my-repoman/clones
plugins_dir: ~/my-repoman/plugins
logs_dir: ~/my-repoman/logs
```

## Troubleshooting

### "Authentication failed" error
See [docs/ssh-authentication.md](docs/ssh-authentication.md) for SSH setup options.

Quick fix:
```bash
ssh-add ~/.ssh/id_ed25519
```

### "Pristine not found" error
Initialize it first:
```bash
repoman init <repo-name>
```

### "Repository already exists in vault"
It's already added. Check with:
```bash
repoman list
```

### Agent won't start
Check if already running:
```bash
repoman agent status
```

Check logs:
```bash
cat ~/.repoman/logs/agent.log
```
