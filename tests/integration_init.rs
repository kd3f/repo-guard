//! Integration tests for init and configuration.

mod common;

use std::process::Command;

use common::git_fixture::TestGitRepo;

#[test]
fn init_dry_run_does_not_write_files() {
    let repo = TestGitRepo::new();
    let root = repo.root();

    let output = Command::new(env!("CARGO_BIN_EXE_repo-guard"))
        .args(["init"])
        .current_dir(root)
        .output()
        .expect("run init");

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(!root.join(".repo-guard.toml").exists());
    assert!(!root.join(".githooks").exists());
}

#[test]
fn init_apply_creates_config_and_hooks() {
    let repo = TestGitRepo::new();
    let root = repo.root();

    let output = Command::new(env!("CARGO_BIN_EXE_repo-guard"))
        .args(["init", "--apply"])
        .current_dir(root)
        .output()
        .expect("run init apply");

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(root.join(".repo-guard.toml").exists());
    assert!(root.join(".githooks/pre-commit").exists());
    assert!(root.join(".githooks/pre-push").exists());

    let hooks_path = repo.run_git(&["config", "--local", "--get", "core.hooksPath"]);
    assert!(hooks_path.contains(".githooks"));

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        for hook in ["pre-commit", "pre-push"] {
            let path = root.join(".githooks").join(hook);
            let mode = std::fs::metadata(path)
                .expect("hook file")
                .permissions()
                .mode();
            assert_ne!(
                mode & 0o111,
                0,
                "{hook} must be executable after init --apply"
            );
        }
    }
}

#[test]
fn doctor_reports_valid_config_after_init() {
    let repo = TestGitRepo::new();
    let root = repo.root();

    Command::new(env!("CARGO_BIN_EXE_repo-guard"))
        .args(["init", "--apply"])
        .current_dir(root)
        .output()
        .expect("init apply");

    let output = Command::new(env!("CARGO_BIN_EXE_repo-guard"))
        .args(["doctor", "--format", "json"])
        .current_dir(root)
        .output()
        .expect("doctor");

    assert!(output.status.success());
    let text = String::from_utf8_lossy(&output.stdout);
    assert!(text.contains("config.valid"));
}

#[test]
fn init_apply_refuses_hooks_path_conflict() {
    let repo = TestGitRepo::new();
    let root = repo.root();
    repo.run_git(&["config", "--local", "core.hooksPath", ".husky"]);

    let output = Command::new(env!("CARGO_BIN_EXE_repo-guard"))
        .args(["init", "--apply"])
        .current_dir(root)
        .output()
        .expect("run init apply");

    assert_eq!(output.status.code(), Some(4));
    let text = String::from_utf8_lossy(&output.stdout);
    assert!(text.contains("hooks.path_conflict"));
}
