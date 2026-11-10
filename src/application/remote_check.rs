//! Application workflow for `repo-guard remote check`.

use std::path::Path;

use crate::domain::exit_code::ExitCode;
use crate::domain::remote::evaluate_remotes;
use crate::domain::report::CommandReport;
use crate::infrastructure::process::{ProcessRunner, SystemProcessRunner};
use crate::infrastructure::remote::{check_origin_reachable, list_remotes};
use crate::infrastructure::repository::git_repo_root;

pub fn run_remote_check_system(cwd: &Path, require_origin: bool) -> (CommandReport, ExitCode) {
    run_remote_check(&SystemProcessRunner, cwd, require_origin)
}

pub fn run_remote_check<R: ProcessRunner>(
    runner: &R,
    cwd: &Path,
    require_origin: bool,
) -> (CommandReport, ExitCode) {
    let Some(root) = git_repo_root(runner, cwd) else {
        return (
            CommandReport::new("remote check", ExitCode::RepoOrConfigError)
                .finalize(ExitCode::RepoOrConfigError),
            ExitCode::RepoOrConfigError,
        );
    };

    let remotes = list_remotes(runner, &root).unwrap_or_default();
    let origin_reachable = if remotes.iter().any(|r| r.name == "origin") {
        check_origin_reachable(runner, &root)
    } else {
        None
    };

    let evaluation = evaluate_remotes(remotes, require_origin, origin_reachable);
    let exit_code = if evaluation.errors > 0 {
        ExitCode::PolicyViolation
    } else {
        ExitCode::Success
    };

    let mut report = CommandReport::new("remote check", exit_code);
    for issue in evaluation.issues {
        report = report.with_issue(issue);
    }

    (
        report
            .with_summary(serde_json::json!({
                "remote_count": evaluation.remotes.len(),
                "errors": evaluation.errors,
            }))
            .finalize(exit_code),
        exit_code,
    )
}
