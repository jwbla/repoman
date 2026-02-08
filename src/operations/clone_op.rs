use git2::Repository;
use log::{debug, error, info};
use rand::Rng;
use std::path::PathBuf;

use crate::config::Config;
use crate::error::{RepomanError, Result};
use crate::metadata::Metadata;
use crate::vault::Vault;

/// Generate a random clone name suffix (6 random alphanumeric chars)
fn generate_clone_suffix() -> String {
    const CHARSET: &[u8] = b"abcdefghijklmnopqrstuvwxyz0123456789";
    let mut rng = rand::thread_rng();

    (0..6)
        .map(|_| {
            let idx = rng.gen_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect()
}

/// Create a clone from a pristine
/// Uses git reference clone for space efficiency
pub fn clone_from_pristine(
    pristine_name: &str,
    clone_name: Option<String>,
    branch: Option<String>,
    config: &Config,
) -> Result<PathBuf> {
    info!("clone_from_pristine: creating clone from '{}'", pristine_name);

    // Check if repo exists in vault
    let vault = Vault::load(config)?;
    if !vault.contains(pristine_name) {
        error!("clone_from_pristine: '{}' not found in vault", pristine_name);
        return Err(RepomanError::RepoNotInVault(pristine_name.to_string()));
    }

    // Check if pristine exists
    let pristine_path = config.pristines_dir.join(pristine_name);
    if !pristine_path.exists() {
        error!("clone_from_pristine: pristine not found at {}", pristine_path.display());
        return Err(RepomanError::PristineNotFound(pristine_name.to_string()));
    }

    // Generate or use provided clone name
    let clone_suffix = clone_name.unwrap_or_else(generate_clone_suffix);
    let full_clone_name = format!("{}-{}", pristine_name, clone_suffix);
    debug!("clone_from_pristine: clone name will be '{}'", full_clone_name);

    // Clone path
    let clone_path = config.clones_dir.join(&full_clone_name);
    if clone_path.exists() {
        error!("clone_from_pristine: clone already exists at {}", clone_path.display());
        return Err(RepomanError::CloneAlreadyExists(full_clone_name));
    }

    // Load metadata
    let mut metadata = Metadata::load(pristine_name, config)?;

    println!("Creating clone {} from pristine...", full_clone_name);

    // Create a reference clone from the pristine
    // This uses git's alternates mechanism for space efficiency
    let pristine_repo = Repository::open_bare(&pristine_path)?;

    // If --branch specified, verify it exists in the pristine
    if let Some(ref b) = branch {
        let ref_name = format!("refs/heads/{}", b);
        if pristine_repo.find_reference(&ref_name).is_err() {
            return Err(RepomanError::BranchNotFound(b.clone(), pristine_name.to_string()));
        }
    }

    // Get the reference to HEAD to check out
    let head_ref = pristine_repo.head()?;
    let head_commit = head_ref.peel_to_commit()?;

    // Create the clone directory
    std::fs::create_dir_all(&clone_path)?;

    // Initialize a new repository
    let clone_repo = Repository::init(&clone_path)?;

    // Set up alternates to reference the pristine's objects
    let alternates_path = clone_path.join(".git").join("objects").join("info");
    std::fs::create_dir_all(&alternates_path)?;

    let pristine_objects = pristine_path.join("objects");
    let alternates_file = alternates_path.join("alternates");
    std::fs::write(
        &alternates_file,
        pristine_objects.to_string_lossy().as_bytes(),
    )?;

    // Add the pristine as a remote named "origin"
    let pristine_url = pristine_path.to_string_lossy();
    clone_repo.remote("origin", &pristine_url)?;

    // Fetch from the pristine
    let mut remote = clone_repo.find_remote("origin")?;
    remote.fetch(&["refs/heads/*:refs/remotes/origin/*"], None, None)?;

    // Determine which branch to check out
    let branch_name = if let Some(ref b) = branch {
        b.as_str()
    } else {
        let short = head_ref
            .shorthand()
            .unwrap_or("main");
        short
            .strip_prefix("refs/heads/")
            .unwrap_or(short)
    };
    // Owned copy for later use
    let branch_name = branch_name.to_string();

    // Create local branch tracking the remote
    let remote_ref_name = format!("refs/remotes/origin/{}", branch_name);
    if let Ok(remote_ref) = clone_repo.find_reference(&remote_ref_name) {
        let commit = remote_ref.peel_to_commit()?;
        clone_repo.branch(&branch_name, &commit, false)?;

        // Set HEAD to the branch
        clone_repo.set_head(&format!("refs/heads/{}", branch_name))?;

        // Check out the working tree
        clone_repo.checkout_head(Some(git2::build::CheckoutBuilder::default().force()))?;
    } else {
        // Fallback: try to checkout HEAD directly
        clone_repo.set_head_detached(head_commit.id())?;
        clone_repo.checkout_head(Some(git2::build::CheckoutBuilder::default().force()))?;
    }

    // Update metadata with clone entry
    metadata.add_clone(clone_suffix.clone(), clone_path.clone());
    metadata.save(pristine_name, config)?;

    info!("clone_from_pristine: clone created at {}", clone_path.display());
    println!("Clone created: {}", clone_path.display());

    Ok(clone_path)
}

/// List all clones for a given pristine
#[allow(dead_code)]
pub fn list_clones(pristine_name: &str, config: &Config) -> Result<Vec<String>> {
    let metadata = Metadata::load(pristine_name, config)?;
    Ok(metadata.clones.iter().map(|c| c.name.clone()).collect())
}
