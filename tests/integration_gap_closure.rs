//! Iteration 9 gap-closure integration tests (Unicode paths, near-match ignore).

mod common;

use std::process::Command;

use common::git_fixture::TestGitRepo;

fn init_repo(repo: &TestGitRepo) {
    let rg = env!("CARGO_BIN_EXE_repo-guard");
    Command::new(rg)
        .args(["init", "--apply"])
        .current_dir(repo.root())
        .output()
        .expect("init apply");
}

fn sample_config(ignored_paths: &str) -> String {
    format!(
        r#"
schema_version = 1
lfs_threshold_mib = 50
github_safety_ceiling_mib = 95
block_small_lfs_files = false
output_format = "text"
ignored_paths = [{ignored_paths}]
"#
    )
}

#[test]
fn check_accepts_small_unicode_path() {
    let repo = TestGitRepo::new();
    init_repo(&repo);
    let rg = env!("CARGO_BIN_EXE_repo-guard");
    let path = "café/petit.txt";

    repo.write_file(path, b"hello");
    repo.git_add(path);

    let output = Command::new(rg)
        .args(["check"])
        .current_dir(repo.root())
        .output()
        .expect("check");

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stdout)
    );
}

#[test]
fn check_blocks_large_unicode_path_with_repair_commands() {
    let repo = TestGitRepo::new();
    init_repo(&repo);
    let rg = env!("CARGO_BIN_EXE_repo-guard");
    let path = "данные/большой.bin";

    repo.write_file(path, &vec![0u8; 52 * 1024 * 1024]);
    repo.git_add(path);

    let text_out = Command::new(rg)
        .args(["check"])
        .current_dir(repo.root())
        .output()
        .expect("check text");

    assert_eq!(text_out.status.code(), Some(2));
    let text = String::from_utf8_lossy(&text_out.stdout);
    assert!(text.contains("blob.lfs_required"));
    assert!(text.contains("git lfs track"));
    assert!(text.contains("большой.bin"));

    let json_out = Command::new(rg)
        .args(["check", "--format", "json"])
        .current_dir(repo.root())
        .output()
        .expect("check json");

    assert_eq!(json_out.status.code(), Some(2));
    let value: serde_json::Value =
        serde_json::from_slice(&json_out.stdout).expect("valid JSON stdout");
    assert_eq!(value["schema_version"], 1);
    assert_eq!(value["exit_code"], 2);
    assert!(value["issues"].as_array().is_some());
}

#[test]
fn check_handles_unicode_path_with_spaces() {
    let repo = TestGitRepo::new();
    init_repo(&repo);
    let rg = env!("CARGO_BIN_EXE_repo-guard");
    let path = "español spaced dir/fichero.dat";

    repo.write_file(path, b"data");
    repo.git_add(path);

    let output = Command::new(rg)
        .args(["check", "--format", "json"])
        .current_dir(repo.root())
        .output()
        .expect("check");

    assert!(output.status.success());
    serde_json::from_slice::<serde_json::Value>(&output.stdout).expect("valid JSON");
}

#[test]
fn ignore_check_surfaces_near_match_for_duplicated_directory() {
    let repo = TestGitRepo::new();
    init_repo(&repo);
    let rg = env!("CARGO_BIN_EXE_repo-guard");

    repo.write_file("linceo/SEAF/SEAF/Binari/artifact.bin", b"x");
    std::fs::write(
        repo.root().join(".repo-guard.toml"),
        sample_config(r#""linceo/SEAF/Binari/""#),
    )
    .unwrap();

    let text_out = Command::new(rg)
        .args(["ignore", "check"])
        .current_dir(repo.root())
        .output()
        .expect("ignore check text");

    assert!(text_out.status.success());
    let text = String::from_utf8_lossy(&text_out.stdout);
    assert!(text.contains("ignore.missing_path"));
    assert!(text.contains("ignore.near_match"));
    assert!(text.contains("linceo/SEAF/SEAF/Binari"));
    assert!(!text.contains("ignore.confirmed"));

    let json_out = Command::new(rg)
        .args(["ignore", "check", "--format", "json"])
        .current_dir(repo.root())
        .output()
        .expect("ignore check json");

    assert!(json_out.status.success());
    let value: serde_json::Value =
        serde_json::from_slice(&json_out.stdout).expect("valid JSON stdout");
    let near = &value["summary"]["near_matches"];
    assert!(near.is_array());
    assert!(near.as_array().unwrap().iter().any(|entry| {
        entry["configured"] == "linceo/SEAF/Binari/"
            && entry["candidate"] == "linceo/SEAF/SEAF/Binari"
    }));
    assert!(!value["issues"]
        .as_array()
        .unwrap()
        .iter()
        .any(|i| i["code"] == "ignore.confirmed"));
}
