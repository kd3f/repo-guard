//! Canonical Git hook script content.

pub const PRE_COMMIT_HOOK: &str = r#"#!/bin/sh
# Repo Guard pre-commit hook (non-interactive, read-only validation).
set -eu
if ! command -v repo-guard >/dev/null 2>&1; then
  echo "repo-guard: not found on PATH; install with \"cargo install --path . --locked\" and add Cargo's bin directory to PATH" >&2
  exit 3
fi
repo-guard hook pre-commit
"#;

pub const PRE_PUSH_HOOK: &str = r#"#!/bin/sh
# Repo Guard pre-push hook: validate outgoing objects from ref updates, then delegate to Git LFS.
set -eu
if ! command -v repo-guard >/dev/null 2>&1; then
  echo "repo-guard: not found on PATH; install with \"cargo install --path . --locked\" and add Cargo's bin directory to PATH" >&2
  exit 3
fi
REF_STDIN="$(mktemp -t repo-guard-pre-push.XXXXXX)"
trap 'rm -f "$REF_STDIN"' EXIT INT TERM
cat > "$REF_STDIN"
repo-guard hook pre-push < "$REF_STDIN" || exit $?
git lfs pre-push "$@" < "$REF_STDIN"
"#;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::exit_code::MISSING_DEPENDENCY;

    #[test]
    fn hook_templates_check_repo_guard_on_path_before_running() {
        assert!(PRE_COMMIT_HOOK.contains("command -v repo-guard"));
        assert!(PRE_PUSH_HOOK.contains("command -v repo-guard"));
        assert!(PRE_COMMIT_HOOK.contains(&format!("exit {MISSING_DEPENDENCY}")));
        assert!(PRE_PUSH_HOOK.contains(&format!("exit {MISSING_DEPENDENCY}")));
    }

    #[test]
    fn pre_push_template_preserves_git_lfs_stdin_tee() {
        assert!(PRE_PUSH_HOOK.contains("cat > \"$REF_STDIN\""));
        assert!(PRE_PUSH_HOOK.contains("git lfs pre-push \"$@\" < \"$REF_STDIN\""));
    }
}
