//! Application workflow for `repo-guard scan`.

use std::path::Path;

use crate::domain::config::RepoGuardConfig;
use crate::domain::exit_code::ExitCode;
use crate::domain::report::{CommandReport, Issue};
use crate::domain::scan::{evaluate_scan, ScanEvaluation};
use crate::infrastructure::config::try_load_config;
use crate::infrastructure::process::{ProcessRunner, SystemProcessRunner};
use crate::infrastructure::repository::git_repo_root;
use crate::infrastructure::worktree::collect_worktree_files;

pub fn run_scan_system(cwd: &Path) -> (CommandReport, ExitCode) {
    run_scan(&SystemProcessRunner, cwd)
}

pub fn run_scan<R: ProcessRunner>(runner: &R, cwd: &Path) -> (CommandReport, ExitCode) {
    let Some(root) = git_repo_root(runner, cwd) else {
        return repo_error("scan", "Current directory is not inside a Git repository");
    };

    let config = match try_load_config(&root) {
        Ok(Some(c)) => c,
        Ok(None) => return repo_error("scan", ".repo-guard.toml was not found"),
        Err(err) => {
            return repo_error("scan", &format!(".repo-guard.toml is invalid: {err}"));
        }
    };

    let files = match collect_worktree_files(runner, &root) {
        Ok(f) => f,
        Err(err) => {
            let report = CommandReport::new("scan", ExitCode::GitCommandFailure)
                .with_issue(
                    Issue::error("scan.failed", "Failed to scan working tree").with_detail(err),
                )
                .finalize(ExitCode::GitCommandFailure);
            return (report, ExitCode::GitCommandFailure);
        }
    };

    build_scan_report(&config, evaluate_scan(files, &config))
}

fn build_scan_report(
    config: &RepoGuardConfig,
    evaluation: ScanEvaluation,
) -> (CommandReport, ExitCode) {
    let mut report = CommandReport::new("scan", ExitCode::Success);
    for issue in &evaluation.issues {
        report = report.with_issue(issue.clone());
    }

    let largest: Vec<_> = evaluation
        .largest_files(5)
        .into_iter()
        .map(|f| serde_json::json!({"path": f.path, "size_bytes": f.size_bytes}))
        .collect();

    let summary = serde_json::json!({
        "total_scanned": evaluation.total_scanned(),
        "ignored": evaluation.ignored_count(),
        "large_files": evaluation.large_files(config).len(),
        "largest_files": largest,
    });

    (
        report.with_summary(summary).finalize(ExitCode::Success),
        ExitCode::Success,
    )
}

fn repo_error(command: &str, message: &str) -> (CommandReport, ExitCode) {
    let report = CommandReport::new(command, ExitCode::RepoOrConfigError)
        .with_issue(Issue::error("repo.error", message))
        .finalize(ExitCode::RepoOrConfigError);
    (report, ExitCode::RepoOrConfigError)
}
