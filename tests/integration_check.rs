//! Integration tests for staged-index validation.

mod common;

use std::process::Command;

use common::git_fixture::TestGitRepo;

#[test]
fn check_blocks_large_staged_blob() {
    let repo = TestGitRepo::new();
    let root = repo.root();

    init_repo(&repo);

    let large = vec![0u8; 60 * 1024 * 1024];
    repo.write_file("big.bin", &large);
    repo.git_add("big.bin");

    let output = Command::new(env!("CARGO_BIN_EXE_repo-guard"))
        .args(["check"])
        .current_dir(root)
        .output()
        .expect("check");

    assert_eq!(output.status.code(), Some(2));
    let text = String::from_utf8_lossy(&output.stdout);
    assert!(text.contains("blob.lfs_required") || text.contains("big.bin"));
    assert!(text.contains("git lfs track"));
}

#[test]
fn check_accepts_small_staged_file() {
    let repo = TestGitRepo::new();
    let root = repo.root();
    init_repo(&repo);

    repo.write_file("small.txt", b"hello");
    repo.git_add("small.txt");

    let output = Command::new(env!("CARGO_BIN_EXE_repo-guard"))
        .args(["check"])
        .current_dir(root)
        .output()
        .expect("check");

    assert!(output.status.success());
}

fn init_repo(repo: &TestGitRepo) {
    Command::new(env!("CARGO_BIN_EXE_repo-guard"))
        .args(["init", "--apply"])
        .current_dir(repo.root())
        .output()
        .expect("init");
}
