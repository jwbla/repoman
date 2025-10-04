# Repoman

A git repository manager focused on disposable workspaces, automated synchronization, and extensibility through plugins.

## What is Repoman?

Repoman solves the problem of managing multiple git repositories efficiently. Instead of keeping all your repositories on disk at once, repoman maintains a "vault" of repository URLs and creates clean, disposable workspaces on demand.

## Why Repoman?

### 🏗️ Build from Source, Save Space
After building and installing software from source, vault the code to save disk space while keeping it easily accessible for future builds.

### 🧪 Disposable Workspaces
Create experimental workspaces for testing, LLM-assisted development, or feature work. Clean up is instant - just destroy the workspace.

### 📋 Master Repository List
Maintain a centralized list of all repositories you use without keeping them all on disk. Initialize pristines only when needed.

### 🔄 Automated Synchronization
Keep your pristine repositories automatically synchronized with upstream changes using the background agent.

## Core Concepts

### Vault
A centralized list of repository URLs with metadata. Think of it as your "master list" of repositories.

### Pristine
A clean, read-only copy of a repository from the vault. These are your "source of truth" copies that stay synchronized with upstream.

### Clone
A working copy created from a pristine for actual development work. These are disposable workspaces you can experiment with.

## Quick Start

### Add a Repository
```bash
# Add by URL
repoman add https://github.com/neovim/neovim.git

# Add current directory's git repo
repoman add

# Add local repository
repoman add /path/to/local/repo
```

### Create a Pristine
```bash
# Initialize specific repository
repoman init neovim

# Initialize all vaulted repositories
repoman init --all
```

### Create a Working Copy
```bash
# Create clone with auto-generated name
repoman clone neovim

# Create clone with specific name
repoman clone neovim my-feature
```

### Synchronize
```bash
# Sync specific pristine
repoman sync neovim

# Sync all pristines
repoman sync --all
```

### Clean Up
```bash
# Destroy a clone
repoman destroy my-feature

# Destroy a pristine (keeps in vault)
repoman destroy neovim
```

## Background Agent

Start the background agent for automated synchronization:

```bash
# Start agent
repoman agent start

# Check status
repoman agent status

# Stop agent
repoman agent stop
```

## Configuration

Repoman stores its configuration in `~/.config/repoman/config.yaml`. The default configuration creates directories under `~/.repoman/`:

- `vault/` - Repository metadata
- `pristines/` - Clean repository copies
- `clones/` - Working copies
- `plugins/` - Lua plugins
- `logs/` - Log files

## Use Cases

### Build from Source Workflow
1. `repoman add <repo-url>` - Add repository to vault
2. `repoman init <repo>` - Create pristine copy
3. Build and install the software
4. `repoman destroy <pristine>` - Remove pristine to save space
5. Repository remains vaulted for future use

### Experimental Development
1. `repoman clone <pristine> experiment` - Create disposable workspace
2. Make experimental changes
3. Test and iterate
4. `repoman destroy experiment` - Clean up instantly

### LLM-Assisted Development
1. `repoman clone <pristine> llm-session` - Create clean workspace
2. Let LLM make changes
3. Review and test
4. `repoman destroy llm-session` - Clean slate for next session

### QA Testing
1. `repoman clone <pristine> qa-test` - Create test environment
2. Test specific features or branches
3. `repoman destroy qa-test` - Clean up after testing

## Installation

### Prerequisites

Before installing repoman, ensure you have the following dependencies:

- **Go 1.19 or later** - Required for building from source
- **Git** - Required for repository operations
- **Linux or macOS** - Windows support planned for future releases

### Building from Source

1. **Install Go** (if not already installed):
   ```bash
   # On Ubuntu/Debian
   sudo apt update
   sudo apt install golang-go
   
   # On macOS with Homebrew
   brew install go
   
   # Or download from https://golang.org/dl/
   ```

2. **Clone and build repoman**:
   ```bash
   git clone <repoman-repo>
   cd repoman
   make build
   ```

3. **Install repoman**:
   ```bash
   # Option 1: System-wide installation
   make install
   
   # Option 2: User installation
   make install-user
   
   # Option 3: Manual installation
   sudo cp build/repoman /usr/local/bin/
   ```

4. **Verify installation**:
   ```bash
   repoman --version
   ```

### Pre-built Binaries

Download pre-built binaries from the [GitHub releases](https://github.com/USERNAME/repoman/releases) page:

**Available Platforms:**
- Linux (amd64, arm64)
- macOS (amd64, arm64)

**Installation:**
```bash
# Download latest release
wget https://github.com/USERNAME/repoman/releases/latest/download/repoman-linux-amd64

# Make executable and install
chmod +x repoman-linux-amd64
sudo mv repoman-linux-amd64 /usr/local/bin/repoman

# Verify installation
repoman --version
```

**Package Managers (Coming Soon):**
- Homebrew (macOS)
- AUR (Arch Linux)
- Snap package (Ubuntu)

## Requirements

- **Go 1.19+** - For building from source
- **Git** - For repository operations
- **Linux or macOS** - Primary supported platforms
- **~/.repoman/** - Directory for repoman data (created automatically)
- **~/.config/repoman/** - Directory for configuration (created automatically)

## License

[License information to be added]

## Status & Reliability

[![CI/CD](https://github.com/USERNAME/repoman/workflows/CI%2FCD%20Pipeline/badge.svg)](https://github.com/USERNAME/repoman/actions)
[![Security Scans](https://github.com/USERNAME/repoman/workflows/Security%20Scans/badge.svg)](https://github.com/USERNAME/repoman/actions)
[![Go Report Card](https://goreportcard.com/badge/github.com/USERNAME/repoman)](https://goreportcard.com/report/github.com/USERNAME/repoman)

### CI/CD Pipeline
- ✅ **Multi-platform builds** - Linux/macOS (amd64/arm64)
- ✅ **Continuous testing** - Go 1.19, 1.20, 1.21
- ✅ **Code quality** - Linting, formatting, coverage
- ✅ **Security scanning** - Vulnerability detection, dependency checking
- ✅ **Automatic releases** - Tagged releases with checksums

### Security Features
- 🔒 **Dependency vulnerability scanning**
- 🔒 **Secret detection** 
- 🔒 **License compliance**
- 🔒 **CodeQL analysis**
- 🔒 **Responsible disclosure process**

## Contributing

We welcome contributions! Please see our [Contributing Guide](CONTRIBUTING.md) for details.

### Quick Contribution Setup
```bash
git clone https://github.com/USERNAME/repoman.git
cd repoman
make build
make test
```

### Development
```bash
# Run tests with coverage
make test-coverage

# Format code
make fmt

# Run linting
make lint
```

### Reporting Issues
- 🐛 [Bug Reports](https://github.com/USERNAME/repoman/issues/new?template=bug_report.md)
- 💡 [Feature Requests](https://github.com/USERNAME/repoman/issues/new?template=feature_request.md)
- 🔒 [Security Issues](SECURITY.md) - Please follow responsible disclosure
