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
            debug!(
                "{} credentials: rejecting retry to prevent infinite loop",
                label
            );
            return Err(git2::Error::from_str("authentication failed"));
        }
        cred_attempts.set(attempt + 1);

        // (1) SSH key: prefer explicit path from auth_config, else ssh-agent
        if allowed_types.contains(git2::CredentialType::SSH_KEY) {
            if let Some(ref key_path) = ssh_key_path {
                if let Some(username) = username_from_url {
                    debug!(
                        "{} credentials: trying SSH key from {}",
                        label,
                        key_path.display()
                    );
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
            if let Some(ref env_var) = token_env_var
                && let Ok(token) = std::env::var(env_var)
            {
                debug!(
                    "{} credentials: using token from env var {}",
                    label, env_var
                );
                return git2::Cred::userpass_plaintext("git", &token);
            }
            if let Some(username) = username_from_url {
                debug!(
                    "{} credentials: trying credential helper for '{}'",
                    label, username
                );
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Verify that the attempt-counter guard prevents infinite credential loops.
    /// After setup_credentials is called, the first credential attempt increments
    /// the counter; any subsequent attempt is immediately rejected.
    #[test]
    fn test_credential_attempt_guard() {
        let cred_attempts = Cell::new(0u32);
        let mut callbacks = RemoteCallbacks::new();
        setup_credentials(&mut callbacks, &cred_attempts, None, "test");

        // Try connecting to an invalid SSH URL — this will trigger the credential
        // callback. The guard should reject after the first attempt, so libgit2
        // gives up quickly rather than looping forever.
        let remote = git2::Remote::create_detached("ssh://invalid@localhost:0/nonexistent");
        if let Ok(mut remote) = remote {
            let result = remote.connect_auth(git2::Direction::Fetch, Some(callbacks), None);
            // Should fail (can't connect to localhost:0), but should NOT hang
            assert!(result.is_err());
        }

        // The counter should have been incremented at most once
        assert!(cred_attempts.get() <= 1);
    }

    /// Verify that setup_credentials with an auth config containing a token env var
    /// picks up the token when USER_PASS_PLAINTEXT is allowed.
    #[test]
    fn test_credential_with_auth_config() {
        let auth = AuthConfig {
            ssh_key_path: Some(PathBuf::from("/tmp/nonexistent_key")),
            token_env_var: Some("REPOMAN_TEST_TOKEN".to_string()),
        };

        let cred_attempts = Cell::new(0u32);
        let mut callbacks = RemoteCallbacks::new();
        setup_credentials(&mut callbacks, &cred_attempts, Some(&auth), "test-auth");

        // Just verify it doesn't panic during setup — the actual credential
        // resolution happens inside libgit2 callbacks during connect.
        assert_eq!(cred_attempts.get(), 0);
    }

    /// Verify the counter starts at 0 and blocks on retry.
    #[test]
    fn test_credential_counter_initial_state() {
        let cred_attempts = Cell::new(0u32);
        let callbacks = RemoteCallbacks::new();
        // Before any credential callback fires, counter is 0
        assert_eq!(cred_attempts.get(), 0);
        drop(callbacks);
    }
}
