//! Git and Git LFS process probes.

use std::path::{Path, PathBuf};

use crate::domain::doctor::DoctorFindings;
use crate::infrastructure::config::try_load_config;
use crate::infrastructure::process::{ProcessProbeError, ProcessRunner};
use crate::infrastructure::remote::{check_origin_reachable, list_remotes};
use crate::support::constants::CONFIG_FILE_NAME;

/// Collect read-only doctor findings using the provided process runner.
pub fn collect_doctor_findings<R: ProcessRunner>(runner: &R, cwd: &Path) -> DoctorFindings {
    let mut findings = DoctorFindings::default();

    match probe_git(runner, cwd) {
        Ok((available, version)) => {
            findings.git_available = Some(available);
            findings.git_version = version;
        }
        Err(err) => {
            findings.git_available = None;
            findings.git_probe_error = Some(err.to_string());
        }
    }

    if findings.git_available == Some(true) {
        findings.inside_git_repo = Some(false);
        match probe_git_root(runner, cwd) {
            Ok(Some(root)) => {
                findings.inside_git_repo = Some(true);
                findings.repo_root = Some(root.display().to_string());
                findings.config_file_exists = Some(root.join(CONFIG_FILE_NAME).is_file());
                findings.hooks_path = Some(probe_hooks_path(runner, &root));
                if findings.config_file_exists == Some(true) {
                    match try_load_config(&root) {
                        Ok(Some(_)) => findings.config_valid = Some(true),
                        Ok(None) => {}
                        Err(err) => {
                            findings.config_valid = Some(false);
                            findings.config_error = Some(err.to_string());
                        }
                    }
                }
                if let Ok(remotes) = list_remotes(runner, &root) {
                    let has_origin = remotes.iter().any(|r| r.name == "origin");
                    findings.remotes = remotes
                        .iter()
                        .map(|r| format!("{} -> {}", r.name, r.url))
                        .collect();
                    if has_origin {
                        findings.origin_reachable = check_origin_reachable(runner, &root);
                    }
                }
            }
            Ok(None) => {
                findings.inside_git_repo = Some(false);
            }
            Err(_) => {
                findings.inside_git_repo = None;
            }
        }
    }

    if findings.git_available == Some(true) {
        match probe_git_lfs(runner, cwd) {
            Ok((available, version)) => {
                findings.git_lfs_available = Some(available);
                findings.git_lfs_version = version;
            }
            Err(_) => {
                findings.git_lfs_available = None;
            }
        }
    }

    findings
}

pub(crate) fn probe_git<R: ProcessRunner>(
    runner: &R,
    cwd: &Path,
) -> Result<(bool, Option<String>), ProcessProbeError> {
    let cwd_str = path_to_str(cwd);
    match runner.run("git", &["--version"], Some(cwd_str)) {
        Ok(output) if output.success() => Ok((true, output.first_line_stdout())),
        Ok(_) => Ok((false, None)),
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => Ok((false, None)),
        Err(source) => Err(ProcessProbeError::Io {
            program: "git".into(),
            source,
        }),
    }
}

pub(crate) fn probe_git_lfs<R: ProcessRunner>(
    runner: &R,
    cwd: &Path,
) -> Result<(bool, Option<String>), ProcessProbeError> {
    let cwd_str = path_to_str(cwd);
    match runner.run("git", &["lfs", "version"], Some(cwd_str)) {
        Ok(output) if output.success() => Ok((true, output.first_line_stdout())),
        Ok(_) => Ok((false, None)),
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => Ok((false, None)),
        Err(source) => Err(ProcessProbeError::Io {
            program: "git lfs".into(),
            source,
        }),
    }
}

pub(crate) fn probe_git_root<R: ProcessRunner>(
    runner: &R,
    cwd: &Path,
) -> Result<Option<PathBuf>, ProcessProbeError> {
    let cwd_str = path_to_str(cwd);
    let output = runner
        .run("git", &["rev-parse", "--show-toplevel"], Some(cwd_str))
        .map_err(|source| ProcessProbeError::Io {
            program: "git".into(),
            source,
        })?;

    if output.success() {
        let root = output.stdout.trim();
        if root.is_empty() {
            Ok(None)
        } else {
            Ok(Some(PathBuf::from(root)))
        }
    } else {
        Ok(None)
    }
}

pub(crate) fn probe_hooks_path<R: ProcessRunner>(runner: &R, repo_root: &Path) -> Option<String> {
    let cwd_str = path_to_str(repo_root);
    let output = runner
        .run(
            "git",
            &["config", "--local", "--get", "core.hooksPath"],
            Some(cwd_str),
        )
        .ok()?;

    if output.success() {
        let value = output.stdout.trim().to_string();
        if value.is_empty() {
            None
        } else {
            Some(value)
        }
    } else {
        None
    }
}

fn path_to_str(path: &Path) -> &str {
    path.to_str().unwrap_or(".")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::process::{FakeProcessRunner, ProcessOutput};
    use std::io;
    use tempfile::tempdir;

    fn ok_stdout(text: &str) -> io::Result<ProcessOutput> {
        Ok(ProcessOutput {
            status_code: 0,
            stdout: text.into(),
            stderr: String::new(),
        })
    }

    fn fail() -> io::Result<ProcessOutput> {
        Ok(ProcessOutput {
            status_code: 1,
            stdout: String::new(),
            stderr: String::new(),
        })
    }

    #[test]
    fn collects_findings_for_healthy_repo() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        std::fs::write(root.join(".repo-guard.toml"), "schema_version = 1\n").unwrap();

        let runner = FakeProcessRunner::default()
            .insert("git", &["--version"], ok_stdout("git version 2.43.0"))
            .insert(
                "git",
                &["rev-parse", "--show-toplevel"],
                ok_stdout(root.to_str().unwrap()),
            )
            .insert("git", &["lfs", "version"], ok_stdout("git-lfs/3.4.0"))
            .insert(
                "git",
                &["config", "--local", "--get", "core.hooksPath"],
                ok_stdout(".githooks"),
            );

        let findings = collect_doctor_findings(&runner, root);
        assert_eq!(findings.git_available, Some(true));
        assert_eq!(findings.git_lfs_available, Some(true));
        assert_eq!(findings.inside_git_repo, Some(true));
        assert_eq!(findings.config_file_exists, Some(true));
        assert_eq!(findings.hooks_path, Some(Some(".githooks".into())));
    }

    #[test]
    fn reports_missing_git() {
        let dir = tempdir().unwrap();
        let runner = FakeProcessRunner::default().insert(
            "git",
            &["--version"],
            Err(io::Error::new(io::ErrorKind::NotFound, "git not found")),
        );

        let findings = collect_doctor_findings(&runner, dir.path());
        assert_eq!(findings.git_available, Some(false));
        assert!(findings.inside_git_repo.is_none());
    }

    #[test]
    fn reports_not_inside_git_repo() {
        let dir = tempdir().unwrap();
        let runner = FakeProcessRunner::default()
            .insert("git", &["--version"], ok_stdout("git version 2.43.0"))
            .insert("git", &["rev-parse", "--show-toplevel"], fail())
            .insert("git", &["lfs", "version"], ok_stdout("git-lfs/3.4.0"));

        let findings = collect_doctor_findings(&runner, dir.path());
        assert_eq!(findings.inside_git_repo, Some(false));
        assert!(findings.config_file_exists.is_none());
    }
}
