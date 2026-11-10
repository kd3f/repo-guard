//! Hook-manager detection and operating-mode selection (pure logic).

use serde::Serialize;

use crate::domain::init::{InitAction, InitConflict, InitPlan};
use crate::support::constants::HOOKS_PATH_VALUE;

/// Known hook orchestration ecosystems Repo Guard must not replace.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum HookManagerKind {
    None,
    RepoGuardStandalone,
    PreCommit,
    Husky,
    Lefthook,
    CustomHooksPath,
    NativeGitHooks,
}

impl HookManagerKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::RepoGuardStandalone => "repo_guard_standalone",
            Self::PreCommit => "pre_commit",
            Self::Husky => "husky",
            Self::Lefthook => "lefthook",
            Self::CustomHooksPath => "custom_hooks_path",
            Self::NativeGitHooks => "native_git_hooks",
        }
    }
}

/// Recommended Repo Guard operating mode for the repository.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OperatingMode {
    Standalone,
    Integration,
}

impl OperatingMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Standalone => "standalone",
            Self::Integration => "integration",
        }
    }
}

/// A single piece of filesystem or Git configuration evidence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DetectionEvidence {
    pub manager: HookManagerKind,
    pub source: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

/// Read-only inputs gathered before classification.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HookEnvironmentSignals {
    pub pre_commit_config: bool,
    pub husky_dir: bool,
    pub lefthook_config: bool,
    pub lefthook_local_config: bool,
    pub hooks_path: Option<String>,
    pub githooks_dir: bool,
    pub githooks_pre_commit: bool,
    pub githooks_pre_push: bool,
    pub native_pre_commit: bool,
    pub native_pre_push: bool,
}

/// Classified hook environment with recommended next steps.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct HookEnvironment {
    pub evidence: Vec<DetectionEvidence>,
    pub primary_manager: HookManagerKind,
    pub recommended_mode: OperatingMode,
    pub hooks_path: Option<String>,
    pub allows_standalone_hook_setup: bool,
    pub safe_actions: Vec<String>,
}

/// Analyze repository signals and produce a hook environment report.
pub fn analyze_hook_environment(signals: &HookEnvironmentSignals) -> HookEnvironment {
    let mut evidence = Vec::new();

    if signals.pre_commit_config {
        evidence.push(DetectionEvidence {
            manager: HookManagerKind::PreCommit,
            source: ".pre-commit-config.yaml".into(),
            detail: None,
        });
    }
    if signals.husky_dir {
        evidence.push(DetectionEvidence {
            manager: HookManagerKind::Husky,
            source: ".husky/".into(),
            detail: None,
        });
    }
    if signals.lefthook_config {
        evidence.push(DetectionEvidence {
            manager: HookManagerKind::Lefthook,
            source: "lefthook.yml".into(),
            detail: None,
        });
    }
    if signals.lefthook_local_config {
        evidence.push(DetectionEvidence {
            manager: HookManagerKind::Lefthook,
            source: "lefthook-local.yml".into(),
            detail: None,
        });
    }
    if let Some(path) = &signals.hooks_path {
        if path == HOOKS_PATH_VALUE {
            evidence.push(DetectionEvidence {
                manager: HookManagerKind::RepoGuardStandalone,
                source: "core.hooksPath".into(),
                detail: Some(path.clone()),
            });
        } else {
            evidence.push(DetectionEvidence {
                manager: HookManagerKind::CustomHooksPath,
                source: "core.hooksPath".into(),
                detail: Some(path.clone()),
            });
        }
    }
    if (signals.githooks_dir || signals.githooks_pre_commit || signals.githooks_pre_push)
        && !evidence
            .iter()
            .any(|e| e.manager == HookManagerKind::RepoGuardStandalone)
    {
        evidence.push(DetectionEvidence {
            manager: HookManagerKind::RepoGuardStandalone,
            source: ".githooks/".into(),
            detail: None,
        });
    }
    if signals.native_pre_commit || signals.native_pre_push {
        let mut sources = Vec::new();
        if signals.native_pre_commit {
            sources.push(".git/hooks/pre-commit");
        }
        if signals.native_pre_push {
            sources.push(".git/hooks/pre-push");
        }
        evidence.push(DetectionEvidence {
            manager: HookManagerKind::NativeGitHooks,
            source: sources.join(", "),
            detail: None,
        });
    }

    let primary_manager = select_primary_manager(&evidence, signals);
    let allows_standalone_hook_setup = standalone_hook_setup_allowed(primary_manager, signals);
    let recommended_mode = if allows_standalone_hook_setup {
        OperatingMode::Standalone
    } else {
        OperatingMode::Integration
    };
    let safe_actions = recommended_safe_actions(primary_manager, recommended_mode);

    HookEnvironment {
        evidence,
        primary_manager,
        recommended_mode,
        hooks_path: signals.hooks_path.clone(),
        allows_standalone_hook_setup,
        safe_actions,
    }
}

fn select_primary_manager(
    evidence: &[DetectionEvidence],
    signals: &HookEnvironmentSignals,
) -> HookManagerKind {
    if signals.pre_commit_config {
        return HookManagerKind::PreCommit;
    }
    if signals.husky_dir {
        return HookManagerKind::Husky;
    }
    if signals.lefthook_config || signals.lefthook_local_config {
        return HookManagerKind::Lefthook;
    }
    if let Some(path) = &signals.hooks_path {
        if path != HOOKS_PATH_VALUE {
            return HookManagerKind::CustomHooksPath;
        }
        return HookManagerKind::RepoGuardStandalone;
    }
    if signals.native_pre_commit || signals.native_pre_push {
        return HookManagerKind::NativeGitHooks;
    }
    if evidence
        .iter()
        .any(|e| e.manager == HookManagerKind::RepoGuardStandalone)
    {
        return HookManagerKind::RepoGuardStandalone;
    }
    HookManagerKind::None
}

fn standalone_hook_setup_allowed(
    primary: HookManagerKind,
    signals: &HookEnvironmentSignals,
) -> bool {
    match primary {
        HookManagerKind::PreCommit
        | HookManagerKind::Husky
        | HookManagerKind::Lefthook
        | HookManagerKind::CustomHooksPath
        | HookManagerKind::NativeGitHooks => false,
        HookManagerKind::RepoGuardStandalone => true,
        HookManagerKind::None => {
            !signals.pre_commit_config
                && !signals.husky_dir
                && !signals.lefthook_config
                && !signals.lefthook_local_config
                && signals
                    .hooks_path
                    .as_deref()
                    .is_none_or(|p| p == HOOKS_PATH_VALUE)
                && !signals.native_pre_commit
                && !signals.native_pre_push
        }
    }
}

fn recommended_safe_actions(primary: HookManagerKind, mode: OperatingMode) -> Vec<String> {
    match mode {
        OperatingMode::Standalone => vec![
            "Run repo-guard init --apply or install --apply to configure .githooks when ready"
                .into(),
            "Run repo-guard check before commits for staged-index validation".into(),
            "Run repo-guard detect after hook changes to confirm operating mode".into(),
        ],
        OperatingMode::Integration => {
            let manager = match primary {
                HookManagerKind::PreCommit => "pre-commit",
                HookManagerKind::Husky => "Husky",
                HookManagerKind::Lefthook => "Lefthook",
                HookManagerKind::CustomHooksPath => "the configured hooks path",
                HookManagerKind::NativeGitHooks => "native .git/hooks scripts",
                _ => "the existing hook setup",
            };
            vec![
                format!(
                    "Keep {manager} as the hook owner; Repo Guard will not take over core.hooksPath"
                ),
                "Run repo-guard check for staged-index validation inside your hook pipeline".into(),
                "Run repo-guard detect to review evidence and recommended integration steps".into(),
                "Do not run init --apply or install --apply for standalone hook setup in this repository".into(),
            ]
        }
    }
}

/// Remove standalone hook actions and record a blocking conflict when another manager owns hooks.
pub fn enforce_hook_ownership(plan: &mut InitPlan, env: &HookEnvironment) {
    if env.allows_standalone_hook_setup {
        return;
    }

    plan.actions.retain(|action| {
        !matches!(
            action,
            InitAction::CreateHook { .. } | InitAction::SetHooksPath { .. }
        )
    });

    let message = manager_conflict_message(env);
    if !plan
        .conflicts
        .iter()
        .any(|c| c.code == "hooks.manager_owned")
    {
        plan.conflicts.push(InitConflict {
            code: "hooks.manager_owned",
            message,
        });
    }
}

fn manager_conflict_message(env: &HookEnvironment) -> String {
    let owner = env.primary_manager.as_str();
    let sources: Vec<&str> = env.evidence.iter().map(|e| e.source.as_str()).collect();
    format!(
        "Detected {owner} hook ownership ({}) ; refusing standalone .githooks setup. Use integration mode instead.",
        sources.join(", ")
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fresh_repo_recommends_standalone() {
        let env = analyze_hook_environment(&HookEnvironmentSignals::default());
        assert_eq!(env.primary_manager, HookManagerKind::None);
        assert_eq!(env.recommended_mode, OperatingMode::Standalone);
        assert!(env.allows_standalone_hook_setup);
    }

    #[test]
    fn pre_commit_blocks_standalone() {
        let signals = HookEnvironmentSignals {
            pre_commit_config: true,
            ..Default::default()
        };
        let env = analyze_hook_environment(&signals);
        assert_eq!(env.primary_manager, HookManagerKind::PreCommit);
        assert_eq!(env.recommended_mode, OperatingMode::Integration);
        assert!(!env.allows_standalone_hook_setup);
    }

    #[test]
    fn husky_blocks_standalone() {
        let signals = HookEnvironmentSignals {
            husky_dir: true,
            ..Default::default()
        };
        let env = analyze_hook_environment(&signals);
        assert_eq!(env.primary_manager, HookManagerKind::Husky);
        assert!(!env.allows_standalone_hook_setup);
    }

    #[test]
    fn lefthook_blocks_standalone() {
        let signals = HookEnvironmentSignals {
            lefthook_config: true,
            ..Default::default()
        };
        let env = analyze_hook_environment(&signals);
        assert_eq!(env.primary_manager, HookManagerKind::Lefthook);
        assert!(!env.allows_standalone_hook_setup);
    }

    #[test]
    fn custom_hooks_path_blocks_standalone() {
        let signals = HookEnvironmentSignals {
            hooks_path: Some(".husky".into()),
            ..Default::default()
        };
        let env = analyze_hook_environment(&signals);
        assert_eq!(env.primary_manager, HookManagerKind::CustomHooksPath);
        assert!(!env.allows_standalone_hook_setup);
    }

    #[test]
    fn repo_guard_hooks_path_allows_standalone() {
        let signals = HookEnvironmentSignals {
            hooks_path: Some(".githooks".into()),
            githooks_dir: true,
            ..Default::default()
        };
        let env = analyze_hook_environment(&signals);
        assert_eq!(env.primary_manager, HookManagerKind::RepoGuardStandalone);
        assert!(env.allows_standalone_hook_setup);
    }

    #[test]
    fn native_git_hooks_block_standalone() {
        let signals = HookEnvironmentSignals {
            native_pre_commit: true,
            ..Default::default()
        };
        let env = analyze_hook_environment(&signals);
        assert_eq!(env.primary_manager, HookManagerKind::NativeGitHooks);
        assert!(!env.allows_standalone_hook_setup);
    }

    #[test]
    fn enforce_removes_hook_actions_when_manager_detected() {
        let signals = HookEnvironmentSignals {
            pre_commit_config: true,
            ..Default::default()
        };
        let env = analyze_hook_environment(&signals);
        let mut plan = InitPlan {
            actions: vec![
                InitAction::CreateConfig {
                    relative_path: ".repo-guard.toml".into(),
                },
                InitAction::CreateHook {
                    name: "pre-commit".into(),
                },
                InitAction::SetHooksPath {
                    value: ".githooks".into(),
                },
            ],
            conflicts: Vec::new(),
            skipped: Vec::new(),
        };
        enforce_hook_ownership(&mut plan, &env);
        assert!(plan
            .actions
            .iter()
            .all(|a| matches!(a, InitAction::CreateConfig { .. })));
        assert!(plan
            .conflicts
            .iter()
            .any(|c| c.code == "hooks.manager_owned"));
    }
}
