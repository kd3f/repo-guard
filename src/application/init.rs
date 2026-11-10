//! Application workflow for `repo-guard init`.

use std::path::Path;

use crate::domain::compatibility::{analyze_hook_environment, enforce_hook_ownership};
use crate::domain::exit_code::ExitCode;
use crate::domain::init::{
    build_init_plan, build_init_report, default_init_config, has_blocking_conflicts, InitAction,
    InitPlan,
};
use crate::domain::report::CommandReport;
use crate::infrastructure::config::write_config;
use crate::infrastructure::hook_environment::collect_hook_environment;
use crate::infrastructure::hooks::{PRE_COMMIT_HOOK, PRE_PUSH_HOOK};
use crate::infrastructure::process::{ProcessRunner, SystemProcessRunner};
use crate::infrastructure::repository::{
    collect_init_context, ensure_hook_file, ensure_repo_guard_hooks_executable,
    git_lfs_install_local, git_repo_root, read_hook_file, set_hooks_path,
};
use crate::support::constants::{
    PRE_COMMIT_HOOK as PRE_COMMIT_NAME, PRE_PUSH_HOOK as PRE_PUSH_NAME,
};

pub fn run_init_system(cwd: &Path, apply: bool) -> (CommandReport, ExitCode) {
    run_init(&SystemProcessRunner, cwd, apply)
}

pub fn run_init<R: ProcessRunner>(
    runner: &R,
    cwd: &Path,
    apply: bool,
) -> (CommandReport, ExitCode) {
    let context = collect_init_context(runner, cwd);
    let repo_root = git_repo_root(runner, cwd);
    let existing_pre_commit = repo_root
        .as_ref()
        .and_then(|root| read_hook_file(root, PRE_COMMIT_NAME));
    let existing_pre_push = repo_root
        .as_ref()
        .and_then(|root| read_hook_file(root, PRE_PUSH_NAME));

    let mut plan = build_init_plan(
        &context,
        PRE_COMMIT_HOOK,
        PRE_PUSH_HOOK,
        existing_pre_commit.as_deref(),
        existing_pre_push.as_deref(),
    );

    if let Some(signals) = collect_hook_environment(runner, cwd) {
        let hook_env = analyze_hook_environment(&signals);
        enforce_hook_ownership(&mut plan, &hook_env);
    }

    let apply_blocked = apply && has_blocking_conflicts(&plan);
    let mut executed = Vec::new();

    if apply && !has_blocking_conflicts(&plan) {
        if let Some(root) = repo_root {
            executed = execute_init_plan(runner, &root, &plan);
        }
    }

    build_init_report(&plan, &context, apply, &executed, apply_blocked)
}

pub(crate) fn execute_init_plan<R: ProcessRunner>(
    runner: &R,
    repo_root: &Path,
    plan: &InitPlan,
) -> Vec<String> {
    let mut executed = Vec::new();

    for action in &plan.actions {
        match action {
            InitAction::CreateConfig { relative_path } => {
                if write_config(repo_root, &default_init_config()).is_ok() {
                    executed.push(format!("Created {relative_path}"));
                }
            }
            InitAction::CreateHook { name } => {
                let content = if name == PRE_COMMIT_NAME {
                    PRE_COMMIT_HOOK
                } else {
                    PRE_PUSH_HOOK
                };
                if ensure_hook_file(repo_root, name, content).unwrap_or(false) {
                    executed.push(format!("Created .githooks/{name}"));
                }
            }
            InitAction::SetHooksPath { value } => {
                if set_hooks_path(runner, repo_root, value).is_ok() {
                    executed.push(format!("Set core.hooksPath to {value}"));
                }
            }
            InitAction::RunGitLfsInstallLocal => {
                if git_lfs_install_local(runner, repo_root).is_ok() {
                    executed.push("Ran git lfs install --local".into());
                }
            }
        }
    }

    if let Err(message) =
        ensure_repo_guard_hooks_executable(repo_root, PRE_COMMIT_HOOK, PRE_PUSH_HOOK)
    {
        executed.push(format!(
            "Could not ensure hook executable permissions: {message}"
        ));
    }

    executed
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::process::{FakeProcessRunner, ProcessOutput};
    use tempfile::tempdir;

    fn ok(text: &str) -> ProcessOutput {
        ProcessOutput {
            status_code: 0,
            stdout: text.into(),
            stderr: String::new(),
        }
    }

    #[test]
    fn init_dry_run_does_not_create_files() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        std::fs::write(root.join(".git"), "gitdir: nowhere\n").ok();

        let runner = FakeProcessRunner::default()
            .insert("git", &["--version"], Ok(ok("git version 2.43.0")))
            .insert(
                "git",
                &["rev-parse", "--show-toplevel"],
                Ok(ok(&root.display().to_string())),
            )
            .insert("git", &["lfs", "version"], Ok(ok("git-lfs/3.4.0")));

        let (_, code) = run_init(&runner, root, false);
        assert_eq!(code, ExitCode::Success);
        assert!(!root.join(".repo-guard.toml").exists());
    }
}
