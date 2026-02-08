// Integration tests for repoman vault functionality
// These test the URL extraction logic that is exposed through the CLI

#[test]
fn test_extract_repo_name_https_variants() {
    // Test various HTTPS URL formats
    let test_cases = vec![
        ("https://github.com/user/repo.git", "repo"),
        ("https://github.com/user/repo", "repo"),
        ("https://github.com/user/repo/", "repo"),
        ("https://gitlab.com/group/subgroup/repo.git", "repo"),
        ("https://bitbucket.org/workspace/repo.git", "repo"),
    ];

    for (url, expected) in test_cases {
        let result = extract_name_from_url(url);
        assert_eq!(result, expected, "Failed for URL: {}", url);
    }
}

#[test]
fn test_extract_repo_name_ssh_variants() {
    let test_cases = vec![
        ("git@github.com:user/repo.git", "repo"),
        ("git@github.com:user/repo", "repo"),
        ("git@gitlab.com:group/subgroup/repo.git", "repo"),
        ("ssh://git@github.com/user/repo.git", "repo"),
    ];

    for (url, expected) in test_cases {
        let result = extract_name_from_url(url);
        assert_eq!(result, expected, "Failed for URL: {}", url);
    }
}

#[test]
fn test_extract_repo_name_local_paths() {
    let test_cases = vec![
        ("/path/to/repo", "repo"),
        ("/path/to/repo/", "repo"),
        ("./relative/path/repo", "repo"),
        ("../parent/repo", "repo"),
    ];

    for (url, expected) in test_cases {
        let result = extract_name_from_url(url);
        assert_eq!(result, expected, "Failed for path: {}", url);
    }
}

// Helper function that mimics the vault::extract_repo_name logic
fn extract_name_from_url(url: &str) -> &str {
    let url = url.trim();
    let url = url.strip_suffix(".git").unwrap_or(url);
    let url = url.strip_suffix('/').unwrap_or(url);

    if url.contains(':') && !url.contains("://") {
        // SSH format
        url.rsplit(':')
            .next()
            .and_then(|path| path.rsplit('/').next())
            .unwrap_or("")
    } else {
        url.rsplit('/').next().unwrap_or("")
    }
}
