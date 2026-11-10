//! Pure logic for the `init` command.

use crate::domain::config::RepoGuardConfig;
use crate::domain::exit_code::ExitCode;
use crate::domain::report::{CommandReport, Issue};
use crate::support::constants::{
    CONFIG_FILE_NAME, HOOKS_DIR_NAME, HOOKS_PATH_VALUE, PRE_COMMIT_HOOK, PRE_PUSH_HOOK,
};

/// A planned file or Git configuration change.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InitAction {
    CreateConfig { relative_path: String },
    CreateHook { name: String },
    SetHooksPath { value: String },
    RunGitLfsInstallLocal,
}

/// Conflict that blocks silent overwrite.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InitConflict {
    pub code: &'static str,
    pub message: String,
}

/// Input state gathered before planning init actions.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct InitContext {
    pub inside_git_repo: bool,
    pub git_available: bool,
    pub git_lfs_available: bool,
    pub config_exists: bool,
    pub hooks_dir_exists: bool,
    pub pre_commit_exists: bool,
    pub pre_push_exists: bool,
    pub hooks_path: Option<String>,
    pub pre_commit_differs: bool,
    pub pre_push_differs: bool,
}

/// Planned init outcome before execution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InitPlan {
    pub actions: Vec<InitAction>,
    pub conflicts: Vec<InitConflict>,
    pub skipped: Vec<String>,
}

/// Normalize hook script text for cross-platform comparison.
pub fn normalize_hook_content(content: &str) -> String {
    content.replace("\r\n", "\n").trim().to_string()
}

pub fn hook_content_equivalent(existing: &str, template: &str) -> bool {
    normalize_hook_content(existing) == normalize_hook_content(template)
}

/// Build an init plan from repository state and embedded hook templates.
pub fn build_init_plan(
    context: &InitContext,
    pre_commit_template: &str,
    pre_push_template: &str,
    existing_pre_commit: Option<&str>,
    existing_pre_push: Option<&str>,
) -> InitPlan {
    let mut plan = InitPlan {
        actions: Vec::new(),
        conflicts: Vec::new(),
        skipped: Vec::new(),
    };

    if !context.git_available {
        return plan;
    }
    if !context.inside_git_repo {
        return plan;
    }

    if context.config_exists {
        plan.skipped.push(format!(
            "{CONFIG_FILE_NAME} already exists; not overwriting"
        ));
    } else {
        plan.actions.push(InitAction::CreateConfig {
            relative_path: CONFIG_FILE_NAME.into(),
        });
    }

    if !context.hooks_dir_exists {
        plan.actions.push(InitAction::CreateHook {
            name: PRE_COMMIT_HOOK.into(),
        });
        plan.actions.push(InitAction::CreateHook {
            name: PRE_PUSH_HOOK.into(),
        });
    } else {
        if context.pre_commit_exists {
            if existing_pre_commit
                .is_some_and(|content| !hook_content_equivalent(content, pre_commit_template))
            {
                plan.conflicts.push(InitConflict {
                    code: "hooks.pre_commit_exists",
                    message: format!(
                        "{HOOKS_DIR_NAME}/{PRE_COMMIT_HOOK} exists with different content"
                    ),
                });
            } else {
                plan.skipped.push(format!(
                    "{HOOKS_DIR_NAME}/{PRE_COMMIT_HOOK} already exists; not overwriting"
                ));
            }
        } else {
            plan.actions.push(InitAction::CreateHook {
                name: PRE_COMMIT_HOOK.into(),
            });
        }

        if context.pre_push_exists {
            if existing_pre_push
                .is_some_and(|content| !hook_content_equivalent(content, pre_push_template))
            {
                plan.conflicts.push(InitConflict {
                    code: "hooks.pre_push_exists",
                    message: format!(
                        "{HOOKS_DIR_NAME}/{PRE_PUSH_HOOK} exists with different content"
                    ),
                });
            } else {
                plan.skipped.push(format!(
                    "{HOOKS_DIR_NAME}/{PRE_PUSH_HOOK} already exists; not overwriting"
                ));
            }
        } else {
            plan.actions.push(InitAction::CreateHook {
                name: PRE_PUSH_HOOK.into(),
            });
        }
    }

    match context.hooks_path.as_deref() {
        Some(path) if path == HOOKS_PATH_VALUE => {
            plan.skipped.push(format!(
                "core.hooksPath already set to {HOOKS_PATH_VALUE}; no change needed"
            ));
        }
        Some(path) => {
            plan.conflicts.push(InitConflict {
                code: "hooks.path_conflict",
                message: format!(
                    "core.hooksPath is already set to \"{path}\"; refusing to override without resolving the conflict"
                ),
            });
        }
        None => {
            plan.actions.push(InitAction::SetHooksPath {
                value: HOOKS_PATH_VALUE.into(),
            });
        }
    }

    if context.git_lfs_available {
        plan.actions.push(InitAction::RunGitLfsInstallLocal);
    }

    plan
}

/// Conflicts that must block all `--apply` writes.
pub fn has_blocking_conflicts(plan: &InitPlan) -> bool {
    plan.conflicts.iter().any(|c| {
        matches!(
            c.code,
            "hooks.path_conflict" | "hooks.pre_commit_exists" | "hooks.pre_push_exists"
        )
    })
}

/// Conflicts that indicate hook ownership prevented standalone hook setup.
pub fn has_hook_ownership_conflict(plan: &InitPlan) -> bool {
    plan.conflicts
        .iter()
        .any(|c| c.code == "hooks.manager_owned")
}

/// Build init command report from plan and execution results.
pub fn build_init_report(
    plan: &InitPlan,
    context: &InitContext,
    apply: bool,
    executed: &[String],
    apply_blocked: bool,
) -> (CommandReport, ExitCode) {
    let mut report = CommandReport::new("init", ExitCode::Success);
    let mut exit_code = ExitCode::Success;

    if !context.git_available {
        report = report.with_issue(Issue::error(
            "git.missing",
            "Git is not installed or not callable on PATH",
        ));
        return (
            report.finalize(ExitCode::MissingDependency),
            ExitCode::MissingDependency,
        );
    }
    if !context.inside_git_repo {
        report = report.with_issue(Issue::error(
            "repo.not_git",
            "Current directory is not inside a Git repository",
        ));
        return (
            report.finalize(ExitCode::RepoOrConfigError),
            ExitCode::RepoOrConfigError,
        );
    }
    if !context.git_lfs_available {
        report = report.with_issue(Issue::warning(
            "git_lfs.missing",
            "Git LFS is not installed; init will skip git lfs install --local",
        ));
    }

    for conflict in &plan.conflicts {
        let issue = if conflict.code == "hooks.manager_owned" {
            Issue::warning(conflict.code, &conflict.message)
        } else {
            Issue::error(conflict.code, &conflict.message)
        };
        report = report.with_issue(issue);
        if matches!(
            conflict.code,
            "hooks.path_conflict" | "hooks.pre_commit_exists" | "hooks.pre_push_exists"
        ) {
            exit_code = ExitCode::RepoOrConfigError;
        }
    }

    for skip in &plan.skipped {
        report = report.with_issue(Issue::info("init.skipped", skip));
    }

    for action in &plan.actions {
        let message = match action {
            InitAction::CreateConfig { relative_path } => {
                format!("Create {relative_path} with default policy")
            }
            InitAction::CreateHook { name } => {
                format!("Create {HOOKS_DIR_NAME}/{name}")
            }
            InitAction::SetHooksPath { value } => {
                format!("Set core.hooksPath to {value}")
            }
            InitAction::RunGitLfsInstallLocal => "Run git lfs install --local".into(),
        };
        let code = if apply && !apply_blocked {
            "init.applied"
        } else {
            "init.planned"
        };
        report = report.with_issue(Issue::info(code, message));
    }

    for result in executed {
        report = report.with_issue(Issue::info("init.executed", result));
    }

    if !apply {
        report = report.with_issue(Issue::info(
            "init.dry_run",
            "Dry run only; pass --apply to write files and configure Git",
        ));
    } else if apply_blocked {
        report = report.with_issue(Issue::error(
            "init.blocked",
            "Apply blocked due to conflicts; resolve conflicts and retry",
        ));
    } else if has_hook_ownership_conflict(plan) {
        report = report.with_issue(Issue::info(
            "init.hooks_skipped",
            "Hook setup skipped because another manager owns hook orchestration",
        ));
    }

    let summary = serde_json::json!({
        "apply": apply,
        "planned_actions": plan.actions.len(),
        "conflicts": plan.conflicts.len(),
        "executed": executed.len(),
        "apply_blocked": apply_blocked,
    });

    (report.with_summary(summary).finalize(exit_code), exit_code)
}

/// Default configuration written by init.
pub fn default_init_config() -> RepoGuardConfig {
    RepoGuardConfig::default()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn healthy_context() -> InitContext {
        InitContext {
            inside_git_repo: true,
            git_available: true,
            git_lfs_available: true,
            config_exists: false,
            hooks_dir_exists: false,
            pre_commit_exists: false,
            pre_push_exists: false,
            hooks_path: None,
            pre_commit_differs: false,
            pre_push_differs: false,
        }
    }

    const PRE_COMMIT: &str = "pre-commit-template";
    const PRE_PUSH: &str = "pre-push-template";

    #[test]
    fn plan_includes_default_actions_for_fresh_repo() {
        let plan = build_init_plan(&healthy_context(), PRE_COMMIT, PRE_PUSH, None, None);
        assert!(plan
            .actions
            .iter()
            .any(|a| matches!(a, InitAction::CreateConfig { .. })));
        assert!(plan.conflicts.is_empty());
    }

    #[test]
    fn plan_detects_hooks_path_conflict() {
        let mut ctx = healthy_context();
        ctx.hooks_path = Some(".husky".into());
        let plan = build_init_plan(&ctx, PRE_COMMIT, PRE_PUSH, None, None);
        assert!(plan
            .conflicts
            .iter()
            .any(|c| c.code == "hooks.path_conflict"));
    }

    #[test]
    fn hook_content_equivalent_ignores_crlf() {
        assert!(hook_content_equivalent(
            "#!/bin/sh\r\nrepo-guard hook pre-commit\r\n",
            "#!/bin/sh\nrepo-guard hook pre-commit\n"
        ));
    }

    #[test]
    fn plan_skips_existing_config() {
        let mut ctx = healthy_context();
        ctx.config_exists = true;
        let plan = build_init_plan(&ctx, PRE_COMMIT, PRE_PUSH, None, None);
        assert!(!plan
            .actions
            .iter()
            .any(|a| matches!(a, InitAction::CreateConfig { .. })));
    }

    #[test]
    fn dry_run_report_does_not_fail_on_planned_actions() {
        let plan = build_init_plan(&healthy_context(), PRE_COMMIT, PRE_PUSH, None, None);
        let (report, code) = build_init_report(&plan, &healthy_context(), false, &[], false);
        assert_eq!(code, ExitCode::Success);
        assert!(report.issues.iter().any(|i| i.code == "init.dry_run"));
    }
}
