//! Working-tree scanning via Git and filesystem.

use std::path::Path;

use crate::domain::scan::ScannedFile;
use crate::infrastructure::process::ProcessRunner;

pub fn collect_worktree_files<R: ProcessRunner>(
    runner: &R,
    repo_root: &Path,
) -> Result<Vec<ScannedFile>, String> {
    let cwd = path_to_str(repo_root);
    let tracked = runner
        .run("git", &["ls-files", "-z"], Some(cwd))
        .map_err(|e| e.to_string())?;
    let untracked = runner
        .run(
            "git",
            &["ls-files", "--others", "--exclude-standard", "-z"],
            Some(cwd),
        )
        .map_err(|e| e.to_string())?;
    let ignored = runner
        .run(
            "git",
            &["ls-files", "--others", "-i", "--exclude-standard", "-z"],
            Some(cwd),
        )
        .map_err(|e| e.to_string())?;

    let ignored_set: std::collections::HashSet<String> =
        split_null(&ignored.stdout).into_iter().collect();

    let mut files = Vec::new();
    for path in split_null(&tracked.stdout)
        .into_iter()
        .chain(split_null(&untracked.stdout))
    {
        let full = repo_root.join(&path);
        let size = std::fs::metadata(&full).map(|m| m.len()).unwrap_or(0);
        files.push(ScannedFile {
            ignored: ignored_set.contains(&path),
            path,
            size_bytes: size,
        });
    }

    for path in ignored_set {
        let full = repo_root.join(&path);
        let size = std::fs::metadata(&full).map(|m| m.len()).unwrap_or(0);
        files.push(ScannedFile {
            path,
            size_bytes: size,
            ignored: true,
        });
    }

    Ok(files)
}

fn split_null(data: &str) -> Vec<String> {
    data.split('\0')
        .filter(|p| !p.is_empty())
        .map(str::to_string)
        .collect()
}

fn path_to_str(path: &Path) -> &str {
    path.to_str().unwrap_or(".")
}
