//! Shared report types for command output.

use serde::Serialize;

use crate::domain::exit_code::ExitCode;

/// JSON schema version for machine-readable reports.
pub const REPORT_SCHEMA_VERSION: u32 = 1;

/// Severity of a reported issue.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum IssueSeverity {
    Error,
    Warning,
    Info,
}

/// A single finding in a command report.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Issue {
    pub code: String,
    pub severity: IssueSeverity,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

impl Issue {
    pub fn error(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            severity: IssueSeverity::Error,
            message: message.into(),
            detail: None,
        }
    }

    pub fn warning(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            severity: IssueSeverity::Warning,
            message: message.into(),
            detail: None,
        }
    }

    pub fn info(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            severity: IssueSeverity::Info,
            message: message.into(),
            detail: None,
        }
    }

    pub fn with_detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }
}

/// Overall status derived from issues and exit code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ReportStatus {
    Passed,
    Failed,
}

/// Command report suitable for text and JSON presenters.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CommandReport {
    pub schema_version: u32,
    pub command: String,
    pub status: ReportStatus,
    pub exit_code: i32,
    pub issues: Vec<Issue>,
    pub summary: serde_json::Value,
}

impl CommandReport {
    pub fn new(command: impl Into<String>, exit_code: ExitCode) -> Self {
        Self {
            schema_version: REPORT_SCHEMA_VERSION,
            command: command.into(),
            status: ReportStatus::Passed,
            exit_code: exit_code.as_i32(),
            issues: Vec::new(),
            summary: serde_json::json!({}),
        }
    }

    pub fn with_issue(mut self, issue: Issue) -> Self {
        self.issues.push(issue);
        self
    }

    pub fn with_summary(mut self, summary: serde_json::Value) -> Self {
        self.summary = summary;
        self
    }

    pub fn finalize(mut self, exit_code: ExitCode) -> Self {
        self.exit_code = exit_code.as_i32();
        self.status = if exit_code == ExitCode::Success {
            ReportStatus::Passed
        } else {
            ReportStatus::Failed
        };
        self
    }
}
