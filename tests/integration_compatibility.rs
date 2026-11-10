//! Integration tests for hook-manager detection and non-takeover behavior.

mod common;

use std::process::Command;

use common::git_fixture::TestGitRepo;

fn parse_detect_summary(stdout: &[u8]) -> serde_json::Value {
    let value: serde_json::Value =
        serde_json::from_slice(stdout).expect("detect stdout must be valid JSON");
    value
        .get("summary")
        .cloned()
        .expect("detect report must include summary")
}

fn hooks_path_is_unset(repo: &TestGitRepo) -> bool {
    let output = Command::new("git")
        .args(["config", "--local", "--get", "core.hooksPath"])
        .current_dir(repo.root())
        .output()
        .expect("git config");
    !output.status.success()
}

#[test]
fn detect_recommends_standalone_for_fresh_repo() {
    let repo = TestGitRepo::new();
    let root = repo.root();

    let output = Command::new(env!("CARGO_BIN_EXE_repo-guard"))
        .args(["detect", "--format", "json"])
        .current_dir(root)
        .output()
        .expect("detect");

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let summary = parse_detect_summary(&output.stdout);
    assert_eq!(summary["recommended_mode"], "standalone");
    assert_eq!(summary["allows_standalone_hook_setup"], true);
}

#[test]
fn detect_reports_pre_commit_integration_mode() {
    let repo = TestGitRepo::new();
    let root = repo.root();
    repo.write_file(".pre-commit-config.yaml", b"repos: []\n");

    let output = Command::new(env!("CARGO_BIN_EXE_repo-guard"))
        .args(["detect", "--format", "json"])
        .current_dir(root)
        .output()
        .expect("detect");

    assert!(output.status.success());
    let summary = parse_detect_summary(&output.stdout);
    assert_eq!(summary["recommended_mode"], "integration");
    assert_eq!(summary["primary_manager"], "pre_commit");
    assert_eq!(summary["allows_standalone_hook_setup"], false);
}

#[test]
fn detect_reports_husky_integration_mode() {
    let repo = TestGitRepo::new();
    let root = repo.root();
    std::fs::create_dir_all(root.join(".husky")).expect("husky dir");

    let output = Command::new(env!("CARGO_BIN_EXE_repo-guard"))
        .args(["detect"])
        .current_dir(root)
        .output()
        .expect("detect");

    assert!(output.status.success());
    let text = String::from_utf8_lossy(&output.stdout);
    assert!(text.contains("integration"));
    assert!(text.contains("husky"));
}

#[test]
fn detect_reports_lefthook_integration_mode() {
    let repo = TestGitRepo::new();
    let root = repo.root();
    repo.write_file("lefthook.yml", b"pre-commit:\n  commands: {}\n");

    let output = Command::new(env!("CARGO_BIN_EXE_repo-guard"))
        .args(["detect", "--format", "json"])
        .current_dir(root)
        .output()
        .expect("detect");

    assert!(output.status.success());
    let summary = parse_detect_summary(&output.stdout);
    assert_eq!(summary["primary_manager"], "lefthook");
}

#[test]
fn init_apply_refuses_pre_commit_takeover() {
    let repo = TestGitRepo::new();
    let root = repo.root();
    repo.write_file(".pre-commit-config.yaml", b"repos: []\n");

    let output = Command::new(env!("CARGO_BIN_EXE_repo-guard"))
        .args(["init", "--apply"])
        .current_dir(root)
        .output()
        .expect("init apply");

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let text = String::from_utf8_lossy(&output.stdout);
    assert!(text.contains("hooks.manager_owned"));
    assert!(!root.join(".githooks/pre-commit").exists());
    assert!(hooks_path_is_unset(&repo));
    assert!(root.join(".repo-guard.toml").exists());
}

#[test]
fn init_apply_refuses_husky_directory_takeover() {
    let repo = TestGitRepo::new();
    let root = repo.root();
    std::fs::create_dir_all(root.join(".husky")).expect("husky");

    let output = Command::new(env!("CARGO_BIN_EXE_repo-guard"))
        .args(["init", "--apply"])
        .current_dir(root)
        .output()
        .expect("init apply");

    let text = String::from_utf8_lossy(&output.stdout);
    assert!(text.contains("hooks.manager_owned") || text.contains("hooks.path_conflict"));
    assert!(!root.join(".githooks/pre-commit").exists());
}

#[test]
fn init_apply_still_creates_config_when_manager_detected() {
    let repo = TestGitRepo::new();
    let root = repo.root();
    repo.write_file(".pre-commit-config.yaml", b"repos: []\n");

    let output = Command::new(env!("CARGO_BIN_EXE_repo-guard"))
        .args(["init", "--apply"])
        .current_dir(root)
        .output()
        .expect("init apply");

    assert!(output.status.success());
    assert!(root.join(".repo-guard.toml").exists());
    assert!(!root.join(".githooks").exists());
}

#[test]
fn install_apply_refuses_lefthook_takeover() {
    let repo = TestGitRepo::new();
    let root = repo.root();
    repo.write_file(".repo-guard.toml", b"schema_version = 1\n");
    repo.write_file("lefthook-local.yml", b"pre-commit:\n  commands: {}\n");

    let output = Command::new(env!("CARGO_BIN_EXE_repo-guard"))
        .args(["install", "--apply"])
        .current_dir(root)
        .output()
        .expect("install apply");

    let text = String::from_utf8_lossy(&output.stdout);
    assert!(text.contains("hooks.manager_owned"));
    assert!(!root.join(".githooks/pre-commit").exists());
}

#[test]
fn init_apply_preserves_standalone_when_no_manager() {
    let repo = TestGitRepo::new();
    let root = repo.root();

    let output = Command::new(env!("CARGO_BIN_EXE_repo-guard"))
        .args(["init", "--apply"])
        .current_dir(root)
        .output()
        .expect("init apply");

    assert!(output.status.success());
    assert!(root.join(".githooks/pre-commit").exists());
}

#[test]
fn detect_reports_custom_hooks_path() {
    let repo = TestGitRepo::new();
    let root = repo.root();
    repo.run_git(&["config", "--local", "core.hooksPath", ".custom-hooks"]);

    let output = Command::new(env!("CARGO_BIN_EXE_repo-guard"))
        .args(["detect", "--format", "json"])
        .current_dir(root)
        .output()
        .expect("detect");

    assert!(output.status.success());
    let summary = parse_detect_summary(&output.stdout);
    assert_eq!(summary["primary_manager"], "custom_hooks_path");
}

#[test]
fn integrate_print_pre_commit_snippet() {
    let repo = TestGitRepo::new();
    let output = Command::new(env!("CARGO_BIN_EXE_repo-guard"))
        .args(["integrate", "pre-commit", "--print"])
        .current_dir(repo.root())
        .output()
        .expect("integrate print");

    assert!(output.status.success());
    let text = String::from_utf8_lossy(&output.stdout);
    assert!(text.contains("repo-guard check"));
    assert!(text.contains("pass_filenames: false"));
}

#[test]
fn integrate_detect_matches_detect_command() {
    let repo = TestGitRepo::new();
    repo.write_file(".pre-commit-config.yaml", b"repos: []\n");

    let detect = Command::new(env!("CARGO_BIN_EXE_repo-guard"))
        .args(["detect", "--format", "json"])
        .current_dir(repo.root())
        .output()
        .expect("detect");
    let integrate = Command::new(env!("CARGO_BIN_EXE_repo-guard"))
        .args(["integrate", "detect", "--format", "json"])
        .current_dir(repo.root())
        .output()
        .expect("integrate detect");

    assert!(integrate.status.success());
    let a: serde_json::Value = serde_json::from_slice(&detect.stdout).unwrap();
    let b: serde_json::Value = serde_json::from_slice(&integrate.stdout).unwrap();
    assert_eq!(
        a["summary"]["recommended_mode"],
        b["summary"]["recommended_mode"]
    );
}
