use git2::RemoteCallbacks;
use log::debug;
use std::cell::Cell;
use std::path::PathBuf;

use crate::metadata::AuthConfig;

/// Set up credential callbacks on the given `RemoteCallbacks`.
///
/// The caller **must** declare `cred_attempts: Cell<u32>` before `RemoteCallbacks`
/// so the borrow outlives the callbacks (Rust drops in reverse declaration order).
///
/// `auth_config` is optional per-repo auth configuration from metadata.
/// `label` is a short string used in debug log messages (e.g. "init", "sync").
pub fn setup_credentials<'a>(
    callbacks: &mut RemoteCallbacks<'a>,
    cred_attempts: &'a Cell<u32>,
    auth_config: Option<&AuthConfig>,
    label: &str,
) {
    // Clone owned values for the closure
    let ssh_key_path: Option<PathBuf> = auth_config.and_then(|a| a.ssh_key_path.clone());
    let token_env_var: Option<String> = auth_config.and_then(|a| a.token_env_var.clone());
    let label = label.to_string();

    callbacks.credentials(move |url, username_from_url, allowed_types| {
        let attempt = cred_attempts.get();
        debug!(
            "{} credentials: attempt={}, url={}, username={:?}, allowed={:?}",
            label, attempt, url, username_from_url, allowed_types
        );

        if attempt > 0 {
            debug!("{} credentials: rejecting retry to prevent infinite loop", label);
            return Err(git2::Error::from_str("authentication failed"));
        }
        cred_attempts.set(attempt + 1);

        // (1) SSH key: prefer explicit path from auth_config, else ssh-agent
        if allowed_types.contains(git2::CredentialType::SSH_KEY) {
            if let Some(ref key_path) = ssh_key_path {
                if let Some(username) = username_from_url {
                    debug!("{} credentials: trying SSH key from {}", label, key_path.display());
                    return git2::Cred::ssh_key(username, None, key_path, None);
                }
            } else if let Some(username) = username_from_url {
                debug!("{} credentials: trying ssh-agent for '{}'", label, username);
                return git2::Cred::ssh_key_from_agent(username);
            }
        }

        // (2) Default credentials
        if allowed_types.contains(git2::CredentialType::DEFAULT) {
            debug!("{} credentials: trying default credentials", label);
            return git2::Cred::default();
        }

        // (3) Token from env var, else credential helper
        if allowed_types.contains(git2::CredentialType::USER_PASS_PLAINTEXT) {
            if let Some(ref env_var) = token_env_var {
                if let Ok(token) = std::env::var(env_var) {
                    debug!("{} credentials: using token from env var {}", label, env_var);
                    return git2::Cred::userpass_plaintext("git", &token);
                }
            }
            if let Some(username) = username_from_url {
                debug!("{} credentials: trying credential helper for '{}'", label, username);
                return git2::Cred::credential_helper(
                    &git2::Config::open_default()?,
                    url,
                    Some(username),
                );
            }
        }

        debug!("{} credentials: no matching credential type", label);
        Err(git2::Error::from_str("No valid credentials available"))
    });
}
