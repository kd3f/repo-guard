//! Outgoing push and history validation integration tests.

mod common;

use std::io::Write;
use std::process::{Command, Stdio};

use common::git_fixture::TestGitRepo;

fn run_repo_guard(args: &[&str], cwd: &std::path::Path) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_repo-guard"))
        .args(args)
        .current_dir(cwd)
        .output()
        .expect("run repo-guard")
}

fn init_repo_guard(repo: &TestGitRepo) {
    let output = run_repo_guard(&["init", "--apply"], repo.root());
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn write_large_file(repo: &TestGitRepo, relative: &str, size_mib: u64) {
    let bytes = vec![b'x'; (size_mib * 1024 * 1024) as usize];
    repo.write_file(relative, &bytes);
}

fn run_pre_push_with_stdin(repo: &TestGitRepo, stdin: &str) -> std::process::Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_repo-guard"))
        .args(["hook", "pre-push"])
        .current_dir(repo.root())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn pre-push");
    if let Some(mut input) = child.stdin.take() {
        input.write_all(stdin.as_bytes()).expect("write stdin");
    }
    child.wait_with_output().expect("wait pre-push")
}

#[test]
fn history_check_range_blocks_large_blob_in_commit() {
    let repo = TestGitRepo::new();
    init_repo_guard(&repo);
    repo.write_file("seed.txt", b"seed");
    repo.git_add("seed.txt");
    repo.git_commit("seed");
    write_large_file(&repo, "big.bin", 55);
    repo.git_add("big.bin");
    repo.git_commit_no_verify("add large file");

    let output = run_repo_guard(
        &["history", "check", "--range", "HEAD~1..HEAD"],
        repo.root(),
    );
    assert_eq!(output.status.code(), Some(2));
    let text = String::from_utf8_lossy(&output.stdout);
    assert!(text.contains("blob.lfs_required") || text.contains("big.bin"));
}

#[test]
fn history_check_all_inspects_reachable_history() {
    let repo = TestGitRepo::new();
    init_repo_guard(&repo);
    repo.write_file("small.txt", b"ok");
    repo.git_add("small.txt");
    repo.git_commit("small file");

    let output = run_repo_guard(&["history", "check", "--all"], repo.root());
    assert!(output.status.success());
}

#[test]
fn pre_push_hook_blocks_outgoing_large_blob_with_stdin() {
    let repo = TestGitRepo::new();
    init_repo_guard(&repo);
    write_large_file(&repo, "push-big.bin", 55);
    repo.git_add("push-big.bin");
    repo.git_commit_no_verify("large outgoing");

    let head = repo.run_git(&["rev-parse", "HEAD"]).trim().to_string();
    let zero = "0000000000000000000000000000000000000000";
    let stdin = format!("refs/heads/main {head} refs/heads/main {zero}\n");

    let output = run_pre_push_with_stdin(&repo, &stdin);
    assert_eq!(output.status.code(), Some(2));
    let text = String::from_utf8_lossy(&output.stdout);
    assert!(text.contains("push-big.bin") || text.contains("blob.lfs_required"));
}

#[test]
fn pre_push_without_stdin_ref_updates_reports_no_outgoing_validation() {
    let repo = TestGitRepo::new();
    init_repo_guard(&repo);

    let output = run_pre_push_with_stdin(&repo, "");
    assert!(output.status.success());
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        text.contains("no outgoing-range validation was performed"),
        "expected empty-stdin message, got: {text}"
    );
}

#[test]
fn pre_push_skips_ref_deletion() {
    let repo = TestGitRepo::new();
    init_repo_guard(&repo);

    let zero = "0000000000000000000000000000000000000000";
    let stdin = format!("{zero} {zero} refs/heads/old {zero}\n");

    let output = run_pre_push_with_stdin(&repo, &stdin);
    assert!(output.status.success());
    let text = String::from_utf8_lossy(&output.stdout);
    assert!(text.contains("deletions_skipped") || text.contains("deletion"));
}

#[test]
fn push_to_bare_remote_is_blocked_for_large_blob() {
    let repo = TestGitRepo::new();
    init_repo_guard(&repo);

    let bare_dir = tempfile::tempdir().expect("bare dir");
    let bare_path = bare_dir.path().to_str().expect("bare path");
    repo.run_git(&["init", "--bare", bare_path]);
    repo.run_git(&["remote", "add", "origin", bare_path]);

    write_large_file(&repo, "remote-big.bin", 55);
    repo.git_add("remote-big.bin");
    repo.git_commit_no_verify("large for remote");

    let push = repo.git_output(&["push", "-u", "origin", "HEAD"]);

    assert!(
        !push.status.success(),
        "push should be blocked by pre-push hook: {}",
        String::from_utf8_lossy(&push.stderr)
    );
}

#[test]
fn history_check_json_output_is_valid() {
    let repo = TestGitRepo::new();
    init_repo_guard(&repo);
    repo.write_file("seed.txt", b"seed");
    repo.git_add("seed.txt");
    repo.git_commit("seed");
    repo.write_file("ok.txt", b"fine");
    repo.git_add("ok.txt");
    repo.git_commit("ok");

    let output = run_repo_guard(
        &[
            "history",
            "check",
            "--range",
            "HEAD~1..HEAD",
            "--format",
            "json",
        ],
        repo.root(),
    );
    assert!(output.status.success());
    let value: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("valid history json");
    assert!(value.get("summary").is_some());
}
