//! Pure logic for the `doctor` command.

use crate::domain::exit_code::ExitCode;
use crate::domain::report::{CommandReport, Issue};

/// Raw findings collected by infrastructure before report assembly.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DoctorFindings {
    pub git_available: Option<bool>,
    pub git_version: Option<String>,
    pub git_lfs_available: Option<bool>,
    pub git_lfs_version: Option<String>,
    pub inside_git_repo: Option<bool>,
    pub repo_root: Option<String>,
    pub config_file_exists: Option<bool>,
    pub hooks_path: Option<Option<String>>,
    pub git_probe_error: Option<String>,
    pub config_valid: Option<bool>,
    pub config_error: Option<String>,
    pub remotes: Vec<String>,
    pub origin_reachable: Option<bool>,
}

/// Evaluate doctor findings into a command report and exit code.
pub fn build_doctor_report(findings: &DoctorFindings) -> (CommandReport, ExitCode) {
    let mut report = CommandReport::new("doctor", ExitCode::Success);
    let mut exit_code = ExitCode::Success;

    match findings.git_available {
        Some(true) => {
            let version = findings.git_version.as_deref().unwrap_or("unknown version");
            report = report.with_issue(Issue::info(
                "git.available",
                format!("Git is installed ({version})"),
            ));
        }
        Some(false) => {
            report = report.with_issue(Issue::error(
                "git.missing",
                "Git is not installed or not callable on PATH",
            ));
            exit_code = ExitCode::MissingDependency;
        }
        None => {
            let detail = findings
                .git_probe_error
                .as_deref()
                .unwrap_or("unknown error");
            report = report.with_issue(
                Issue::error("git.probe_failed", "Failed to probe Git availability")
                    .with_detail(detail),
            );
            exit_code = ExitCode::GitCommandFailure;
        }
    }

    if findings.git_available == Some(true) {
        match findings.inside_git_repo {
            Some(true) => {
                let root = findings.repo_root.as_deref().unwrap_or("unknown location");
                report = report.with_issue(Issue::info(
                    "repo.inside_git",
                    format!("Current directory is inside a Git repository ({root})"),
                ));
            }
            Some(false) => {
                report = report.with_issue(Issue::error(
                    "repo.not_git",
                    "Current directory is not inside a Git repository",
                ));
                exit_code = worse(exit_code, ExitCode::RepoOrConfigError);
            }
            None => {
                report = report.with_issue(Issue::error(
                    "repo.unknown",
                    "Could not determine whether the current directory is inside a Git repository",
                ));
                exit_code = worse(exit_code, ExitCode::GitCommandFailure);
            }
        }
    }

    match findings.git_lfs_available {
        Some(true) => {
            let version = findings
                .git_lfs_version
                .as_deref()
                .unwrap_or("unknown version");
            report = report.with_issue(Issue::info(
                "git_lfs.available",
                format!("Git LFS is installed ({version})"),
            ));
        }
        Some(false) => {
            report = report.with_issue(Issue::error(
                "git_lfs.missing",
                "Git LFS is not installed or not callable on PATH",
            ));
            exit_code = worse(exit_code, ExitCode::MissingDependency);
        }
        None if findings.git_available == Some(true) => {
            report = report.with_issue(Issue::error(
                "git_lfs.probe_failed",
                "Failed to probe Git LFS availability",
            ));
            exit_code = worse(exit_code, ExitCode::GitCommandFailure);
        }
        None => {}
    }

    match findings.config_file_exists {
        Some(true) => match findings.config_valid {
            Some(true) => {
                report = report.with_issue(Issue::info(
                    "config.valid",
                    ".repo-guard.toml exists and is valid",
                ));
            }
            Some(false) => {
                let detail = findings
                    .config_error
                    .as_deref()
                    .unwrap_or("invalid configuration");
                report = report.with_issue(
                    Issue::error("config.invalid", ".repo-guard.toml is invalid")
                        .with_detail(detail),
                );
                exit_code = worse(exit_code, ExitCode::RepoOrConfigError);
            }
            None => {
                report = report.with_issue(Issue::info(
                    "config.present",
                    ".repo-guard.toml exists in the repository root",
                ));
            }
        },
        Some(false) => {
            report = report.with_issue(Issue::warning(
                "config.missing",
                ".repo-guard.toml was not found in the repository root",
            ));
        }
        None => {}
    }

    if !findings.remotes.is_empty() {
        for remote in &findings.remotes {
            report = report.with_issue(Issue::info(
                "remote.configured",
                format!("Git remote configured: {remote}"),
            ));
        }
        match findings.origin_reachable {
            Some(true) => {
                report = report.with_issue(Issue::info(
                    "remote.origin_reachable",
                    "origin responded to a non-destructive connectivity check",
                ));
            }
            Some(false) => {
                report = report.with_issue(Issue::warning(
                    "remote.origin_unreachable",
                    "origin is configured but did not respond to git ls-remote",
                ));
            }
            None => {}
        }
    }

    match &findings.hooks_path {
        Some(Some(path)) if !path.trim().is_empty() => {
            report = report.with_issue(Issue::info(
                "hooks.configured",
                format!("core.hooksPath is configured ({path})"),
            ));
        }
        Some(Some(_)) | Some(None) => {
            report = report.with_issue(Issue::warning(
                "hooks.not_configured",
                "core.hooksPath is not configured for this repository",
            ));
        }
        None => {}
    }

    let summary = serde_json::json!({
        "git_available": findings.git_available,
        "git_lfs_available": findings.git_lfs_available,
        "inside_git_repo": findings.inside_git_repo,
        "config_file_exists": findings.config_file_exists,
        "config_valid": findings.config_valid,
        "remotes": findings.remotes,
        "origin_reachable": findings.origin_reachable,
        "hooks_path_configured": findings.hooks_path.as_ref().map(|p| p.as_ref().map(|s| !s.is_empty()).unwrap_or(false)),
    });

    report = report.with_summary(summary).finalize(exit_code);
    (report, exit_code)
}

fn worse(current: ExitCode, candidate: ExitCode) -> ExitCode {
    match (current, candidate) {
        (ExitCode::Success, c) => c,
        (c, ExitCode::Success) => c,
        (a, b) => {
            if a.as_i32() >= b.as_i32() {
                a
            } else {
                b
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_checks_pass_yields_success() {
        let findings = DoctorFindings {
            git_available: Some(true),
            git_version: Some("2.43.0".into()),
            git_lfs_available: Some(true),
            git_lfs_version: Some("3.4.0".into()),
            inside_git_repo: Some(true),
            repo_root: Some("/tmp/repo".into()),
            config_file_exists: Some(true),
            config_valid: Some(true),
            hooks_path: Some(Some(".githooks".into())),
            ..DoctorFindings::default()
        };

        let (report, exit_code) = build_doctor_report(&findings);
        assert_eq!(exit_code, ExitCode::Success);
        assert_eq!(report.status, crate::domain::report::ReportStatus::Passed);
        assert!(report.issues.iter().any(|i| i.code == "git.available"));
    }

    #[test]
    fn missing_git_yields_missing_dependency() {
        let findings = DoctorFindings {
            git_available: Some(false),
            ..DoctorFindings::default()
        };

        let (_, exit_code) = build_doctor_report(&findings);
        assert_eq!(exit_code, ExitCode::MissingDependency);
    }

    #[test]
    fn not_in_repo_yields_repo_error_when_git_available() {
        let findings = DoctorFindings {
            git_available: Some(true),
            git_version: Some("2.43.0".into()),
            git_lfs_available: Some(true),
            git_lfs_version: Some("3.4.0".into()),
            inside_git_repo: Some(false),
            config_file_exists: Some(false),
            hooks_path: Some(None),
            ..DoctorFindings::default()
        };

        let (_, exit_code) = build_doctor_report(&findings);
        assert_eq!(exit_code, ExitCode::RepoOrConfigError);
    }

    #[test]
    fn missing_git_lfs_yields_missing_dependency() {
        let findings = DoctorFindings {
            git_available: Some(true),
            git_version: Some("2.43.0".into()),
            git_lfs_available: Some(false),
            inside_git_repo: Some(true),
            repo_root: Some("/tmp/repo".into()),
            ..DoctorFindings::default()
        };

        let (_, exit_code) = build_doctor_report(&findings);
        assert_eq!(exit_code, ExitCode::MissingDependency);
    }

    #[test]
    fn missing_config_and_hooks_are_warnings_only() {
        let findings = DoctorFindings {
            git_available: Some(true),
            git_version: Some("2.43.0".into()),
            git_lfs_available: Some(true),
            git_lfs_version: Some("3.4.0".into()),
            inside_git_repo: Some(true),
            repo_root: Some("/tmp/repo".into()),
            config_file_exists: Some(false),
            hooks_path: Some(None),
            ..DoctorFindings::default()
        };

        let (report, exit_code) = build_doctor_report(&findings);
        assert_eq!(exit_code, ExitCode::Success);
        assert!(report.issues.iter().any(|i| i.code == "config.missing"));
        assert!(report
            .issues
            .iter()
            .any(|i| i.code == "hooks.not_configured"));
    }
}
