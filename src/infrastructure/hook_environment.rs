//! Filesystem and Git probes for hook-manager detection.

use std::fs;
use std::path::Path;

use crate::domain::compatibility::HookEnvironmentSignals;
use crate::infrastructure::process::ProcessRunner;
use crate::infrastructure::repository::{git_repo_root, read_hooks_path};
use crate::support::constants::{HOOKS_DIR_NAME, PRE_COMMIT_HOOK, PRE_PUSH_HOOK};

const PRE_COMMIT_CONFIG: &str = ".pre-commit-config.yaml";
const HUSKY_DIR: &str = ".husky";
const LEFTHOOK_CONFIG: &str = "lefthook.yml";
const LEFTHOOK_LOCAL_CONFIG: &str = "lefthook-local.yml";

/// Collect hook-environment signals from a repository root.
pub fn collect_hook_environment_signals(
    repo_root: &Path,
    hooks_path: Option<String>,
) -> HookEnvironmentSignals {
    let githooks = repo_root.join(HOOKS_DIR_NAME);
    let native_hooks = repo_root.join(".git").join("hooks");

    HookEnvironmentSignals {
        pre_commit_config: repo_root.join(PRE_COMMIT_CONFIG).is_file(),
        husky_dir: repo_root.join(HUSKY_DIR).is_dir(),
        lefthook_config: repo_root.join(LEFTHOOK_CONFIG).is_file(),
        lefthook_local_config: repo_root.join(LEFTHOOK_LOCAL_CONFIG).is_file(),
        hooks_path,
        githooks_dir: githooks.is_dir(),
        githooks_pre_commit: githooks.join(PRE_COMMIT_HOOK).is_file(),
        githooks_pre_push: githooks.join(PRE_PUSH_HOOK).is_file(),
        native_pre_commit: native_hook_present(&native_hooks.join(PRE_COMMIT_HOOK)),
        native_pre_push: native_hook_present(&native_hooks.join(PRE_PUSH_HOOK)),
    }
}

/// Resolve hook signals from the current working directory.
pub fn collect_hook_environment<R: ProcessRunner>(
    runner: &R,
    cwd: &Path,
) -> Option<HookEnvironmentSignals> {
    git_repo_root(runner, cwd).map(|root| {
        let hooks_path = read_hooks_path(runner, &root);
        collect_hook_environment_signals(&root, hooks_path)
    })
}

fn native_hook_present(path: &Path) -> bool {
    if !path.is_file() {
        return false;
    }
    fs::read_to_string(path)
        .map(|content| {
            let trimmed = content.trim();
            !trimmed.is_empty() && !is_git_sample_hook(trimmed)
        })
        .unwrap_or(false)
}

fn is_git_sample_hook(content: &str) -> bool {
    content.contains("An example hook script") || content.contains("example hook")
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn collects_pre_commit_and_husky_signals() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        fs::write(root.join(PRE_COMMIT_CONFIG), "repos: []\n").unwrap();
        fs::create_dir_all(root.join(HUSKY_DIR)).unwrap();

        let signals = collect_hook_environment_signals(root, None);
        assert!(signals.pre_commit_config);
        assert!(signals.husky_dir);
    }

    #[test]
    fn ignores_empty_native_hook() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        let hooks = root.join(".git/hooks");
        fs::create_dir_all(&hooks).unwrap();
        fs::write(hooks.join(PRE_COMMIT_HOOK), "\n").unwrap();

        let signals = collect_hook_environment_signals(root, None);
        assert!(!signals.native_pre_commit);
    }
}
