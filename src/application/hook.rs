//! Git hook command handlers (`hook pre-commit`, `hook pre-push`).

use std::io::{self, Read};
use std::path::Path;

use crate::application::check::load_repo_config;
use crate::application::check::run_check;
use crate::application::history_check::inspect_outgoing_ref_updates;
use crate::domain::exit_code::ExitCode;
use crate::domain::history::{build_history_report, evaluate_outgoing_blobs, UnresolvedPolicy};
use crate::domain::prepush::parse_pre_push_stdin;
use crate::domain::report::{CommandReport, Issue};
use crate::infrastructure::process::{ProcessRunner, SystemProcessRunner};
use crate::infrastructure::repository::git_repo_root;

pub fn run_hook_pre_commit_system(cwd: &Path) -> (CommandReport, ExitCode) {
    let (report, code) = run_check(&SystemProcessRunner, cwd);
    let mut hook_report = CommandReport::new("hook pre-commit", code);
    hook_report.issues = report.issues;
    hook_report.summary = report.summary;
    (hook_report.finalize(code), code)
}

pub fn run_hook_pre_push_system(cwd: &Path, stdin: &mut dyn Read) -> (CommandReport, ExitCode) {
    run_hook_pre_push(&SystemProcessRunner, cwd, stdin)
}

pub fn run_hook_pre_push<R: ProcessRunner>(
    runner: &R,
    cwd: &Path,
    stdin: &mut dyn Read,
) -> (CommandReport, ExitCode) {
    let mut buffer = String::new();
    stdin.read_to_string(&mut buffer).ok();

    let Some(root) = git_repo_root(runner, cwd) else {
        let report = CommandReport::new("hook pre-push", ExitCode::RepoOrConfigError)
            .with_issue(Issue::error(
                "repo.not_git",
                "Current directory is not inside a Git repository",
            ))
            .finalize(ExitCode::RepoOrConfigError);
        return (report, ExitCode::RepoOrConfigError);
    };

    let config = match load_repo_config(&root) {
        Ok(config) => config,
        Err(message) => {
            let report = CommandReport::new("hook pre-push", ExitCode::RepoOrConfigError)
                .with_issue(Issue::error("config.missing", message))
                .finalize(ExitCode::RepoOrConfigError);
            return (report, ExitCode::RepoOrConfigError);
        }
    };

    let updates = parse_pre_push_stdin(&buffer);
    if updates.is_empty() {
        let report = CommandReport::new("hook pre-push", ExitCode::Success)
            .with_issue(Issue::info(
                "hook.pre_push",
                "No ref updates on stdin; no outgoing-range validation was performed",
            ))
            .with_summary(serde_json::json!({
                "refs_stdin_bytes": buffer.len(),
                "refs_parsed": 0,
            }))
            .finalize(ExitCode::Success);
        return (report, ExitCode::Success);
    }

    let inspection = match inspect_outgoing_ref_updates(runner, &root, &updates) {
        Ok(value) => value,
        Err(message) => {
            let report = CommandReport::new("hook pre-push", ExitCode::GitCommandFailure)
                .with_issue(
                    Issue::error(
                        "hook.pre_push.git_failed",
                        "Failed to inspect outgoing objects",
                    )
                    .with_detail(message),
                )
                .finalize(ExitCode::GitCommandFailure);
            return (report, ExitCode::GitCommandFailure);
        }
    };

    let evaluation = evaluate_outgoing_blobs(&config, &inspection.blobs);
    let (mut report, code) = build_history_report(
        "hook pre-push",
        &inspection,
        &evaluation,
        UnresolvedPolicy::Strict,
    );
    report.summary = serde_json::json!({
        "refs_stdin_bytes": buffer.len(),
        "refs_parsed": updates.len(),
        "summary": report.summary,
    });
    (report.finalize(code), code)
}

pub fn run_hook_pre_push_stdio(cwd: &Path) -> (CommandReport, ExitCode) {
    run_hook_pre_push_system(cwd, &mut io::stdin())
}
