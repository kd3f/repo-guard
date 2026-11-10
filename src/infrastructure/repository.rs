//! Git repository operations beyond basic probes.

use std::fs;
use std::path::{Path, PathBuf};

use crate::domain::init::hook_content_equivalent;
use crate::domain::init::InitContext;
use crate::infrastructure::git::{probe_git, probe_git_lfs, probe_git_root, probe_hooks_path};
use crate::infrastructure::hooks::{PRE_COMMIT_HOOK, PRE_PUSH_HOOK};
use crate::infrastructure::process::ProcessRunner;
use crate::support::constants::{
    CONFIG_FILE_NAME, HOOKS_DIR_NAME, HOOKS_PATH_VALUE, PRE_COMMIT_HOOK as PRE_COMMIT_NAME,
    PRE_PUSH_HOOK as PRE_PUSH_NAME,
};

/// Resolve the Git repository root, if any.
pub fn git_repo_root<R: ProcessRunner>(runner: &R, cwd: &Path) -> Option<PathBuf> {
    probe_git_root(runner, cwd).ok().flatten()
}

/// Read local `core.hooksPath` value.
pub fn read_hooks_path<R: ProcessRunner>(runner: &R, repo_root: &Path) -> Option<String> {
    probe_hooks_path(runner, repo_root)
}

/// Gather repository state needed for init planning.
pub fn collect_init_context<R: ProcessRunner>(runner: &R, cwd: &Path) -> InitContext {
    let mut context = InitContext::default();

    match probe_git(runner, cwd) {
        Ok((available, _)) => context.git_available = available,
        Err(_) => context.git_available = false,
    }

    if !context.git_available {
        return context;
    }

    match probe_git_lfs(runner, cwd) {
        Ok((available, _)) => context.git_lfs_available = available,
        Err(_) => context.git_lfs_available = false,
    }

    if let Some(root) = git_repo_root(runner, cwd) {
        context.inside_git_repo = true;
        context.config_exists = root.join(CONFIG_FILE_NAME).is_file();
        let hooks_dir = root.join(HOOKS_DIR_NAME);
        context.hooks_dir_exists = hooks_dir.is_dir();
        let pre_commit = hooks_dir.join(PRE_COMMIT_NAME);
        let pre_push = hooks_dir.join(PRE_PUSH_NAME);
        context.pre_commit_exists = pre_commit.is_file();
        context.pre_push_exists = pre_push.is_file();
        context.hooks_path = read_hooks_path(runner, &root);
        if context.pre_commit_exists {
            context.pre_commit_differs = fs::read_to_string(&pre_commit)
                .map(|c| c != PRE_COMMIT_HOOK)
                .unwrap_or(true);
        }
        if context.pre_push_exists {
            context.pre_push_differs = fs::read_to_string(&pre_push)
                .map(|c| c != PRE_PUSH_HOOK)
                .unwrap_or(true);
        }
    }

    context
}

/// Set local `core.hooksPath`.
pub fn set_hooks_path<R: ProcessRunner>(
    runner: &R,
    repo_root: &Path,
    value: &str,
) -> Result<(), String> {
    let cwd = path_to_str(repo_root);
    let output = runner
        .run(
            "git",
            &["config", "--local", "core.hooksPath", value],
            Some(cwd),
        )
        .map_err(|e| e.to_string())?;
    if output.success() {
        Ok(())
    } else {
        Err(output.stderr.trim().to_string())
    }
}

/// Run `git lfs install --local`.
pub fn git_lfs_install_local<R: ProcessRunner>(runner: &R, repo_root: &Path) -> Result<(), String> {
    let cwd = path_to_str(repo_root);
    let output = runner
        .run("git", &["lfs", "install", "--local"], Some(cwd))
        .map_err(|e| e.to_string())?;
    if output.success() {
        Ok(())
    } else {
        Err(output.stderr.trim().to_string())
    }
}

/// Create `.githooks` directory and hook file if missing.
pub fn ensure_hook_file(repo_root: &Path, name: &str, content: &str) -> Result<bool, String> {
    if !is_repo_guard_hook_name(name) {
        return Err(format!(
            "refusing to write non-Repo-Guard hook file: {name}"
        ));
    }

    let hooks_dir = repo_root.join(HOOKS_DIR_NAME);
    fs::create_dir_all(&hooks_dir).map_err(|e| e.to_string())?;
    let path = hooks_dir.join(name);
    let created = if path.is_file() {
        false
    } else {
        fs::write(&path, content).map_err(|e| e.to_string())?;
        true
    };

    ensure_repo_guard_hook_executable(&path, content)?;
    Ok(created)
}

/// Ensure Repo Guard-owned hook scripts are executable on Unix-like systems.
pub fn ensure_repo_guard_hooks_executable(
    repo_root: &Path,
    pre_commit_template: &str,
    pre_push_template: &str,
) -> Result<(), String> {
    for (name, template) in [
        (PRE_COMMIT_NAME, pre_commit_template),
        (PRE_PUSH_NAME, pre_push_template),
    ] {
        let path = repo_root.join(HOOKS_DIR_NAME).join(name);
        if path.is_file() {
            ensure_repo_guard_hook_executable(&path, template)?;
        }
    }
    Ok(())
}

fn is_repo_guard_hook_name(name: &str) -> bool {
    name == PRE_COMMIT_NAME || name == PRE_PUSH_NAME
}

fn ensure_repo_guard_hook_executable(path: &Path, template: &str) -> Result<(), String> {
    let existing = fs::read_to_string(path).map_err(|e| e.to_string())?;
    if hook_content_equivalent(&existing, template) {
        ensure_unix_hook_executable(path)?;
    }
    Ok(())
}

#[cfg(unix)]
fn ensure_unix_hook_executable(path: &Path) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;

    let metadata = fs::metadata(path).map_err(|e| e.to_string())?;
    let mut permissions = metadata.permissions();
    let mode = permissions.mode();
    if mode & 0o111 == 0 {
        permissions.set_mode(0o755);
        fs::set_permissions(path, permissions).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[cfg(not(unix))]
fn ensure_unix_hook_executable(_path: &Path) -> Result<(), String> {
    Ok(())
}

/// Read hook file content when present.
pub fn read_hook_file(repo_root: &Path, name: &str) -> Option<String> {
    let path = repo_root.join(HOOKS_DIR_NAME).join(name);
    fs::read_to_string(path).ok()
}

pub fn is_repo_guard_hooks_path(path: Option<&str>) -> bool {
    path == Some(HOOKS_PATH_VALUE)
}

fn path_to_str(path: &Path) -> &str {
    path.to_str().unwrap_or(".")
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn ensure_hook_file_rejects_unknown_hook_names() {
        let dir = tempdir().unwrap();
        let err = ensure_hook_file(dir.path(), "post-commit", "#!/bin/sh\n").unwrap_err();
        assert!(err.contains("non-Repo-Guard"));
    }

    #[cfg(unix)]
    #[test]
    fn ensure_hook_file_sets_unix_executable_permissions() {
        use std::os::unix::fs::PermissionsExt;

        use crate::infrastructure::hooks::{PRE_COMMIT_HOOK, PRE_PUSH_HOOK};

        let dir = tempdir().unwrap();
        assert!(ensure_hook_file(dir.path(), PRE_COMMIT_NAME, PRE_COMMIT_HOOK).unwrap());
        assert!(ensure_hook_file(dir.path(), PRE_PUSH_NAME, PRE_PUSH_HOOK).unwrap());

        for name in [PRE_COMMIT_NAME, PRE_PUSH_NAME] {
            let path = dir.path().join(HOOKS_DIR_NAME).join(name);
            let mode = fs::metadata(&path).unwrap().permissions().mode();
            assert_ne!(mode & 0o111, 0, "{name} must be executable on Unix");
        }

        let path = dir.path().join(HOOKS_DIR_NAME).join(PRE_COMMIT_NAME);
        fs::write(&path, PRE_COMMIT_HOOK).unwrap();
        let mut permissions = fs::metadata(&path).unwrap().permissions();
        permissions.set_mode(0o644);
        fs::set_permissions(&path, permissions).unwrap();

        ensure_repo_guard_hooks_executable(dir.path(), PRE_COMMIT_HOOK, PRE_PUSH_HOOK).unwrap();
        let mode = fs::metadata(&path).unwrap().permissions().mode();
        assert_ne!(
            mode & 0o111,
            0,
            "existing Repo Guard hook must be re-chmodded"
        );
    }

    #[cfg(unix)]
    #[test]
    fn ensure_hook_file_does_not_chmod_foreign_hook_content() {
        use std::os::unix::fs::PermissionsExt;

        use crate::infrastructure::hooks::{PRE_COMMIT_HOOK, PRE_PUSH_HOOK};

        let dir = tempdir().unwrap();
        fs::create_dir_all(dir.path().join(HOOKS_DIR_NAME)).unwrap();
        let path = dir.path().join(HOOKS_DIR_NAME).join(PRE_COMMIT_NAME);
        fs::write(&path, "#!/bin/sh\nforeign hook\n").unwrap();
        let mut permissions = fs::metadata(&path).unwrap().permissions();
        permissions.set_mode(0o644);
        fs::set_permissions(&path, permissions).unwrap();

        ensure_repo_guard_hooks_executable(dir.path(), PRE_COMMIT_HOOK, PRE_PUSH_HOOK).unwrap();
        let mode = fs::metadata(&path).unwrap().permissions().mode();
        assert_eq!(mode & 0o111, 0, "foreign hook content must not be chmodded");
    }
}
