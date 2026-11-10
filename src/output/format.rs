//! Human-readable and JSON report presenters.

use crate::domain::report::{CommandReport, IssueSeverity};

/// Supported output formats for command reports.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    Text,
    Json,
}

/// Render a command report to stdout content.
pub fn render_report(
    report: &CommandReport,
    format: OutputFormat,
) -> Result<String, serde_json::Error> {
    match format {
        OutputFormat::Text => Ok(render_text(report)),
        OutputFormat::Json => render_json(report),
    }
}

fn render_text(report: &CommandReport) -> String {
    let mut lines = Vec::new();
    lines.push(format!("repo-guard {}", report.command));
    lines.push(format!(
        "Status: {} (exit {})",
        status_label(report.status),
        report.exit_code
    ));
    lines.push(String::new());

    if report.issues.is_empty() {
        lines.push("No issues found.".into());
    } else {
        lines.push("Checks:".into());
        for issue in &report.issues {
            lines.push(format!(
                "  [{}] {}: {}",
                severity_label(issue.severity),
                issue.code,
                issue.message
            ));
            if let Some(detail) = &issue.detail {
                lines.push(format!("         {detail}"));
            }
        }
    }

    lines.join("\n")
}

fn render_json(report: &CommandReport) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(report)
}

fn status_label(status: crate::domain::report::ReportStatus) -> &'static str {
    match status {
        crate::domain::report::ReportStatus::Passed => "passed",
        crate::domain::report::ReportStatus::Failed => "failed",
    }
}

fn severity_label(severity: IssueSeverity) -> &'static str {
    match severity {
        IssueSeverity::Error => "ERROR",
        IssueSeverity::Warning => "WARN",
        IssueSeverity::Info => "INFO",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::exit_code::ExitCode;
    use crate::domain::report::{Issue, ReportStatus};

    #[test]
    fn text_output_includes_issues() {
        let report = CommandReport::new("doctor", ExitCode::Success)
            .with_issue(Issue::warning("git.available", "Git is installed"))
            .finalize(ExitCode::Success);

        let text = render_text(&report);
        assert!(text.contains("repo-guard doctor"));
        assert!(text.contains("git.available"));
    }

    #[test]
    fn json_output_matches_schema_fields() {
        let report = CommandReport::new("doctor", ExitCode::Success)
            .with_issue(Issue::warning("git.available", "Git is installed"))
            .with_summary(serde_json::json!({"git_available": true}))
            .finalize(ExitCode::Success);

        let json = render_json(&report).unwrap();
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(value["schema_version"], 1);
        assert_eq!(value["status"], "passed");
        assert_eq!(value["exit_code"], 0);
        assert!(value["issues"].is_array());
        assert!(value["summary"].is_object());
    }

    #[test]
    fn failed_report_renders_failed_status() {
        let report = CommandReport::new("doctor", ExitCode::MissingDependency)
            .with_issue(Issue::error("git.missing", "Git is not installed"))
            .finalize(ExitCode::MissingDependency);

        assert_eq!(report.status, ReportStatus::Failed);
        let text = render_text(&report);
        assert!(text.contains("failed"));
    }
}
