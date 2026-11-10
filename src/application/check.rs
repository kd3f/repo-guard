//! Application workflow for `repo-guard check`.

use std::path::Path;

use crate::domain::check::build_check_report;
use crate::domain::config::RepoGuardConfig;
use crate::domain::exit_code::ExitCode;
use crate::domain::lfs_check::{evaluate_lfs_consistency, parse_gitattributes};
use crate::domain::policy::{evaluate_staged_blobs, PolicyEvaluation};
use crate::domain::report::{CommandReport, Issue};
use crate::infrastructure::config::{load_config, try_load_config};
use crate::infrastructure::git::probe_git_lfs;
use crate::infrastructure::process::{ProcessRunner, SystemProcessRunner};
use crate::infrastructure::repository::git_repo_root;
use crate::infrastructure::staged::list_staged_blobs;
use std::fs;

pub fn run_check_system(cwd: &Path) -> (CommandReport, ExitCode) {
    run_check(&SystemProcessRunner, cwd)
}

pub fn run_check<R: ProcessRunner>(runner: &R, cwd: &Path) -> (CommandReport, ExitCode) {
    let Some(root) = git_repo_root(runner, cwd) else {
        let report = CommandReport::new("check", ExitCode::RepoOrConfigError)
            .with_issue(Issue::error(
                "repo.not_git",
                "Current directory is not inside a Git repository",
            ))
            .finalize(ExitCode::RepoOrConfigError);
        return (report, ExitCode::RepoOrConfigError);
    };

    let config = match try_load_config(&root) {
        Ok(Some(config)) => config,
        Ok(None) => {
            let report = CommandReport::new("check", ExitCode::RepoOrConfigError)
                .with_issue(Issue::error(
                    "config.missing",
                    ".repo-guard.toml was not found; run repo-guard init --apply",
                ))
                .finalize(ExitCode::RepoOrConfigError);
            return (report, ExitCode::RepoOrConfigError);
        }
        Err(err) => {
            let report = CommandReport::new("check", ExitCode::RepoOrConfigError)
                .with_issue(
                    Issue::error("config.invalid", ".repo-guard.toml is invalid")
                        .with_detail(err.to_string()),
                )
                .finalize(ExitCode::RepoOrConfigError);
            return (report, ExitCode::RepoOrConfigError);
        }
    };

    run_check_with_config(runner, cwd, &root, &config)
}

pub fn run_check_with_config<R: ProcessRunner>(
    runner: &R,
    cwd: &Path,
    repo_root: &Path,
    config: &RepoGuardConfig,
) -> (CommandReport, ExitCode) {
    let blobs = match list_staged_blobs(runner, repo_root) {
        Ok(blobs) => blobs,
        Err(err) => {
            let report = CommandReport::new("check", ExitCode::GitCommandFailure)
                .with_issue(
                    Issue::error("git.staged_failed", "Failed to inspect staged Git index")
                        .with_detail(err),
                )
                .finalize(ExitCode::GitCommandFailure);
            return (report, ExitCode::GitCommandFailure);
        }
    };

    let mut evaluation = evaluate_staged_blobs(config, &blobs);
    merge_lfs_consistency(runner, repo_root, cwd, &blobs, &mut evaluation);

    build_check_report(&evaluation, blobs.len())
}

fn merge_lfs_consistency<R: ProcessRunner>(
    runner: &R,
    repo_root: &Path,
    cwd: &Path,
    blobs: &[crate::domain::policy::StagedBlob],
    evaluation: &mut PolicyEvaluation,
) {
    let attrs = fs::read_to_string(repo_root.join(".gitattributes")).unwrap_or_default();
    let rules = parse_gitattributes(&attrs);
    let git_lfs_installed = probe_git_lfs(runner, cwd)
        .map(|(ok, _)| ok)
        .unwrap_or(false);
    let staged_info: Vec<_> = blobs
        .iter()
        .map(|b| (b.path.clone(), b.is_lfs_pointer, true))
        .collect();
    let lfs_eval = evaluate_lfs_consistency(git_lfs_installed, &rules, &staged_info);
    for issue in lfs_eval.issues {
        if issue.code == "lfs.not_pointer" || issue.code == "git_lfs.missing" {
            evaluation.violation_count += 1;
        }
        evaluation.issues.push(issue);
    }
}

pub fn load_repo_config(repo_root: &Path) -> Result<RepoGuardConfig, String> {
    load_config(repo_root).map_err(|e| e.to_string())
}
