//! Application workflow for `repo-guard integrate`.

use std::path::Path;

use crate::application::detect::run_detect;
use crate::domain::exit_code::ExitCode;
use crate::domain::integrate::{
    integration_snippet, integration_title, ordering_note, IntegrationTarget,
};
use crate::domain::report::{CommandReport, Issue};
use crate::infrastructure::process::{ProcessRunner, SystemProcessRunner};

pub enum IntegrateRequest {
    Detect,
    Print(IntegrationTarget),
}

pub fn run_integrate_system(cwd: &Path, request: IntegrateRequest) -> (CommandReport, ExitCode) {
    run_integrate(&SystemProcessRunner, cwd, request)
}

pub fn run_integrate<R: ProcessRunner>(
    runner: &R,
    cwd: &Path,
    request: IntegrateRequest,
) -> (CommandReport, ExitCode) {
    match request {
        IntegrateRequest::Detect => run_detect(runner, cwd),
        IntegrateRequest::Print(target) => build_print_report(target, cwd, runner),
    }
}

fn build_print_report<R: ProcessRunner>(
    target: IntegrationTarget,
    cwd: &Path,
    runner: &R,
) -> (CommandReport, ExitCode) {
    let title = integration_title(target);
    let snippet = integration_snippet(target);
    let mut report = CommandReport::new(format!("integrate {title}"), ExitCode::Success);

    report = report.with_issue(Issue::info(
        "integrate.snippet",
        format!("Review and add this {title} integration snippet manually"),
    ));
    report = report.with_issue(Issue::info("integrate.content", snippet));

    let (detect_report, _) = run_detect(runner, cwd);

    if let Some(mode) = detect_report
        .summary
        .get("recommended_mode")
        .and_then(|v| v.as_str())
    {
        report = report.with_issue(Issue::info(
            "integrate.mode",
            format!("Detected operating mode: {mode}"),
        ));
    }
    if let Some(manager) = detect_report
        .summary
        .get("primary_manager")
        .and_then(|v| v.as_str())
    {
        let note = match manager {
            "pre_commit" => ordering_note(
                crate::domain::compatibility::HookManagerKind::PreCommit,
                crate::domain::compatibility::OperatingMode::Integration,
            ),
            "husky" => ordering_note(
                crate::domain::compatibility::HookManagerKind::Husky,
                crate::domain::compatibility::OperatingMode::Integration,
            ),
            "lefthook" => ordering_note(
                crate::domain::compatibility::HookManagerKind::Lefthook,
                crate::domain::compatibility::OperatingMode::Integration,
            ),
            _ => "Run repo-guard check after hooks that mutate the staged index.".into(),
        };
        report = report.with_issue(Issue::info("integrate.ordering", note));
    }

    report = report.with_issue(Issue::info(
        "integrate.manual",
        "Repo Guard does not modify external hook manager configuration automatically",
    ));

    let summary = serde_json::json!({
        "target": title,
        "snippet_lines": snippet.lines().count(),
    });

    (
        report.with_summary(summary).finalize(ExitCode::Success),
        ExitCode::Success,
    )
}
