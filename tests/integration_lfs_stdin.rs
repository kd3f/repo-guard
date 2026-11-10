//! Git LFS pre-push stdin preservation via the production hook tee pattern.

mod common;

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use common::git_fixture::TestGitRepo;
use common::push_fixture::{init_repo_guard, seed_repo, write_large_file, ZERO_SHA};
use common::test_env::path_with_repo_guard;

fn sh_path() -> Option<PathBuf> {
    for candidate in ["sh", "bash"] {
        if Command::new(candidate)
            .args(["-c", "exit 0"])
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
        {
            return Some(PathBuf::from(candidate));
        }
    }
    None
}

fn fixture_path(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

fn run_test_pre_push_hook(repo: &TestGitRepo, capture: &Path, stdin: &str) -> Output {
    let sh = sh_path().expect("sh required");
    let hook = fixture_path("pre-push-with-capture.sh");
    let capture_sh = fixture_path("lfs_pre_push_capture.sh");

    let mut child = Command::new(&sh)
        .arg(hook)
        .arg("origin")
        .current_dir(repo.root())
        .env("PATH", path_with_repo_guard())
        .env("GIT_LFS_CAPTURE_SH", capture_sh)
        .env("GIT_LFS_STDIN_CAPTURE", capture)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn hook");

    if let Some(mut input) = child.stdin.take() {
        input.write_all(stdin.as_bytes()).expect("stdin");
    }

    child.wait_with_output().expect("wait hook")
}

#[test]
fn githooks_pre_push_preserves_ref_stdin_for_git_lfs_on_success() {
    if sh_path().is_none() {
        eprintln!("skip: sh/bash not available");
        return;
    }

    let repo = TestGitRepo::new();
    init_repo_guard(&repo);
    seed_repo(&repo);

    let head = repo.run_git(&["rev-parse", "HEAD"]).trim().to_string();
    let branch = repo
        .run_git(&["rev-parse", "--abbrev-ref", "HEAD"])
        .trim()
        .to_string();
    let stdin = format!("refs/heads/{branch} {head} refs/heads/{branch} {ZERO_SHA}\n");

    let capture = tempfile::NamedTempFile::new().expect("capture");
    let output = run_test_pre_push_hook(&repo, capture.path(), &stdin);
    assert!(
        output.status.success(),
        "hook should succeed for clean history: stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let captured = fs::read_to_string(capture.path()).expect("capture read");
    assert_eq!(
        captured, stdin,
        "Git LFS must receive the original ref list bytes"
    );
}

#[test]
fn githooks_pre_push_skips_git_lfs_when_repo_guard_blocks() {
    if sh_path().is_none() {
        return;
    }

    let repo = TestGitRepo::new();
    init_repo_guard(&repo);
    write_large_file(&repo, "block-lfs.bin", 55);
    repo.git_add("block-lfs.bin");
    repo.git_commit_no_verify("large");

    let head = repo.run_git(&["rev-parse", "HEAD"]).trim().to_string();
    let branch = repo
        .run_git(&["rev-parse", "--abbrev-ref", "HEAD"])
        .trim()
        .to_string();
    let stdin = format!("refs/heads/{branch} {head} refs/heads/{branch} {ZERO_SHA}\n");

    let capture = tempfile::NamedTempFile::new().expect("capture");
    let output = run_test_pre_push_hook(&repo, capture.path(), &stdin);
    assert_ne!(
        output.status.code(),
        Some(0),
        "repo-guard must block the push"
    );
    let captured = fs::read_to_string(capture.path()).unwrap_or_default();
    assert!(
        captured.is_empty(),
        "Git LFS pre-push must not run when repo-guard fails"
    );
}
