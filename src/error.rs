use std::path::PathBuf;
use thiserror::Error;

const AUTH_HELP: &str = r#"

SSH authentication failed. Your SSH key may not be loaded in the agent.

To fix this, try one of:

  1. Load your key for this session:
     $ ssh-add ~/.ssh/id_ed25519

  2. Use 'keychain' for persistent key management (recommended):
     $ sudo pacman -S keychain  # or apt install keychain
     # Add to ~/.bashrc:
     eval $(keychain --eval --quiet id_ed25519)

  3. If using GNOME/KDE, ensure gnome-keyring or kwallet is configured
     to unlock SSH keys on login.

  4. For HTTPS repos, configure git credential helper:
     $ git config --global credential.helper cache
"#;

#[derive(Error, Debug)]
pub enum RepomanError {
    #[error("Repository '{0}' not found in vault")]
    RepoNotInVault(String),

    #[error("Repository '{0}' already exists in vault")]
    RepoAlreadyInVault(String),

    #[error("Pristine '{0}' not found")]
    PristineNotFound(String),

    #[error("Pristine '{0}' already exists")]
    PristineAlreadyExists(String),

    #[error("Clone '{0}' not found")]
    CloneNotFound(String),

    #[error("Clone '{0}' already exists")]
    CloneAlreadyExists(String),

    #[error("Not a git repository: {0}")]
    NotAGitRepo(PathBuf),

    #[error("No remotes found in repository")]
    NoRemotesFound,

    #[error("Could not extract repository name from URL: {0}")]
    InvalidRepoUrl(String),

    #[error("Failed to load vault: {0}")]
    VaultLoadError(String),

    #[error("Failed to save vault: {0}")]
    VaultSaveError(String),

    #[error("Failed to load metadata for '{0}': {1}")]
    MetadataLoadError(String, String),

    #[error("Failed to save metadata for '{0}': {1}")]
    MetadataSaveError(String, String),

    #[error("Authentication failed for '{0}'{AUTH_HELP}")]
    AuthenticationFailed(String),

    #[error("Git operation failed: {0}")]
    GitError(#[from] git2::Error),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("Agent is already running (PID: {0})")]
    AgentAlreadyRunning(u32),

    #[error("Agent is not running")]
    AgentNotRunning,

    #[error("Invalid agent action: {0}. Expected 'start', 'stop', or 'status'")]
    InvalidAgentAction(String),

    #[error("Failed to spawn agent process: {0}")]
    AgentSpawnError(String),

    #[allow(dead_code)]
    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("Alias '{0}' not found")]
    AliasNotFound(String),

    #[error("Fast-forward failed for clone '{0}': {1}")]
    FastForwardFailed(String, String),

    #[error("Branch '{0}' not found in pristine '{1}'")]
    BranchNotFound(String, String),
}

pub type Result<T> = std::result::Result<T, RepomanError>;

/// Check if a git2 error is an authentication failure
pub fn is_auth_error(err: &git2::Error) -> bool {
    err.code() == git2::ErrorCode::Auth
        || err.class() == git2::ErrorClass::Ssh
        || err.message().to_lowercase().contains("auth")
        || err.message().to_lowercase().contains("credential")
        || err.message().to_lowercase().contains("permission denied")
}

/// Convert a git2 error to a RepomanError, with special handling for auth failures
pub fn git_error_with_context(err: git2::Error, repo_name: &str) -> RepomanError {
    if is_auth_error(&err) {
        RepomanError::AuthenticationFailed(repo_name.to_string())
    } else {
        RepomanError::GitError(err)
    }
}
