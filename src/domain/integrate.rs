//! Printed integration snippets for external hook managers.

use crate::domain::compatibility::{HookManagerKind, OperatingMode};

#[derive(Debug, Clone, Copy)]
pub enum IntegrationTarget {
    PreCommit,
    Husky,
    Lefthook,
    Ci,
}

pub fn integration_snippet(target: IntegrationTarget) -> &'static str {
    match target {
        IntegrationTarget::PreCommit => PRE_COMMIT_SNIPPET,
        IntegrationTarget::Husky => HUSKY_SNIPPET,
        IntegrationTarget::Lefthook => LEFTHOOK_SNIPPET,
        IntegrationTarget::Ci => CI_SNIPPET,
    }
}

pub fn integration_title(target: IntegrationTarget) -> &'static str {
    match target {
        IntegrationTarget::PreCommit => "pre-commit",
        IntegrationTarget::Husky => "Husky",
        IntegrationTarget::Lefthook => "Lefthook",
        IntegrationTarget::Ci => "CI",
    }
}

const PRE_COMMIT_SNIPPET: &str = r#"# Add to .pre-commit-config.yaml (run AFTER formatters/fixers that mutate the index):
repos:
  - repo: local
    hooks:
      - id: repo-guard-check
        name: repo-guard staged index check
        entry: repo-guard check
        language: system
        pass_filenames: false
        stages: [pre-commit]
"#;

const HUSKY_SNIPPET: &str = r#"# Add to .husky/pre-commit (run AFTER lint-staged or other index-mutating hooks):
repo-guard check
"#;

const LEFTHOOK_SNIPPET: &str = r#"# Add to lefthook.yml (run AFTER formatters/fixers):
pre-commit:
  commands:
    repo-guard:
      run: repo-guard check
      priority: 100
"#;

const CI_SNIPPET: &str = r#"# Example GitHub Actions job step:
- name: Repo Guard history check
  run: repo-guard history check --range "${{ github.event.pull_request.base.sha }}..${{ github.sha }}"
"#;

pub fn ordering_note(manager: HookManagerKind, mode: OperatingMode) -> String {
    if mode == OperatingMode::Integration {
        format!(
            "Run repo-guard check after any {} hook that modifies and re-stages files.",
            manager_label(manager)
        )
    } else {
        "Standalone mode can use repo-guard init --apply for .githooks hooks.".into()
    }
}

fn manager_label(manager: HookManagerKind) -> &'static str {
    match manager {
        HookManagerKind::PreCommit => "pre-commit",
        HookManagerKind::Husky => "Husky",
        HookManagerKind::Lefthook => "Lefthook",
        HookManagerKind::CustomHooksPath => "custom hook",
        HookManagerKind::NativeGitHooks => "native Git hook",
        HookManagerKind::RepoGuardStandalone => "repo-guard standalone",
        HookManagerKind::None => "hook manager",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snippets_are_non_empty() {
        assert!(integration_snippet(IntegrationTarget::PreCommit).contains("repo-guard check"));
        assert!(integration_snippet(IntegrationTarget::Ci).contains("history check"));
    }
}
