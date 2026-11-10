//! End-to-end pre-push and outgoing-history acceptance tests (local bare remotes).

mod common;

use common::git_fixture::TestGitRepo;
use common::push_fixture::{
    attach_bare_remote, default_branch, git_push, init_repo_guard, pre_push_stdin_line,
    run_pre_push_hook, run_repo_guard, seed_and_push, seed_repo, write_large_file,
    write_lfs_pointer, BareRemote, DEFAULT_BRANCH, ZERO_SHA,
};

#[test]
fn e2e_new_branch_push_blocks_large_blob_and_leaves_bare_unchanged() {
    let repo = TestGitRepo::new();
    init_repo_guard(&repo);
    let bare = BareRemote::new();
    attach_bare_remote(&repo, &bare, "origin");

    write_large_file(&repo, "new-branch-big.bin", 55);
    repo.git_add("new-branch-big.bin");
    repo.git_commit_no_verify("large on new branch");

    let branch = default_branch(&repo);
    let before = bare.head();
    let push = git_push(&repo, &["-u", "origin", &branch]);
    assert!(!push.status.success(), "push must be blocked");
    assert_eq!(bare.head(), before, "bare remote must remain unchanged");
}

#[test]
fn e2e_existing_branch_push_blocks_new_large_blob() {
    let repo = TestGitRepo::new();
    init_repo_guard(&repo);
    let bare = BareRemote::new();
    attach_bare_remote(&repo, &bare, "origin");
    let branch = DEFAULT_BRANCH;
    seed_and_push(&repo, &bare, branch);

    write_large_file(&repo, "existing-branch.bin", 55);
    repo.git_add("existing-branch.bin");
    repo.git_commit_no_verify("large follow-up");

    let before = bare.head();
    let push = git_push(&repo, &["origin", DEFAULT_BRANCH]);
    assert!(!push.status.success());
    assert_eq!(bare.head(), before);
}

#[test]
fn e2e_existing_branch_push_allows_small_blob() {
    let repo = TestGitRepo::new();
    init_repo_guard(&repo);
    let bare = BareRemote::new();
    attach_bare_remote(&repo, &bare, "origin");
    let branch = DEFAULT_BRANCH;
    seed_and_push(&repo, &bare, branch);

    repo.write_file("small-ok.txt", b"fine");
    repo.git_add("small-ok.txt");
    repo.git_commit("small follow-up");

    let before = bare.head();
    let push = git_push(&repo, &["origin", DEFAULT_BRANCH]);
    assert!(
        push.status.success(),
        "{}",
        String::from_utf8_lossy(&push.stderr)
    );
    assert_ne!(bare.head(), before);
}

#[test]
fn e2e_force_push_blocks_large_blob() {
    let repo = TestGitRepo::new();
    init_repo_guard(&repo);
    let bare = BareRemote::new();
    attach_bare_remote(&repo, &bare, "origin");
    let branch = DEFAULT_BRANCH;
    seed_and_push(&repo, &bare, branch);

    write_large_file(&repo, "force-big.bin", 55);
    repo.git_add("force-big.bin");
    repo.git_commit_no_verify("large for force");

    let before = bare.head();
    let push = git_push(&repo, &["--force", "origin", DEFAULT_BRANCH]);
    assert!(!push.status.success());
    assert_eq!(bare.head(), before);
}

#[test]
fn e2e_tag_push_blocks_large_blob() {
    let repo = TestGitRepo::new();
    init_repo_guard(&repo);
    let bare = BareRemote::new();
    attach_bare_remote(&repo, &bare, "origin");
    seed_repo(&repo);

    write_large_file(&repo, "tag-big.bin", 55);
    repo.git_add("tag-big.bin");
    repo.git_commit_no_verify("large for tag");
    repo.run_git(&["tag", "v-large"]);

    let before = bare.head();
    let push = git_push(&repo, &["origin", "v-large"]);
    assert!(!push.status.success());
    assert_eq!(bare.head(), before);
}

#[test]
fn e2e_multiple_refs_deduplicate_and_block_on_any_large_blob() {
    let repo = TestGitRepo::new();
    init_repo_guard(&repo);
    seed_repo(&repo);

    let good_head = repo.run_git(&["rev-parse", "HEAD"]).trim().to_string();
    write_large_file(&repo, "multi-big.bin", 55);
    repo.git_add("multi-big.bin");
    repo.git_commit_no_verify("bad branch");
    let bad_head = repo.run_git(&["rev-parse", "HEAD"]).trim().to_string();

    let branch = default_branch(&repo);
    let stdin = format!(
        "{}{}",
        pre_push_stdin_line(
            &format!("refs/heads/{branch}"),
            &good_head,
            &format!("refs/heads/{branch}"),
            ZERO_SHA,
        ),
        pre_push_stdin_line(
            "refs/heads/feature",
            &bad_head,
            "refs/heads/feature",
            ZERO_SHA,
        ),
    );

    let output = run_pre_push_hook(&repo, &stdin);
    assert_eq!(output.status.code(), Some(2));
    let text = String::from_utf8_lossy(&output.stdout);
    assert!(text.contains("multi-big.bin") || text.contains("blob.lfs_required"));
}

#[test]
fn e2e_ref_deletion_push_is_allowed() {
    let repo = TestGitRepo::new();
    init_repo_guard(&repo);
    let bare = BareRemote::new();
    attach_bare_remote(&repo, &bare, "origin");
    let branch = DEFAULT_BRANCH;
    seed_and_push(&repo, &bare, branch);

    let stdin = pre_push_stdin_line(ZERO_SHA, ZERO_SHA, "refs/heads/to-delete", &bare.head());
    let output = run_pre_push_hook(&repo, &stdin);
    assert!(output.status.success());
}

#[test]
fn e2e_unicode_path_outgoing_push_is_blocked() {
    let repo = TestGitRepo::new();
    init_repo_guard(&repo);
    seed_repo(&repo);
    let path = "linceo/SEAF/Бинар/large.bin";
    write_large_file(&repo, path, 55);
    repo.git_add(path);
    repo.git_commit_no_verify("unicode large");

    let head = repo.run_git(&["rev-parse", "HEAD"]).trim().to_string();
    let branch = default_branch(&repo);
    let stdin = pre_push_stdin_line(
        &format!("refs/heads/{branch}"),
        &head,
        &format!("refs/heads/{branch}"),
        ZERO_SHA,
    );
    let output = run_pre_push_hook(&repo, &stdin);
    assert_eq!(output.status.code(), Some(2));
    let text = String::from_utf8_lossy(&output.stdout);
    assert!(text.contains("Бинар") || text.contains("blob.lfs_required"));
}

#[test]
fn e2e_space_path_outgoing_push_is_blocked() {
    let repo = TestGitRepo::new();
    init_repo_guard(&repo);
    seed_repo(&repo);
    let path = "assets/my large file.bin";
    write_large_file(&repo, path, 55);
    repo.git_add(path);
    repo.git_commit_no_verify("space path large");

    let head = repo.run_git(&["rev-parse", "HEAD"]).trim().to_string();
    let branch = default_branch(&repo);
    let stdin = pre_push_stdin_line(
        &format!("refs/heads/{branch}"),
        &head,
        &format!("refs/heads/{branch}"),
        ZERO_SHA,
    );
    let output = run_pre_push_hook(&repo, &stdin);
    assert_eq!(output.status.code(), Some(2));
    let text = String::from_utf8_lossy(&output.stdout);
    assert!(text.contains("my large file.bin") || text.contains("blob.lfs_required"));
}

#[test]
fn e2e_lfs_pointer_blob_passes_outgoing_push() {
    let repo = TestGitRepo::new();
    init_repo_guard(&repo);
    seed_repo(&repo);
    write_lfs_pointer(&repo, "tracked.bin", 1024);
    repo.git_add("tracked.bin");
    repo.git_commit_no_verify("lfs pointer commit");

    let head = repo.run_git(&["rev-parse", "HEAD"]).trim().to_string();
    let branch = default_branch(&repo);
    let stdin = pre_push_stdin_line(
        &format!("refs/heads/{branch}"),
        &head,
        &format!("refs/heads/{branch}"),
        ZERO_SHA,
    );
    let output = run_pre_push_hook(&repo, &stdin);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stdout)
    );
}

#[test]
fn e2e_zero_remote_object_id_validates_conservatively() {
    let repo = TestGitRepo::new();
    init_repo_guard(&repo);
    write_large_file(&repo, "zero-remote.bin", 55);
    repo.git_add("zero-remote.bin");
    repo.git_commit_no_verify("zero remote id");

    let head = repo.run_git(&["rev-parse", "HEAD"]).trim().to_string();
    let branch = default_branch(&repo);
    let stdin = pre_push_stdin_line(
        &format!("refs/heads/{branch}"),
        &head,
        &format!("refs/heads/{branch}"),
        ZERO_SHA,
    );
    let output = run_pre_push_hook(&repo, &stdin);
    assert_eq!(output.status.code(), Some(2));
}

#[test]
fn e2e_unavailable_remote_baseline_strict_blocks_via_hook() {
    let repo = TestGitRepo::new();
    init_repo_guard(&repo);
    seed_repo(&repo);
    repo.write_file("ok.txt", b"ok");
    repo.git_add("ok.txt");
    repo.git_commit("ok commit");

    let head = repo.run_git(&["rev-parse", "HEAD"]).trim().to_string();
    let branch = default_branch(&repo);
    let missing = "deadbeefdeadbeefdeadbeefdeadbeefdeadbeef";
    let stdin = pre_push_stdin_line(
        &format!("refs/heads/{branch}"),
        &head,
        &format!("refs/heads/{branch}"),
        missing,
    );
    let output = run_pre_push_hook(&repo, &stdin);
    assert_eq!(output.status.code(), Some(2));
    let text = String::from_utf8_lossy(&output.stdout);
    assert!(text.contains("unresolved_baseline"));
}

#[test]
fn e2e_advisory_history_check_allows_with_warning() {
    let repo = TestGitRepo::new();
    init_repo_guard(&repo);
    seed_repo(&repo);
    repo.write_file("ok.txt", b"ok");
    repo.git_add("ok.txt");
    repo.git_commit("ok commit");

    let output = run_repo_guard(
        &repo,
        &["history", "check", "--range", "HEAD~1..HEAD", "--advisory"],
    );
    assert!(output.status.success());
}

#[test]
fn e2e_hook_defaults_to_strict_for_unresolved_baseline() {
    let repo = TestGitRepo::new();
    init_repo_guard(&repo);
    seed_repo(&repo);

    let head = repo.run_git(&["rev-parse", "HEAD"]).trim().to_string();
    let missing = "deadbeefdeadbeefdeadbeefdeadbeefdeadbeef";
    let branch = default_branch(&repo);
    let stdin = pre_push_stdin_line(
        &format!("refs/heads/{branch}"),
        &head,
        &format!("refs/heads/{branch}"),
        missing,
    );
    let output = run_pre_push_hook(&repo, &stdin);
    assert_eq!(output.status.code(), Some(2));
}
