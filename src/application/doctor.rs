//! Application workflows and command orchestration.

use std::path::Path;

use crate::domain::compatibility::analyze_hook_environment;
use crate::domain::doctor::build_doctor_report;
use crate::domain::exit_code::ExitCode;
use crate::domain::report::{CommandReport, Issue};
use crate::infrastructure::git::collect_doctor_findings;
use crate::infrastructure::hook_environment::collect_hook_environment;
use crate::infrastructure::process::{ProcessRunner, SystemProcessRunner};

/// Run the read-only `doctor` workflow.
pub fn run_doctor<R: ProcessRunner>(runner: &R, cwd: &Path) -> (CommandReport, ExitCode) {
    let findings = collect_doctor_findings(runner, cwd);
    let (mut report, exit_code) = build_doctor_report(&findings);

    if let Some(signals) = collect_hook_environment(runner, cwd) {
        let env = analyze_hook_environment(&signals);
        report = report.with_issue(Issue::info(
            "hooks.operating_mode",
            format!(
                "Recommended operating mode: {} (primary manager: {})",
                env.recommended_mode.as_str(),
                env.primary_manager.as_str()
            ),
        ));
        if !env.allows_standalone_hook_setup {
            report = report.with_issue(Issue::warning(
                "hooks.integration_recommended",
                "Another hook manager owns orchestration; use repo-guard integrate for safe adoption",
            ));
        }
    }

    (report.finalize(exit_code), exit_code)
}

/// Run `doctor` using the system process runner.
pub fn run_doctor_system(cwd: &Path) -> (CommandReport, ExitCode) {
    run_doctor(&SystemProcessRunner, cwd)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::process::{FakeProcessRunner, ProcessOutput};
    use tempfile::tempdir;

    #[test]
    fn run_doctor_integrates_findings_and_report() {
        let dir = tempdir().unwrap();
        let root = dir.path();

        let runner = FakeProcessRunner::default()
            .insert(
                "git",
                &["--version"],
                Ok(ProcessOutput {
                    status_code: 0,
                    stdout: "git version 2.43.0".into(),
                    stderr: String::new(),
                }),
            )
            .insert(
                "git",
                &["rev-parse", "--show-toplevel"],
                Ok(ProcessOutput {
                    status_code: 0,
                    stdout: format!("{}\n", root.display()),
                    stderr: String::new(),
                }),
            )
            .insert(
                "git",
                &["lfs", "version"],
                Ok(ProcessOutput {
                    status_code: 0,
                    stdout: "git-lfs/3.4.0".into(),
                    stderr: String::new(),
                }),
            )
            .insert(
                "git",
                &["config", "--local", "--get", "core.hooksPath"],
                Ok(ProcessOutput {
                    status_code: 1,
                    stdout: String::new(),
                    stderr: String::new(),
                }),
            );

        let (report, exit_code) = run_doctor(&runner, root);
        assert_eq!(exit_code, ExitCode::Success);
        assert_eq!(report.command, "doctor");
        assert!(report
            .issues
            .iter()
            .any(|i| i.code == "hooks.not_configured"));
    }
}
