//! Shared integration-test environment helpers.

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// Directory containing the `repo-guard` binary built for integration tests.
pub fn repo_guard_bin_dir() -> PathBuf {
    Path::new(env!("CARGO_BIN_EXE_repo-guard"))
        .parent()
        .expect("repo-guard binary parent directory")
        .to_path_buf()
}

/// Prepend the test-built `repo-guard` binary directory to `PATH`.
pub fn path_with_repo_guard() -> OsString {
    let mut paths = vec![repo_guard_bin_dir()];
    if let Some(existing) = std::env::var_os("PATH") {
        paths.extend(std::env::split_paths(&existing));
    }
    std::env::join_paths(paths).expect("valid PATH")
}

/// Git command with `PATH` configured so Repo Guard hooks can find the test binary.
pub fn git_command(cwd: &Path) -> Command {
    let mut command = Command::new("git");
    command.current_dir(cwd).env("PATH", path_with_repo_guard());
    command
}

pub fn git_output(cwd: &Path, args: &[&str]) -> Output {
    git_command(cwd)
        .args(args)
        .output()
        .unwrap_or_else(|e| panic!("failed to run git {:?} in {}: {e}", args, cwd.display()))
}

pub fn git_run(cwd: &Path, args: &[&str]) -> String {
    let output = git_output(cwd, args);
    if !output.status.success() {
        panic!(
            "git {:?} failed in {}: {}",
            args,
            cwd.display(),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    String::from_utf8_lossy(&output.stdout).into_owned()
}

/// Minimal `PATH` that excludes the Cargo-built test binary directory.
#[allow(dead_code)]
pub fn path_without_repo_guard() -> OsString {
    let bin_dir = repo_guard_bin_dir();
    let filtered: Vec<PathBuf> = std::env::var_os("PATH")
        .map(|path| std::env::split_paths(&path).collect::<Vec<_>>())
        .unwrap_or_default()
        .into_iter()
        .filter(|entry| entry != &bin_dir)
        .collect();
    if filtered.is_empty() {
        OsString::from("")
    } else {
        std::env::join_paths(filtered).expect("valid PATH")
    }
}
