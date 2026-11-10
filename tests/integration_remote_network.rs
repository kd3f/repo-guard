//! Optional network-dependent remote checks (excluded from offline CI by default).

mod common;

use std::process::Command;

use common::git_fixture::TestGitRepo;

/// Requires outbound network access to a public host; not part of the offline suite.
#[test]
#[ignore = "network optional: run with cargo test --ignored -- --nocapture"]
fn remote_check_public_host_reachable_when_origin_exists() {
    let repo = TestGitRepo::new();
    repo.run_git(&[
        "remote",
        "add",
        "origin",
        "https://github.com/github/gitignore.git",
    ]);

    let output = Command::new(env!("CARGO_BIN_EXE_repo-guard"))
        .args(["remote", "check", "--require-origin"])
        .current_dir(repo.root())
        .output()
        .expect("remote check");

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}
