use std::process::Command;

use tempfile::TempDir;

use repoman::config::Config;
use repoman::operations;

#[test]
fn test_full_pipeline_local_repo() {
    // 1. Create a tempdir and set up Config pointing all dirs into it.
    let tmp = TempDir::new().expect("failed to create tempdir");
    let base = tmp.path();

    let config = Config {
        vault_dir: base.join("vault"),
        pristines_dir: base.join("pristines"),
        clones_dir: base.join("clones"),
        plugins_dir: base.join("plugins"),
        logs_dir: base.join("logs"),
        agent_heartbeat_interval: None,
        json_output: None,
        max_parallel: None,
        repos: None,
    };

    // Create the directories that Config points to.
    std::fs::create_dir_all(&config.vault_dir).unwrap();
    std::fs::create_dir_all(&config.pristines_dir).unwrap();
    std::fs::create_dir_all(&config.clones_dir).unwrap();
    std::fs::create_dir_all(&config.plugins_dir).unwrap();
    std::fs::create_dir_all(&config.logs_dir).unwrap();

    // 2. Create a local bare git repo and a work repo, commit a file, push to bare.
    let bare_dir = base.join("origin.git");
    let work_dir = base.join("workrepo");

    Command::new("git")
        .args(["init", "--bare"])
        .arg(&bare_dir)
        .output()
        .expect("git init --bare failed");

    Command::new("git")
        .args(["init"])
        .arg(&work_dir)
        .output()
        .expect("git init failed");

    // Configure user identity for the work repo.
    Command::new("git")
        .args(["config", "user.email", "test@test.com"])
        .current_dir(&work_dir)
        .output()
        .expect("git config email failed");

    Command::new("git")
        .args(["config", "user.name", "Test User"])
        .current_dir(&work_dir)
        .output()
        .expect("git config name failed");

    // Create initial commit.
    std::fs::write(work_dir.join("README.md"), "# Test Repo\n").unwrap();

    Command::new("git")
        .args(["add", "."])
        .current_dir(&work_dir)
        .output()
        .expect("git add failed");

    Command::new("git")
        .args(["commit", "-m", "initial commit"])
        .current_dir(&work_dir)
        .output()
        .expect("git commit failed");

    // Determine the default branch name in the work repo.
    let branch_output = Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .current_dir(&work_dir)
        .output()
        .expect("git rev-parse failed");
    let branch = String::from_utf8(branch_output.stdout)
        .unwrap()
        .trim()
        .to_string();

    // Add bare repo as remote and push.
    let bare_url = format!("file://{}", bare_dir.display());

    Command::new("git")
        .args(["remote", "add", "origin", &bare_url])
        .current_dir(&work_dir)
        .output()
        .expect("git remote add failed");

    let push_output = Command::new("git")
        .args(["push", "-u", "origin", &branch])
        .current_dir(&work_dir)
        .output()
        .expect("git push failed");
    assert!(
        push_output.status.success(),
        "git push failed: {}",
        String::from_utf8_lossy(&push_output.stderr)
    );

    // 3. add_repo → verify vault entry exists.
    let name = operations::add_repo(Some(bare_url.clone()), &config).expect("add_repo failed");

    let vault_path = config.vault_dir.join("vault.json");
    assert!(
        vault_path.exists(),
        "vault.json should exist after add_repo"
    );

    let vault_contents = std::fs::read_to_string(&vault_path).unwrap();
    assert!(
        vault_contents.contains(&name),
        "vault.json should contain repo name '{}'",
        name
    );

    // 4. init_pristine → verify pristine dir exists and is bare.
    let pristine_path =
        operations::init_pristine(&name, None, &config).expect("init_pristine failed");

    assert!(pristine_path.exists(), "pristine dir should exist");
    // A bare repo has a HEAD file directly in its root.
    assert!(
        pristine_path.join("HEAD").exists(),
        "pristine should be a bare repo (HEAD file present)"
    );
    // A bare repo should NOT have a .git subdirectory.
    assert!(
        !pristine_path.join(".git").exists(),
        "pristine should be bare (no .git subdirectory)"
    );

    // 5. clone_from_pristine → verify clone dir exists and has a working tree.
    let clone_path =
        operations::clone_from_pristine(&name, Some("test".to_string()), None, &config)
            .expect("clone_from_pristine failed");

    assert!(clone_path.exists(), "clone dir should exist");
    assert!(
        clone_path.join(".git").exists(),
        "clone should have a .git directory (working tree)"
    );
    assert!(
        clone_path.join("README.md").exists(),
        "clone should contain README.md from the initial commit"
    );

    // 6. Add a new commit to the source work repo and push to bare.
    std::fs::write(work_dir.join("second.txt"), "second file\n").unwrap();

    Command::new("git")
        .args(["add", "."])
        .current_dir(&work_dir)
        .output()
        .expect("git add failed");

    Command::new("git")
        .args(["commit", "-m", "second commit"])
        .current_dir(&work_dir)
        .output()
        .expect("git commit failed");

    let push2 = Command::new("git")
        .args(["push", "origin", &branch])
        .current_dir(&work_dir)
        .output()
        .expect("git push failed");
    assert!(
        push2.status.success(),
        "second git push failed: {}",
        String::from_utf8_lossy(&push2.stderr)
    );

    // Record the new commit hash from the bare repo.
    let rev_output = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(&work_dir)
        .output()
        .expect("git rev-parse HEAD failed");
    let new_commit = String::from_utf8(rev_output.stdout)
        .unwrap()
        .trim()
        .to_string();

    // 7. sync_pristine → verify new commit is in pristine.
    operations::sync_pristine(&name, &config).expect("sync_pristine failed");

    // Check that the pristine contains the new commit by listing refs.
    let log_output = Command::new("git")
        .args(["log", "--oneline", "--all"])
        .current_dir(&pristine_path)
        .output()
        .expect("git log in pristine failed");
    let log_str = String::from_utf8(log_output.stdout).unwrap();
    assert!(
        log_str.contains("second commit"),
        "pristine should contain 'second commit' after sync, got: {}",
        log_str
    );

    // Also verify via rev-parse that the exact commit hash is present.
    let pristine_rev = Command::new("git")
        .args(["rev-parse", &format!("refs/heads/{}", branch)])
        .current_dir(&pristine_path)
        .output()
        .expect("git rev-parse in pristine failed");
    let pristine_head = String::from_utf8(pristine_rev.stdout)
        .unwrap()
        .trim()
        .to_string();
    assert_eq!(
        pristine_head, new_commit,
        "pristine HEAD should match the new commit after sync"
    );

    // 8. destroy_clone → verify clone removed.
    operations::destroy_clone("test", &config).expect("destroy_clone failed");
    assert!(
        !clone_path.exists(),
        "clone dir should be removed after destroy_clone"
    );

    // 9. destroy_pristine → verify pristine removed.
    operations::destroy_pristine(&name, &config).expect("destroy_pristine failed");
    assert!(
        !pristine_path.exists(),
        "pristine dir should be removed after destroy_pristine"
    );
}
