//! Helpers for bare-remote push acceptance tests.

#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::process::Command;

use super::git_fixture::TestGitRepo;
use super::test_env::{git_command, git_run};
use tempfile::TempDir;

pub const ZERO_SHA: &str = "0000000000000000000000000000000000000000";
pub const DEFAULT_BRANCH: &str = "main";

pub struct BareRemote {
    _dir: TempDir,
    path: PathBuf,
}

impl BareRemote {
    pub fn new() -> Self {
        let dir = tempfile::tempdir().expect("bare tempdir");
        let path = dir.path().to_path_buf();
        run_git(&path, &["init", "--bare", "-b", "main"]);
        Self { _dir: dir, path }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn path_str(&self) -> &str {
        self.path.to_str().expect("bare path utf8")
    }

    pub fn head(&self) -> String {
        run_git(&self.path, &["rev-parse", "HEAD"])
            .trim()
            .to_string()
    }
}

pub fn write_large_file(repo: &TestGitRepo, relative: &str, size_mib: u64) {
    let bytes = vec![b'x'; (size_mib * 1024 * 1024) as usize];
    repo.write_file(relative, &bytes);
}

pub fn write_lfs_pointer(repo: &TestGitRepo, relative: &str, size: u64) {
    let content = format!(
        "version https://git-lfs.github.com/spec/v1\noid sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef\nsize {size}\n"
    );
    repo.write_file(relative, content.as_bytes());
}

pub fn attach_bare_remote(repo: &TestGitRepo, bare: &BareRemote, name: &str) {
    repo.run_git(&["remote", "add", name, bare.path_str()]);
}

pub fn git_push(repo: &TestGitRepo, args: &[&str]) -> std::process::Output {
    git_command(repo.root())
        .arg("push")
        .args(args)
        .output()
        .expect("git push")
}

pub fn seed_repo(repo: &TestGitRepo) {
    repo.write_file("seed.txt", b"seed");
    repo.git_add("seed.txt");
    repo.git_commit("seed");
}

pub fn seed_and_push(repo: &TestGitRepo, _bare: &BareRemote, branch: &str) {
    seed_repo(repo);
    let push = git_push(repo, &["-u", "origin", branch]);
    assert!(
        push.status.success(),
        "seed push failed: {}",
        String::from_utf8_lossy(&push.stderr)
    );
}

pub fn default_branch(repo: &TestGitRepo) -> String {
    repo.run_git(&["rev-parse", "--abbrev-ref", "HEAD"])
        .trim()
        .to_string()
}

pub fn pre_push_stdin_line(
    local_ref: &str,
    local_sha: &str,
    remote_ref: &str,
    remote_sha: &str,
) -> String {
    format!("{local_ref} {local_sha} {remote_ref} {remote_sha}\n")
}

pub fn run_pre_push_hook(repo: &TestGitRepo, stdin: &str) -> std::process::Output {
    use std::io::Write;
    use std::process::Stdio;

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

pub fn init_repo_guard(repo: &TestGitRepo) {
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_repo-guard"))
        .args(["init", "--apply"])
        .current_dir(repo.root())
        .output()
        .expect("repo-guard init");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

pub fn run_repo_guard(repo: &TestGitRepo, args: &[&str]) -> std::process::Output {
    std::process::Command::new(env!("CARGO_BIN_EXE_repo-guard"))
        .args(args)
        .current_dir(repo.root())
        .output()
        .expect("repo-guard")
}

fn run_git(cwd: &Path, args: &[&str]) -> String {
    git_run(cwd, args)
}
