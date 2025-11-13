# Repoman
## Overview
Repoman is a git repository manager with focus on disposable workspaces, automated synchronization, and extensibility through plugins.

## Agent and Client
### Client
The repoman client is the FE cli tool that submits jobs to the BE and can provide visibility into the state of the app.
### Agent
The repoman agent is the BE that performs all of the actual work.  It performs operations in parallel per repo.

## Concepts
**Vault** A centralized list of repository URLs with accompanying metadata, stored in `~/.repoman/vault/vault.json`.
**Pristine** A clean, local, read-only copy of a repo from the vault, stored in `~/.repoman/pristines/<repo-name>/` as git reference clones for space efficiency. These are the "source of truth" and should never be modified directly.
**Clone** A working copy created from a pristine for feature work, stored in `~/.repoman/clones/<repo-name>-<clone-name>/`. These are disposable workspaces.

## Operations
### Add
#### Description
Add repository to vault. If no argument provided, checks if current directory is in a git repo and extracts the origin remote URL.
#### Examples
```sh
repoman add <git url>
```
```sh
repoman add
```

### Clone
#### Description
Create clone from a pristine. Clone name is auto-generated if not provided.
#### Examples
```sh
repoman clone <pristine> [<clone-name>]
```

### Destroy
#### Description
Destroy target clone or pristine. Destroying a pristine removes it from disk but keeps it in the vault.
#### Examples
```sh
repoman destroy <clone>
```
```sh
repoman destroy <pristine>
```

### Init
#### Description
Create pristine(s) of a vaulted repository using vault name.
#### Examples
```sh
repoman init <vault-name>
```
```sh
repoman init 
```

### Sync
#### Description
Update pristine(s) from origin
#### Examples
```sh
repoman sync 
```
```sh
repoman sync <pristine>
```

### Agent
#### Description
Start/stop background agent
#### Examples
```sh
repoman agent start|stop|status
```

## Plugins
#### Description
Plugin system using Lua, modeled after Neovim's plugin system. Plugins code stored in `~/.repoman/plugins/<plugin-name>/` and executed as:
#### Examples
```sh
repoman <plugin-name> <plugin-command>
```

## Hooks
### Async
Hooks can specify async operations that don't block subsequent commands. Example: Salesforce scratch org provisioning can run async while build proceeds.

### Pre Clone
Executed before cloning

### Post Clone
Executed after cloning

### Pre Build
Executed before build operations

### Post Build
Executed after build operations

### Pre Destroy
Executed before destroying clone (not pristine)

### Post Destroy
Executed after destroying clone (not pristine)

## Metadata for Repos
Each repository has metadata stored in `~/.repoman/vault/<repo-name>/metadata.json`:
- .git URL
- Created on
- Last Updated
- Default branch name (probably master/main)
- Tracked branches
- Clones[] (array of clone objects with name, path, created timestamp)
- Readme
- Sync interval
- Last sync auto/manual
- Build configuration
- Hook configurations

## Config
Configuration stored in `~/.config/repoman/config.yaml` with paths for:
- `~/.repoman/vault/` - Repository metadata
- `~/.repoman/pristines/` - Git reference clones
- `~/.repoman/clones/` - Working copies
- `~/.repoman/plugins/` - Lua plugins
- `~/.repoman/logs/` - Log files

## Use Cases
- **Build from source**: Vault code after building and installing, save disk space
- **Disposable workspaces**: Create experimental workspaces for LLM testing or feature development
- **Master repository list**: Maintain list of software/repos without keeping all on disk
- **QA someone's branch**: Quick clone and test workflow
- **Fresh copy for LLM**: Clean environment for AI-assisted development
- Background garbage collection for git repos
- Mirror to external systems (Gitea, etc.)
- Auto-deploy changes
- Auto-merge capability for clones
