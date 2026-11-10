//! Install command domain logic (extends init safety rules).

use crate::domain::init::{InitAction, InitConflict, InitContext, InitPlan};
use crate::support::constants::{HOOKS_DIR_NAME, HOOKS_PATH_VALUE};

pub use crate::domain::init::build_init_plan;

/// Install uses the same planning rules as init for hooks and Git LFS setup.
pub fn build_install_plan(
    context: &InitContext,
    pre_commit_template: &str,
    pre_push_template: &str,
    existing_pre_commit: Option<&str>,
    existing_pre_push: Option<&str>,
) -> InitPlan {
    build_init_plan(
        context,
        pre_commit_template,
        pre_push_template,
        existing_pre_commit,
        existing_pre_push,
    )
}

pub fn install_requires_config(context: &InitContext) -> Option<InitConflict> {
    if context.inside_git_repo && !context.config_exists {
        Some(InitConflict {
            code: "config.missing",
            message: ".repo-guard.toml is required before install; run repo-guard init --apply"
                .into(),
        })
    } else {
        None
    }
}

pub fn filter_install_actions(plan: &InitPlan) -> InitPlan {
    InitPlan {
        actions: plan
            .actions
            .iter()
            .filter(|action| !matches!(action, InitAction::CreateConfig { .. }))
            .cloned()
            .collect(),
        conflicts: plan.conflicts.clone(),
        skipped: plan
            .skipped
            .iter()
            .filter(|s| !s.contains(".repo-guard.toml"))
            .cloned()
            .collect(),
    }
}

pub fn describe_install_action(action: &InitAction) -> String {
    match action {
        InitAction::CreateConfig { relative_path } => format!("Create {relative_path}"),
        InitAction::CreateHook { name } => format!("Create {HOOKS_DIR_NAME}/{name}"),
        InitAction::SetHooksPath { value } => format!("Set core.hooksPath to {value}"),
        InitAction::RunGitLfsInstallLocal => "Run git lfs install --local".into(),
    }
}

pub fn is_hooks_only_action(action: &InitAction) -> bool {
    !matches!(action, InitAction::CreateConfig { .. })
}

pub const EXPECTED_HOOKS_PATH: &str = HOOKS_PATH_VALUE;
