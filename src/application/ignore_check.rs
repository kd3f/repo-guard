//! Application workflow for `repo-guard ignore check`.

use std::path::Path;

use crate::domain::exit_code::ExitCode;
use crate::domain::ignore::{build_near_matches, evaluate_ignore_paths};
use crate::domain::report::{CommandReport, Issue};
use crate::infrastructure::config::try_load_config;
use crate::infrastructure::process::{ProcessRunner, SystemProcessRunner};
use crate::infrastructure::repository::git_repo_root;
use crate::infrastructure::staged::list_staged_blobs;

pub fn run_ignore_check_system(cwd: &Path) -> (CommandReport, ExitCode) {
    run_ignore_check(&SystemProcessRunner, cwd)
}

pub fn run_ignore_check<R: ProcessRunner>(runner: &R, cwd: &Path) -> (CommandReport, ExitCode) {
    let Some(root) = git_repo_root(runner, cwd) else {
        return error_report(
            "ignore check",
            ExitCode::RepoOrConfigError,
            "repo.not_git",
            "Not in a Git repository",
        );
    };

    let config = match try_load_config(&root) {
        Ok(Some(c)) => c,
        Ok(None) => {
            return error_report(
                "ignore check",
                ExitCode::RepoOrConfigError,
                "config.missing",
                ".repo-guard.toml was not found",
            )
        }
        Err(err) => {
            return error_report(
                "ignore check",
                ExitCode::RepoOrConfigError,
                "config.invalid",
                &err.to_string(),
            )
        }
    };

    if config.ignored_paths.is_empty() {
        let report = CommandReport::new("ignore check", ExitCode::Success)
            .with_issue(Issue::info(
                "ignore.empty",
                "No ignored_paths configured in .repo-guard.toml",
            ))
            .finalize(ExitCode::Success);
        return (report, ExitCode::Success);
    }

    let existence: Vec<_> = config
        .ignored_paths
        .iter()
        .map(|p| (p.clone(), root.join(p).exists()))
        .collect();

    let ignored_by_git: Vec<_> = config
        .ignored_paths
        .iter()
        .map(|p| (p.clone(), git_check_ignore(runner, &root, p)))
        .collect();

    let staged = list_staged_blobs(runner, &root).unwrap_or_default();
    let staged_despite: Vec<_> = staged
        .iter()
        .filter(|b| b.is_ignored)
        .map(|b| b.path.clone())
        .collect();

    let near_matches = build_near_matches(&config.ignored_paths, &root);
    let evaluation = evaluate_ignore_paths(
        &config.ignored_paths,
        &existence,
        &ignored_by_git,
        &staged_despite,
        &near_matches,
    );

    let exit_code = if evaluation.errors > 0 {
        ExitCode::PolicyViolation
    } else {
        ExitCode::Success
    };

    let mut report = CommandReport::new("ignore check", exit_code);
    for issue in evaluation.issues {
        report = report.with_issue(issue);
    }

    let near_match_json: Vec<_> = evaluation
        .near_matches
        .iter()
        .map(|m| {
            serde_json::json!({
                "configured": m.configured,
                "candidate": m.candidate,
            })
        })
        .collect();

    (
        report
            .with_summary(serde_json::json!({
                "configured_paths": config.ignored_paths.len(),
                "warnings": evaluation.warnings,
                "errors": evaluation.errors,
                "near_matches": near_match_json,
            }))
            .finalize(exit_code),
        exit_code,
    )
}

fn git_check_ignore<R: ProcessRunner>(runner: &R, repo_root: &Path, path: &str) -> bool {
    let cwd = repo_root.to_str().unwrap_or(".");
    runner
        .run("git", &["check-ignore", "-q", "--", path], Some(cwd))
        .map(|o| o.success())
        .unwrap_or(false)
}

fn error_report(
    command: &str,
    code: ExitCode,
    issue_code: &str,
    message: &str,
) -> (CommandReport, ExitCode) {
    (
        CommandReport::new(command, code)
            .with_issue(Issue::error(issue_code, message))
            .finalize(code),
        code,
    )
}
