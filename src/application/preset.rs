//! Application workflows for preset commands.

use std::fs;
use std::path::Path;

use crate::domain::exit_code::ExitCode;
use crate::domain::preset::{list_builtin_presets, plan_preset_apply, preset_apply_issues};
use crate::domain::report::{CommandReport, Issue};
use crate::infrastructure::presets::preset_body;
use crate::infrastructure::process::{ProcessRunner, SystemProcessRunner};
use crate::infrastructure::repository::git_repo_root;

pub fn run_preset_list() -> (CommandReport, ExitCode) {
    let mut report = CommandReport::new("preset list", ExitCode::Success);
    for preset in list_builtin_presets() {
        report = report.with_issue(Issue::info(
            format!("preset.{}", preset.name),
            format!("{} — {}", preset.name, preset.description),
        ));
    }
    (
        report
            .with_summary(serde_json::json!({
                "presets": list_builtin_presets().iter().map(|p| &p.name).collect::<Vec<_>>(),
            }))
            .finalize(ExitCode::Success),
        ExitCode::Success,
    )
}

pub fn run_preset_apply_system(cwd: &Path, preset: &str, apply: bool) -> (CommandReport, ExitCode) {
    run_preset_apply(&SystemProcessRunner, cwd, preset, apply)
}

pub fn run_preset_apply<R: ProcessRunner>(
    runner: &R,
    cwd: &Path,
    preset: &str,
    apply: bool,
) -> (CommandReport, ExitCode) {
    let Some(root) = git_repo_root(runner, cwd) else {
        return error_report("preset apply", "Not in a Git repository");
    };

    let body = match preset_body(preset) {
        Ok(body) => body,
        Err(err) => return error_report("preset apply", &err),
    };

    let gitignore_path = root.join(".gitignore");
    let existing = fs::read_to_string(&gitignore_path).ok();
    let plan = match plan_preset_apply(preset, &body, existing.as_deref()) {
        Ok(plan) => plan,
        Err(err) => return error_report("preset apply", &err),
    };

    if apply && (plan.would_create || plan.would_update) {
        let _ = fs::write(&gitignore_path, &plan.content);
    }

    let mut report = CommandReport::new("preset apply", ExitCode::Success);
    for issue in preset_apply_issues(&plan, apply) {
        report = report.with_issue(issue);
    }

    (
        report
            .with_summary(serde_json::json!({
                "preset": preset,
                "apply": apply,
                "would_create": plan.would_create,
                "would_update": plan.would_update,
            }))
            .finalize(ExitCode::Success),
        ExitCode::Success,
    )
}

fn error_report(command: &str, message: &str) -> (CommandReport, ExitCode) {
    (
        CommandReport::new(command, ExitCode::RepoOrConfigError)
            .with_issue(Issue::error("preset.error", message))
            .finalize(ExitCode::RepoOrConfigError),
        ExitCode::RepoOrConfigError,
    )
}
