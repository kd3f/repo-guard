//! Staged Git index inspection via Git commands.

use std::collections::HashSet;

use crate::domain::lfs::is_lfs_pointer;
use crate::domain::policy::StagedBlob;
use crate::infrastructure::process::ProcessRunner;

/// Enumerate staged blobs with sizes from the Git index.
pub fn list_staged_blobs<R: ProcessRunner>(
    runner: &R,
    repo_root: &std::path::Path,
) -> Result<Vec<StagedBlob>, String> {
    let cwd = path_to_str(repo_root);
    let names_output = runner
        .run("git", &["diff", "--cached", "--name-only", "-z"], Some(cwd))
        .map_err(|e| e.to_string())?;
    if !names_output.success() {
        return Err(names_output.stderr);
    }

    let ignored = list_staged_ignored_paths(runner, repo_root)?;
    let mut blobs = Vec::new();
    for path in split_null_paths(&names_output.stdout) {
        if path.is_empty() {
            continue;
        }
        let oid = blob_oid_for_path(runner, repo_root, &path)?;
        let size = blob_size(runner, repo_root, &oid)?;
        let content = blob_content(runner, repo_root, &oid)?;
        blobs.push(StagedBlob {
            path: path.clone(),
            size_bytes: size,
            is_lfs_pointer: is_lfs_pointer(&content),
            is_ignored: ignored.contains(&path),
        });
    }

    Ok(blobs)
}

fn list_staged_ignored_paths<R: ProcessRunner>(
    runner: &R,
    repo_root: &std::path::Path,
) -> Result<HashSet<String>, String> {
    let cwd = path_to_str(repo_root);
    let output = runner
        .run(
            "git",
            &["ls-files", "-c", "-i", "--exclude-standard", "-z"],
            Some(cwd),
        )
        .map_err(|e| e.to_string())?;
    Ok(split_null_paths(&output.stdout).into_iter().collect())
}

fn blob_oid_for_path<R: ProcessRunner>(
    runner: &R,
    repo_root: &std::path::Path,
    path: &str,
) -> Result<String, String> {
    let cwd = path_to_str(repo_root);
    let output = runner
        .run("git", &["ls-files", "-s", "--", path], Some(cwd))
        .map_err(|e| e.to_string())?;
    let line = output
        .stdout
        .lines()
        .next()
        .ok_or("missing ls-files output")?;
    let parts: Vec<&str> = line.split_whitespace().collect();
    parts
        .get(1)
        .map(|s| (*s).to_string())
        .ok_or_else(|| format!("invalid ls-files line for {path}"))
}

fn blob_size<R: ProcessRunner>(
    runner: &R,
    repo_root: &std::path::Path,
    oid: &str,
) -> Result<u64, String> {
    let cwd = path_to_str(repo_root);
    let output = runner
        .run("git", &["cat-file", "-s", oid], Some(cwd))
        .map_err(|e| e.to_string())?;
    output
        .stdout
        .trim()
        .parse()
        .map_err(|_| format!("invalid blob size for {oid}"))
}

fn blob_content<R: ProcessRunner>(
    runner: &R,
    repo_root: &std::path::Path,
    oid: &str,
) -> Result<Vec<u8>, String> {
    let cwd = path_to_str(repo_root);
    let output = runner
        .run("git", &["cat-file", "blob", oid], Some(cwd))
        .map_err(|e| e.to_string())?;
    Ok(output.stdout.into_bytes())
}

fn split_null_paths(data: &str) -> Vec<String> {
    data.split('\0')
        .filter(|p| !p.is_empty())
        .map(str::to_string)
        .collect()
}

fn path_to_str(path: &std::path::Path) -> &str {
    path.to_str().unwrap_or(".")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::process::{FakeProcessRunner, ProcessOutput};

    #[test]
    fn parses_staged_paths_from_null_delimited_output() {
        let runner = FakeProcessRunner::default()
            .insert(
                "git",
                &["diff", "--cached", "--name-only", "-z"],
                Ok(ProcessOutput {
                    status_code: 0,
                    stdout: "file.txt\0".into(),
                    stderr: String::new(),
                }),
            )
            .insert(
                "git",
                &["ls-files", "-c", "-i", "--exclude-standard", "-z"],
                Ok(ProcessOutput {
                    status_code: 0,
                    stdout: String::new(),
                    stderr: String::new(),
                }),
            )
            .insert(
                "git",
                &["ls-files", "-s", "--", "file.txt"],
                Ok(ProcessOutput {
                    status_code: 0,
                    stdout: "100644 abc123 0\tfile.txt\n".into(),
                    stderr: String::new(),
                }),
            )
            .insert(
                "git",
                &["cat-file", "-s", "abc123"],
                Ok(ProcessOutput {
                    status_code: 0,
                    stdout: "12\n".into(),
                    stderr: String::new(),
                }),
            )
            .insert(
                "git",
                &["cat-file", "blob", "abc123"],
                Ok(ProcessOutput {
                    status_code: 0,
                    stdout: "hello world\n".into(),
                    stderr: String::new(),
                }),
            );

        let blobs = list_staged_blobs(&runner, std::path::Path::new(".")).unwrap();
        assert_eq!(blobs.len(), 1);
        assert_eq!(blobs[0].path, "file.txt");
        assert_eq!(blobs[0].size_bytes, 12);
    }
}
