use crate::error::{RepomanError, Result};
use crate::util;
use serde::Deserialize;
use std::env;
use std::fs;
use std::io::Write;
use std::path::Path;

const GITHUB_REPO: &str = "jwbla/repoman";
const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Deserialize)]
struct Release {
    tag_name: String,
    assets: Vec<Asset>,
}

#[derive(Deserialize)]
struct Asset {
    name: String,
    browser_download_url: String,
}

pub async fn handle_upgrade(skip_confirm: bool) -> Result<()> {
    println!("Checking for updates...");

    let release = fetch_latest_release().await?;

    let latest_version = release
        .tag_name
        .strip_prefix('v')
        .unwrap_or(&release.tag_name);

    let current = semver::Version::parse(CURRENT_VERSION)
        .map_err(|e| RepomanError::Other(format!("Failed to parse current version: {e}")))?;
    let latest = semver::Version::parse(latest_version).map_err(|e| {
        RepomanError::Other(format!(
            "Failed to parse latest version '{latest_version}': {e}"
        ))
    })?;

    if current >= latest {
        println!("Already up to date (v{CURRENT_VERSION}).");
        return Ok(());
    }

    println!("Update available: v{CURRENT_VERSION} -> v{latest_version}");

    let asset_name = expected_asset_name(&release.tag_name);
    let asset = release
        .assets
        .iter()
        .find(|a| a.name == asset_name)
        .ok_or_else(|| {
            RepomanError::Other(format!(
                "No release asset found for this platform (expected '{asset_name}')"
            ))
        })?;

    if !skip_confirm && !util::confirm("Download and install?") {
        println!("Cancelled.");
        return Ok(());
    }

    let current_exe = env::current_exe()
        .map_err(|e| RepomanError::Other(format!("Failed to locate current executable: {e}")))?;

    // Resolve symlinks to get the real binary path
    let current_exe = fs::canonicalize(&current_exe)
        .map_err(|e| RepomanError::Other(format!("Failed to resolve executable path: {e}")))?;

    println!("Downloading {asset_name}...");
    let archive_bytes = download_asset(&asset.browser_download_url).await?;

    println!("Installing to {}...", current_exe.display());
    install_binary(&archive_bytes, &asset_name, &current_exe)?;

    println!("Successfully upgraded to v{latest_version}!");
    Ok(())
}

async fn fetch_latest_release() -> Result<Release> {
    let url = format!("https://api.github.com/repos/{GITHUB_REPO}/releases/latest");

    let client = reqwest::Client::new();
    let resp = client
        .get(&url)
        .header("User-Agent", format!("repoman/{CURRENT_VERSION}"))
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
        .map_err(|e| RepomanError::Other(format!("Failed to check for updates: {e}")))?;

    if !resp.status().is_success() {
        return Err(RepomanError::Other(format!(
            "GitHub API returned {} — are you connected to the internet?",
            resp.status()
        )));
    }

    resp.json::<Release>()
        .await
        .map_err(|e| RepomanError::Other(format!("Failed to parse release info: {e}")))
}

async fn download_asset(url: &str) -> Result<Vec<u8>> {
    let client = reqwest::Client::new();
    let resp = client
        .get(url)
        .header("User-Agent", format!("repoman/{CURRENT_VERSION}"))
        .header("Accept", "application/octet-stream")
        .send()
        .await
        .map_err(|e| RepomanError::Other(format!("Download failed: {e}")))?;

    if !resp.status().is_success() {
        return Err(RepomanError::Other(format!(
            "Download failed with status {}",
            resp.status()
        )));
    }

    resp.bytes()
        .await
        .map(|b| b.to_vec())
        .map_err(|e| RepomanError::Other(format!("Failed to read download: {e}")))
}

/// Determine the expected asset filename for the current platform.
fn expected_asset_name(tag: &str) -> String {
    let os = env::consts::OS;
    let arch = env::consts::ARCH;

    let ext = if os == "windows" { "zip" } else { "tar.gz" };

    format!("repoman-{os}-{arch}-{tag}.{ext}")
}

/// Extract the binary from the archive and replace the current executable.
fn install_binary(archive_bytes: &[u8], asset_name: &str, target_path: &Path) -> Result<()> {
    let tmp_dir = tempfile::tempdir()
        .map_err(|e| RepomanError::Other(format!("Failed to create temp directory: {e}")))?;

    let archive_path = tmp_dir.path().join(asset_name);
    let mut f = fs::File::create(&archive_path)
        .map_err(|e| RepomanError::Other(format!("Failed to write archive: {e}")))?;
    f.write_all(archive_bytes)
        .map_err(|e| RepomanError::Other(format!("Failed to write archive: {e}")))?;
    drop(f);

    let binary_name = if cfg!(windows) {
        "repoman.exe"
    } else {
        "repoman"
    };

    // Extract
    if asset_name.ends_with(".tar.gz") {
        extract_tar_gz(&archive_path, tmp_dir.path())?;
    } else if Path::new(asset_name)
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("zip"))
    {
        extract_zip(&archive_path, tmp_dir.path())?;
    } else {
        return Err(RepomanError::Other(format!(
            "Unknown archive format: {asset_name}"
        )));
    }

    let extracted_binary = tmp_dir.path().join(binary_name);
    if !extracted_binary.exists() {
        return Err(RepomanError::Other(format!(
            "Expected binary '{binary_name}' not found in archive"
        )));
    }

    // Set executable permissions on Unix
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&extracted_binary, fs::Permissions::from_mode(0o755))
            .map_err(|e| RepomanError::Other(format!("Failed to set permissions: {e}")))?;
    }

    // Replace the current binary via atomic rename.
    // On Windows, rename the running exe out of the way first.
    #[cfg(windows)]
    {
        let backup = target_path.with_extension("old.exe");
        // Clean up any previous backup
        let _ = fs::remove_file(&backup);
        fs::rename(target_path, &backup).map_err(|e| {
            RepomanError::Other(format!(
                "Failed to move current executable (do you need admin privileges?): {e}"
            ))
        })?;
    }

    // Move new binary into place
    if fs::rename(&extracted_binary, target_path).is_err() {
        // rename fails across filesystems; fall back to copy
        fs::copy(&extracted_binary, target_path).map_err(|e| {
            RepomanError::Other(format!(
                "Failed to install new binary (check write permissions on {}): {e}",
                target_path.display()
            ))
        })?;
    }

    Ok(())
}

fn extract_tar_gz(archive_path: &Path, dest: &Path) -> Result<()> {
    let status = std::process::Command::new("tar")
        .args([
            "xzf",
            &archive_path.to_string_lossy(),
            "-C",
            &dest.to_string_lossy(),
        ])
        .status()
        .map_err(|e| RepomanError::Other(format!("Failed to run tar: {e}")))?;

    if !status.success() {
        return Err(RepomanError::Other("tar extraction failed".to_string()));
    }
    Ok(())
}

fn extract_zip(archive_path: &Path, dest: &Path) -> Result<()> {
    if cfg!(windows) {
        let status = std::process::Command::new("powershell")
            .args([
                "-NoProfile",
                "-Command",
                &format!(
                    "Expand-Archive -Path '{}' -DestinationPath '{}' -Force",
                    archive_path.display(),
                    dest.display()
                ),
            ])
            .status()
            .map_err(|e| RepomanError::Other(format!("Failed to run PowerShell: {e}")))?;

        if !status.success() {
            return Err(RepomanError::Other("zip extraction failed".to_string()));
        }
    } else {
        let status = std::process::Command::new("unzip")
            .args([
                "-o",
                &archive_path.to_string_lossy(),
                "-d",
                &dest.to_string_lossy(),
            ])
            .status()
            .map_err(|e| RepomanError::Other(format!("Failed to run unzip: {e}")))?;

        if !status.success() {
            return Err(RepomanError::Other("zip extraction failed".to_string()));
        }
    }
    Ok(())
}
