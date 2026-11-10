//! Application workflow for `repo-guard history check`.

use std::path::Path;

use crate::application::check::load_repo_config;
use crate::domain::exit_code::ExitCode;
use crate::domain::history::{
    build_history_report, count_skipped_deletions, evaluate_outgoing_blobs, merge_outgoing_plans,
    unresolved_plans, OutgoingInspection, UnresolvedPolicy,
};
use crate::domain::prepush::{parse_history_range, plan_outgoing_refs, RefUpdate};
use crate::domain::report::{CommandReport, Issue};
use crate::infrastructure::git_objects::{collect_unique_blobs, object_exists, rev_list_objects};
use crate::infrastructure::process::{ProcessRunner, SystemProcessRunner};
use crate::infrastructure::repository::git_repo_root;

pub struct HistoryCheckRequest {
    pub range: Option<String>,
    pub all: bool,
    pub advisory: bool,
}

pub fn run_history_check_system(
    cwd: &Path,
    request: HistoryCheckRequest,
) -> (CommandReport, ExitCode) {
    run_history_check(&SystemProcessRunner, cwd, request)
}

pub fn run_history_check<R: ProcessRunner>(
    runner: &R,
    cwd: &Path,
    request: HistoryCheckRequest,
) -> (CommandReport, ExitCode) {
    let Some(root) = git_repo_root(runner, cwd) else {
        let report = CommandReport::new("history check", ExitCode::RepoOrConfigError)
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
            let report = CommandReport::new("history check", ExitCode::RepoOrConfigError)
                .with_issue(Issue::error("config.missing", message))
                .finalize(ExitCode::RepoOrConfigError);
            return (report, ExitCode::RepoOrConfigError);
        }
    };

    let unresolved_policy = if request.advisory {
        UnresolvedPolicy::Advisory
    } else {
        UnresolvedPolicy::Strict
    };

    let inspection = if request.all {
        inspect_all_history(runner, &root)
    } else if let Some(range) = request.range {
        inspect_history_range(runner, &root, &range)
    } else {
        let report = CommandReport::new("history check", ExitCode::RepoOrConfigError)
            .with_issue(Issue::error(
                "history.range_required",
                "Provide --range <base>..<head> or --all",
            ))
            .finalize(ExitCode::RepoOrConfigError);
        return (report, ExitCode::RepoOrConfigError);
    };

    let inspection = match inspection {
        Ok(value) => value,
        Err(message) => {
            let report = CommandReport::new("history check", ExitCode::GitCommandFailure)
                .with_issue(
                    Issue::error("history.git_failed", "Failed to inspect Git history")
                        .with_detail(message),
                )
                .finalize(ExitCode::GitCommandFailure);
            return (report, ExitCode::GitCommandFailure);
        }
    };

    let evaluation = evaluate_outgoing_blobs(&config, &inspection.blobs);
    build_history_report("history check", &inspection, &evaluation, unresolved_policy)
}

pub fn inspect_outgoing_ref_updates<R: ProcessRunner>(
    runner: &R,
    repo_root: &Path,
    updates: &[RefUpdate],
) -> Result<OutgoingInspection, String> {
    let exists = |sha: &str| object_exists(runner, repo_root, sha);
    let plans = plan_outgoing_refs(updates, exists);
    let specs = merge_outgoing_plans(&plans);
    let blobs = collect_unique_blobs(runner, repo_root, &specs)?;
    Ok(OutgoingInspection {
        blobs,
        unresolved_refs: unresolved_plans(&plans),
        skipped_deletions: count_skipped_deletions(&plans),
        inspected_ranges: specs,
    })
}

fn inspect_history_range<R: ProcessRunner>(
    runner: &R,
    repo_root: &Path,
    range: &str,
) -> Result<OutgoingInspection, String> {
    let (base, head) = parse_history_range(range)?;
    let spec = format!("{base}..{head}");
    let blobs = collect_unique_blobs(runner, repo_root, std::slice::from_ref(&spec))?;
    Ok(OutgoingInspection {
        blobs,
        unresolved_refs: Vec::new(),
        skipped_deletions: 0,
        inspected_ranges: vec![spec],
    })
}

fn inspect_all_history<R: ProcessRunner>(
    runner: &R,
    repo_root: &Path,
) -> Result<OutgoingInspection, String> {
    let head_output = runner
        .run("git", &["rev-parse", "HEAD"], Some(path_to_str(repo_root)))
        .map_err(|e| e.to_string())?;
    if !head_output.success() {
        return Err(head_output.stderr);
    }
    let head = head_output.stdout.trim().to_string();
    let spec = head.clone();
    let _ = rev_list_objects(runner, repo_root, &spec)?;
    let blobs = collect_unique_blobs(runner, repo_root, std::slice::from_ref(&spec))?;
    Ok(OutgoingInspection {
        blobs,
        unresolved_refs: Vec::new(),
        skipped_deletions: 0,
        inspected_ranges: vec![format!("{head} (all reachable history)")],
    })
}

fn path_to_str(path: &Path) -> &str {
    path.to_str().unwrap_or(".")
}
