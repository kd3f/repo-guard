//! pre-commit framework integration acceptance tests.

mod common;

use std::fs;
use std::path::Path;
use std::process::Command;

use common::git_fixture::TestGitRepo;
use common::push_fixture::{init_repo_guard, write_large_file, write_lfs_pointer};
use common::test_env::{git_command, path_with_repo_guard};

fn pre_commit_available() -> bool {
    Command::new("pre-commit")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn consumer_config(include_mutator: bool) -> String {
    let mut yaml = String::from(
        "repos:\n  - repo: local\n    hooks:\n      - id: repo-guard-check\n        name: repo-guard staged index check\n        entry: repo-guard check\n        language: system\n        pass_filenames: false\n        stages: [pre-commit]\n",
    );
    if include_mutator {
        let script = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/stage_large_blob.sh")
            .display()
            .to_string()
            .replace('\\', "/");
        yaml = format!(
            "repos:\n  - repo: local\n    hooks:\n      - id: stage-large\n        name: stage large blob\n        entry: sh \"{script}\"\n        language: system\n        pass_filenames: false\n        always_run: true\n      - id: repo-guard-check\n        name: repo-guard staged index check\n        entry: repo-guard check\n        language: system\n        pass_filenames: false\n        stages: [pre-commit]\n"
        );
    }
    yaml
}

fn init_policy_for_pre_commit(repo: &TestGitRepo) {
    init_repo_guard(repo);
    repo.run_git(&["config", "--unset-all", "core.hooksPath"]);
}

fn manifest_matches_local_hook_entry() {
    let manifest =
        fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join(".pre-commit-hooks.yaml"))
            .expect("root manifest");
    assert!(manifest.contains("entry: repo-guard check"));
    assert!(manifest.contains("pass_filenames: false"));
    assert!(manifest.contains("id: repo-guard-check"));
}

#[test]
fn pre_commit_manifest_validates_at_repo_root() {
    if !pre_commit_available() {
        panic!("pre-commit is required; install with: pip install pre-commit");
    }
    manifest_matches_local_hook_entry();

    let manifest = Path::new(env!("CARGO_MANIFEST_DIR")).join(".pre-commit-hooks.yaml");
    let output = Command::new("pre-commit")
        .args(["validate-manifest", manifest.to_str().expect("utf8")])
        .output()
        .expect("validate-manifest");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn pre_commit_try_repo_uses_root_manifest_when_tracked() {
    if !pre_commit_available() {
        panic!("pre-commit is required; install with: pip install pre-commit");
    }

    let tracked = git_command(Path::new(env!("CARGO_MANIFEST_DIR")))
        .args(["cat-file", "-e", "HEAD:.pre-commit-hooks.yaml"])
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    if !tracked {
        eprintln!(
            "skip: .pre-commit-hooks.yaml is not committed at HEAD; try-repo requires a tracked manifest"
        );
        return;
    }

    let repo = TestGitRepo::new_for_pre_commit_consumer();
    init_policy_for_pre_commit(&repo);
    repo.write_file("ok.txt", b"ok");
    repo.git_add("ok.txt");

    let output = Command::new("pre-commit")
        .args(["try-repo", env!("CARGO_MANIFEST_DIR"), "repo-guard-check"])
        .current_dir(repo.root())
        .env("PATH", path_with_repo_guard())
        .output()
        .expect("try-repo");
    assert!(
        output.status.success(),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn pre_commit_consumer_blocks_large_staged_blob() {
    if !pre_commit_available() {
        panic!("pre-commit is required; install with: pip install pre-commit");
    }

    let repo = TestGitRepo::new();
    init_policy_for_pre_commit(&repo);
    repo.write_file(".pre-commit-config.yaml", consumer_config(false).as_bytes());

    let install = Command::new("pre-commit")
        .args(["install"])
        .current_dir(repo.root())
        .env("PATH", path_with_repo_guard())
        .output()
        .expect("pre-commit install");
    assert!(
        install.status.success(),
        "pre-commit install failed: stdout={} stderr={}",
        String::from_utf8_lossy(&install.stdout),
        String::from_utf8_lossy(&install.stderr)
    );

    write_large_file(&repo, "precommit-block.bin", 55);
    repo.git_add("precommit-block.bin");

    let run = Command::new("pre-commit")
        .args(["run", "repo-guard-check", "--hook-stage", "pre-commit"])
        .current_dir(repo.root())
        .env("PATH", path_with_repo_guard())
        .output()
        .expect("pre-commit run");
    assert!(!run.status.success());
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&run.stdout),
        String::from_utf8_lossy(&run.stderr)
    );
    assert!(text.contains("blob.lfs_required") || text.contains("precommit-block.bin"));
}

#[test]
fn pre_commit_consumer_accepts_lfs_pointer() {
    if !pre_commit_available() {
        panic!("pre-commit is required; install with: pip install pre-commit");
    }

    let repo = TestGitRepo::new();
    init_policy_for_pre_commit(&repo);
    repo.write_file(".pre-commit-config.yaml", consumer_config(false).as_bytes());
    Command::new("pre-commit")
        .args(["install"])
        .current_dir(repo.root())
        .env("PATH", path_with_repo_guard())
        .output()
        .expect("install");

    write_lfs_pointer(&repo, "pointer.bin", 2048);
    repo.git_add("pointer.bin");

    let run = Command::new("pre-commit")
        .args(["run", "repo-guard-check", "--hook-stage", "pre-commit"])
        .current_dir(repo.root())
        .env("PATH", path_with_repo_guard())
        .output()
        .expect("run");
    assert!(
        run.status.success(),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&run.stdout),
        String::from_utf8_lossy(&run.stderr)
    );
}

#[test]
fn pre_commit_runs_after_index_mutating_hook_and_blocks() {
    if !pre_commit_available() {
        panic!("pre-commit is required; install with: pip install pre-commit");
    }

    let repo = TestGitRepo::new();
    init_policy_for_pre_commit(&repo);
    repo.write_file(".pre-commit-config.yaml", consumer_config(true).as_bytes());
    Command::new("pre-commit")
        .args(["install"])
        .current_dir(repo.root())
        .env("PATH", path_with_repo_guard())
        .output()
        .expect("install");

    repo.write_file("seed.txt", b"seed");
    repo.git_add("seed.txt");

    let run = Command::new("pre-commit")
        .args(["run", "--all-files", "--hook-stage", "pre-commit"])
        .current_dir(repo.root())
        .env("PATH", path_with_repo_guard())
        .output()
        .expect("run all hooks");
    assert!(!run.status.success());
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&run.stdout),
        String::from_utf8_lossy(&run.stderr)
    );
    assert!(text.contains("blob.lfs_required") || text.contains("precommit-large.bin"));
}
