//! Application workflow for read-only hook-environment detection.

use std::path::Path;

use crate::domain::compatibility::{analyze_hook_environment, HookEnvironment};
use crate::domain::exit_code::ExitCode;
use crate::domain::report::{CommandReport, Issue};
use crate::infrastructure::git::probe_git;
use crate::infrastructure::hook_environment::collect_hook_environment;
use crate::infrastructure::process::{ProcessRunner, SystemProcessRunner};

pub fn run_detect_system(cwd: &Path) -> (CommandReport, ExitCode) {
    run_detect(&SystemProcessRunner, cwd)
}

pub fn run_detect<R: ProcessRunner>(runner: &R, cwd: &Path) -> (CommandReport, ExitCode) {
    let (available, _) = probe_git(runner, cwd).unwrap_or((false, None));

    if !available {
        let report =
            CommandReport::new("detect", ExitCode::MissingDependency).with_issue(Issue::error(
                "git.missing",
                "Git is not installed or not callable on PATH",
            ));
        return (
            report.finalize(ExitCode::MissingDependency),
            ExitCode::MissingDependency,
        );
    }

    let Some(signals) = collect_hook_environment(runner, cwd) else {
        let report =
            CommandReport::new("detect", ExitCode::RepoOrConfigError).with_issue(Issue::error(
                "repo.not_git",
                "Current directory is not inside a Git repository",
            ));
        return (
            report.finalize(ExitCode::RepoOrConfigError),
            ExitCode::RepoOrConfigError,
        );
    };

    build_detect_report(analyze_hook_environment(&signals))
}

fn build_detect_report(env: HookEnvironment) -> (CommandReport, ExitCode) {
    let mut report = CommandReport::new("detect", ExitCode::Success);

    report = report.with_issue(Issue::info(
        "detect.mode",
        format!(
            "Recommended operating mode: {}",
            env.recommended_mode.as_str()
        ),
    ));
    report = report.with_issue(Issue::info(
        "detect.manager",
        format!("Primary hook manager: {}", env.primary_manager.as_str()),
    ));

    if let Some(path) = &env.hooks_path {
        report = report.with_issue(
            Issue::info("detect.hooks_path", format!("core.hooksPath is \"{path}\""))
                .with_detail(path.clone()),
        );
    } else {
        report = report.with_issue(Issue::info(
            "detect.hooks_path",
            "core.hooksPath is not set locally",
        ));
    }

    for item in &env.evidence {
        let detail = item
            .detail
            .as_ref()
            .map(|d| format!("{} ({d})", item.source))
            .unwrap_or_else(|| item.source.clone());
        report = report.with_issue(
            Issue::info(
                format!("detect.evidence.{}", item.manager.as_str()),
                format!("Found {} evidence", item.manager.as_str()),
            )
            .with_detail(detail),
        );
    }

    for action in &env.safe_actions {
        report = report.with_issue(Issue::info("detect.next_action", action));
    }

    if !env.allows_standalone_hook_setup {
        report = report.with_issue(Issue::warning(
            "detect.standalone_blocked",
            "Standalone init/install hook setup is not safe in this repository",
        ));
    }

    let summary = serde_json::json!({
        "recommended_mode": env.recommended_mode.as_str(),
        "primary_manager": env.primary_manager.as_str(),
        "hooks_path": env.hooks_path,
        "allows_standalone_hook_setup": env.allows_standalone_hook_setup,
        "evidence": env.evidence,
        "safe_actions": env.safe_actions,
    });

    (
        report.with_summary(summary).finalize(ExitCode::Success),
        ExitCode::Success,
    )
}
