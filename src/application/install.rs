//! Application workflow for `repo-guard install`.

use std::path::Path;

use crate::application::init::execute_init_plan;
use crate::domain::compatibility::{analyze_hook_environment, enforce_hook_ownership};
use crate::domain::exit_code::ExitCode;
use crate::domain::init::build_init_report;
use crate::domain::init::has_blocking_conflicts;
use crate::domain::install::{build_install_plan, filter_install_actions, install_requires_config};
use crate::domain::report::{CommandReport, Issue};
use crate::infrastructure::hook_environment::collect_hook_environment;
use crate::infrastructure::hooks::{PRE_COMMIT_HOOK, PRE_PUSH_HOOK};
use crate::infrastructure::process::{ProcessRunner, SystemProcessRunner};
use crate::infrastructure::repository::{collect_init_context, git_repo_root, read_hook_file};
use crate::support::constants::{
    PRE_COMMIT_HOOK as PRE_COMMIT_NAME, PRE_PUSH_HOOK as PRE_PUSH_NAME,
};

pub fn run_install_system(cwd: &Path, apply: bool) -> (CommandReport, ExitCode) {
    run_install(&SystemProcessRunner, cwd, apply)
}

pub fn run_install<R: ProcessRunner>(
    runner: &R,
    cwd: &Path,
    apply: bool,
) -> (CommandReport, ExitCode) {
    let context = collect_init_context(runner, cwd);
    let repo_root = git_repo_root(runner, cwd);

    if let Some(conflict) = install_requires_config(&context) {
        let report = CommandReport::new("install", ExitCode::RepoOrConfigError)
            .with_issue(Issue::error(conflict.code, conflict.message))
            .finalize(ExitCode::RepoOrConfigError);
        return (report, ExitCode::RepoOrConfigError);
    }

    let existing_pre_commit = repo_root
        .as_ref()
        .and_then(|root| read_hook_file(root, PRE_COMMIT_NAME));
    let existing_pre_push = repo_root
        .as_ref()
        .and_then(|root| read_hook_file(root, PRE_PUSH_NAME));

    let raw_plan = build_install_plan(
        &context,
        PRE_COMMIT_HOOK,
        PRE_PUSH_HOOK,
        existing_pre_commit.as_deref(),
        existing_pre_push.as_deref(),
    );
    let mut plan = filter_install_actions(&raw_plan);

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

    let (mut report, exit_code) =
        build_init_report(&plan, &context, apply, &executed, apply_blocked);
    report.command = "install".into();
    (report.finalize(exit_code), exit_code)
}
