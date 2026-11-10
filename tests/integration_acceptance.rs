//! Release acceptance regression tests.

mod common;

use std::fs;
use std::path::Path;
use std::process::Command;

use common::git_fixture::TestGitRepo;
use common::test_env::{git_command, git_output};

fn assert_hooks_path_is_githooks(root: &Path) {
    let output = git_output(root, &["config", "--local", "--get", "core.hooksPath"]);
    assert!(
        output.status.success(),
        "core.hooksPath must be configured after init --apply"
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), ".githooks");
}

#[cfg(unix)]
fn assert_repo_guard_hook_is_executable(root: &Path, hook_name: &str) {
    use std::os::unix::fs::PermissionsExt;

    let path = root.join(".githooks").join(hook_name);
    assert!(path.is_file(), "expected hook file at {}", path.display());
    let mode = fs::metadata(&path).unwrap().permissions().mode();
    assert_ne!(
        mode & 0o111,
        0,
        "{hook_name} must be executable on Unix (mode={mode:o})"
    );
}

fn run_repo_guard(args: &[&str], root: &Path) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_repo-guard"))
        .args(args)
        .current_dir(root)
        .output()
        .expect("run repo-guard")
}

#[test]
fn clone_install_applies_hooks_path_after_init_commit() {
    let origin = TestGitRepo::new();
    let origin_root = origin.root().to_path_buf();

    run_repo_guard(&["init", "--apply"], &origin_root);

    origin.write_file("README.md", b"hello");
    origin.run_git(&["add", "-A"]);
    origin.git_commit("initial");

    let clone_dir = tempfile::tempdir().expect("tempdir");
    git_command(clone_dir.path())
        .args([
            "clone",
            origin_root.to_str().unwrap(),
            clone_dir.path().to_str().unwrap(),
        ])
        .output()
        .expect("clone");

    let hooks_path_before = git_output(
        clone_dir.path(),
        &["config", "--local", "--get", "core.hooksPath"],
    );
    assert!(!hooks_path_before.status.success());

    let install = run_repo_guard(&["install", "--apply"], clone_dir.path());
    assert!(
        install.status.success(),
        "{}",
        String::from_utf8_lossy(&install.stdout)
    );

    let hooks_path_after = git_output(
        clone_dir.path(),
        &["config", "--local", "--get", "core.hooksPath"],
    );
    assert!(hooks_path_after.status.success());
    assert!(String::from_utf8_lossy(&hooks_path_after.stdout).contains(".githooks"));
}

#[test]
fn ignore_check_warns_on_missing_configured_path() {
    let repo = TestGitRepo::new();
    let root = repo.root();
    let rg = env!("CARGO_BIN_EXE_repo-guard");

    Command::new(rg)
        .args(["init", "--apply"])
        .current_dir(root)
        .output()
        .expect("init");

    let config = r#"
schema_version = 1
lfs_threshold_mib = 50
github_safety_ceiling_mib = 95
block_small_lfs_files = false
output_format = "text"
ignored_paths = ["missing/build/"]
"#;
    std::fs::write(root.join(".repo-guard.toml"), config).unwrap();

    let output = Command::new(rg)
        .args(["ignore", "check"])
        .current_dir(root)
        .output()
        .expect("ignore check");

    assert!(output.status.success());
    let text = String::from_utf8_lossy(&output.stdout);
    assert!(text.contains("ignore.missing_path"));
}

#[test]
fn pre_commit_hook_blocks_large_staged_blob() {
    let repo = TestGitRepo::new();
    let root = repo.root();

    let init = run_repo_guard(&["init", "--apply"], root);
    assert!(
        init.status.success(),
        "init --apply failed: {}",
        String::from_utf8_lossy(&init.stderr)
    );

    assert_hooks_path_is_githooks(root);
    #[cfg(unix)]
    {
        assert_repo_guard_hook_is_executable(root, "pre-commit");
        assert_repo_guard_hook_is_executable(root, "pre-push");
    }

    let bin_path = root.join("big.bin");
    fs::write(&bin_path, vec![0u8; 52 * 1024 * 1024]).unwrap();
    repo.git_add("big.bin");

    let commit = repo.git_output(&["commit", "-m", "blocked"]);

    assert!(
        !commit.status.success(),
        "commit must be blocked by the pre-commit hook; stdout={} stderr={}",
        String::from_utf8_lossy(&commit.stdout),
        String::from_utf8_lossy(&commit.stderr)
    );
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&commit.stdout),
        String::from_utf8_lossy(&commit.stderr)
    );
    assert!(
        combined.contains("blob.lfs_required") || combined.contains("big.bin"),
        "blocked commit must report a Repo Guard policy violation, not silently skip the hook"
    );
}

#[test]
fn standalone_pre_commit_hook_finds_repo_guard_on_test_path() {
    use common::test_env::path_with_repo_guard;

    let repo = TestGitRepo::new();
    run_repo_guard(&["init", "--apply"], repo.root());
    assert_hooks_path_is_githooks(repo.root());

    let hook = repo.root().join(".githooks/pre-commit");
    let output = Command::new("sh")
        .arg(&hook)
        .current_dir(repo.root())
        .env("PATH", path_with_repo_guard())
        .output()
        .expect("run pre-commit hook");

    assert!(
        output.status.success(),
        "hook must run repo-guard when test PATH is configured; stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("not found on PATH"),
        "hook must not report a missing repo-guard binary: {stderr}"
    );
}

#[cfg(unix)]
#[test]
fn standalone_pre_commit_hook_reports_missing_repo_guard_on_path() {
    let repo = TestGitRepo::new();
    let root = repo.root();

    run_repo_guard(&["init", "--apply"], root);
    assert_hooks_path_is_githooks(root);

    let hook = root.join(".githooks/pre-commit");
    let output = Command::new("sh")
        .arg(&hook)
        .current_dir(root)
        .env("PATH", "/usr/bin:/bin")
        .output()
        .expect("run pre-commit hook");

    assert_eq!(
        output.status.code(),
        Some(3),
        "missing repo-guard must use exit code 3; stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("repo-guard") && stderr.contains("PATH"),
        "expected actionable missing-binary message, got: {stderr}"
    );
}

#[test]
fn pre_push_hook_blocks_large_blob_via_git_push() {
    let repo = TestGitRepo::new();
    let root = repo.root();

    let init = run_repo_guard(&["init", "--apply"], root);
    assert!(init.status.success());

    assert_hooks_path_is_githooks(root);
    #[cfg(unix)]
    assert_repo_guard_hook_is_executable(root, "pre-push");

    let bare_dir = tempfile::tempdir().expect("bare dir");
    let bare_path = bare_dir.path().to_str().expect("bare path");
    repo.run_git(&["init", "--bare", bare_path]);
    repo.run_git(&["remote", "add", "origin", bare_path]);

    fs::write(root.join("push-big.bin"), vec![0u8; 52 * 1024 * 1024]).unwrap();
    repo.git_add("push-big.bin");
    repo.git_commit_no_verify("large outgoing");

    let push = repo.git_output(&["push", "-u", "origin", "HEAD"]);

    assert!(
        !push.status.success(),
        "push must be blocked by the pre-push hook; stderr={}",
        String::from_utf8_lossy(&push.stderr)
    );
}

#[test]
fn repair_workflow_allows_check_to_pass() {
    let repo = TestGitRepo::new();
    let root = repo.root();
    let rg = env!("CARGO_BIN_EXE_repo-guard");

    Command::new(rg)
        .args(["init", "--apply"])
        .current_dir(root)
        .output()
        .expect("init apply");

    std::fs::write(root.join("big.bin"), vec![0u8; 52 * 1024 * 1024]).unwrap();
    repo.git_add("big.bin");

    repo.run_git(&["lfs", "track", "--filename", "big.bin"]);
    repo.git_add(".gitattributes");
    repo.run_git(&["add", "--renormalize", "--", "big.bin"]);

    let output = Command::new(rg)
        .args(["check"])
        .current_dir(root)
        .output()
        .expect("check");

    assert!(output.status.success());
}

#[test]
fn json_output_is_clean_stdout() {
    let repo = TestGitRepo::new();
    let rg = env!("CARGO_BIN_EXE_repo-guard");

    Command::new(rg)
        .args(["init", "--apply"])
        .current_dir(repo.root())
        .output()
        .expect("init");

    let output = Command::new(rg)
        .args(["doctor", "--format", "json"])
        .current_dir(repo.root())
        .output()
        .expect("doctor json");

    assert!(output.stderr.is_empty() || output.status.success());
    let value: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("stdout must be valid JSON");
    assert_eq!(value["schema_version"], 1);
}
