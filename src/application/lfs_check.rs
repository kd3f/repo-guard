//! Application workflow for `repo-guard lfs check`.

use std::fs;
use std::path::Path;

use crate::domain::exit_code::ExitCode;
use crate::domain::lfs_check::{evaluate_lfs_consistency, parse_gitattributes};
use crate::domain::report::{CommandReport, Issue};
use crate::infrastructure::git::probe_git_lfs;
use crate::infrastructure::process::{ProcessRunner, SystemProcessRunner};
use crate::infrastructure::repository::git_repo_root;
use crate::infrastructure::staged::list_staged_blobs;

pub fn run_lfs_check_system(cwd: &Path) -> (CommandReport, ExitCode) {
    run_lfs_check(&SystemProcessRunner, cwd)
}

pub fn run_lfs_check<R: ProcessRunner>(runner: &R, cwd: &Path) -> (CommandReport, ExitCode) {
    let Some(root) = git_repo_root(runner, cwd) else {
        return error_report("lfs check", "Not in a Git repository");
    };

    let attrs_path = root.join(".gitattributes");
    let attrs_content = fs::read_to_string(&attrs_path).unwrap_or_default();
    let rules = parse_gitattributes(&attrs_content);

    let git_lfs_installed = probe_git_lfs(runner, cwd)
        .map(|(ok, _)| ok)
        .unwrap_or(false);

    let staged = list_staged_blobs(runner, &root).unwrap_or_default();
    let staged_info: Vec<_> = staged
        .iter()
        .map(|b| (b.path.clone(), b.is_lfs_pointer, true))
        .collect();

    let evaluation = evaluate_lfs_consistency(git_lfs_installed, &rules, &staged_info);
    let rule_count = rules.len();
    let exit_code = if evaluation.violations > 0 {
        ExitCode::PolicyViolation
    } else {
        ExitCode::Success
    };

    let mut report = CommandReport::new("lfs check", exit_code);
    for issue in evaluation.issues {
        report = report.with_issue(issue);
    }

    if rules.is_empty() && attrs_content.is_empty() {
        report = report.with_issue(Issue::info(
            "lfs.no_gitattributes",
            ".gitattributes not found or empty",
        ));
    }

    (
        report
            .with_summary(serde_json::json!({
                "rules": rule_count,
                "violations": evaluation.violations,
            }))
            .finalize(exit_code),
        exit_code,
    )
}

fn error_report(command: &str, message: &str) -> (CommandReport, ExitCode) {
    (
        CommandReport::new(command, ExitCode::RepoOrConfigError)
            .with_issue(Issue::error("repo.error", message))
            .finalize(ExitCode::RepoOrConfigError),
        ExitCode::RepoOrConfigError,
    )
}
