//! Pure logic for the `check` command report.

use crate::domain::exit_code::ExitCode;
use crate::domain::policy::{PolicyEvaluation, StagedBlob};
use crate::domain::report::{CommandReport, Issue};

pub fn build_check_report(
    evaluation: &PolicyEvaluation,
    staged_count: usize,
) -> (CommandReport, ExitCode) {
    let exit_code = if evaluation.has_violations() {
        ExitCode::PolicyViolation
    } else {
        ExitCode::Success
    };

    let mut report = CommandReport::new("check", exit_code);
    for issue in &evaluation.issues {
        report = report.with_issue(issue.clone());
    }

    if !evaluation.repair_commands.is_empty() {
        report = report.with_issue(Issue::info(
            "repair.commands",
            "Copy and paste these commands to repair the staged index:",
        ));
        for cmd in &evaluation.repair_commands {
            report = report.with_issue(Issue::info("repair.command", cmd));
        }
    }

    if staged_count == 0 && !evaluation.has_violations() {
        report = report.with_issue(Issue::info("check.empty", "No staged files to validate"));
    }

    let summary = serde_json::json!({
        "staged_files": staged_count,
        "violations": evaluation.violation_count,
        "repair_commands": evaluation.repair_commands,
    });

    (report.with_summary(summary).finalize(exit_code), exit_code)
}

pub fn staged_blob_count(blobs: &[StagedBlob]) -> usize {
    blobs.len()
}
