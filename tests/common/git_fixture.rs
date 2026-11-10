use std::path::{Path, PathBuf};

use super::test_env::git_run;
use tempfile::TempDir;

#[allow(dead_code)]
pub struct TestGitRepo {
    _dir: TempDir,
    root: PathBuf,
}

#[allow(dead_code)]
impl TestGitRepo {
    pub fn new() -> Self {
        Self::new_in(std::env::temp_dir().as_path())
    }

    /// Create a repository under `base` so temp paths stay on the same filesystem
    /// (required for pre-commit `try-repo` on Windows when the crate lives on another drive).
    pub fn new_in(base: &Path) -> Self {
        std::fs::create_dir_all(base).expect("create fixture base");
        let dir = tempfile::tempdir_in(base).expect("tempdir");
        let root = dir.path().to_path_buf();
        run_git(&root, &["init", "-b", "main"]);
        run_git(&root, &["config", "user.email", "test@example.com"]);
        run_git(&root, &["config", "user.name", "Repo Guard Test"]);
        Self { _dir: dir, root }
    }

    /// Consumer repos for pre-commit tests: same drive as the crate checkout.
    pub fn new_for_pre_commit_consumer() -> Self {
        let base = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("_test_repos");
        Self::new_in(&base)
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn run_git(&self, args: &[&str]) -> String {
        run_git(&self.root, args)
    }

    pub fn write_file(&self, relative: &str, content: &[u8]) -> PathBuf {
        let path = self.root.join(relative);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("create parent");
        }
        std::fs::write(&path, content).expect("write file");
        path
    }

    pub fn git_add(&self, path: &str) {
        run_git(&self.root, &["add", "--", path]);
    }

    pub fn git_commit(&self, message: &str) {
        run_git(&self.root, &["commit", "-m", message]);
    }

    pub fn git_commit_no_verify(&self, message: &str) {
        run_git(&self.root, &["commit", "--no-verify", "-m", message]);
    }

    /// Run `git` with `PATH` configured for Repo Guard hook scripts.
    pub fn git_output(&self, args: &[&str]) -> std::process::Output {
        super::test_env::git_output(self.root(), args)
    }
}

fn run_git(cwd: &Path, args: &[&str]) -> String {
    git_run(cwd, args)
}
