//! Git object traversal for history and outgoing-push validation.

use std::collections::HashSet;

use crate::domain::history::OutgoingBlob;
use crate::domain::lfs::is_lfs_pointer;
use crate::infrastructure::process::ProcessRunner;

/// A Git object line from `git rev-list --objects`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RevListObject {
    pub oid: String,
    pub path: Option<String>,
}

pub fn object_exists<R: ProcessRunner>(runner: &R, repo_root: &std::path::Path, oid: &str) -> bool {
    let cwd = path_to_str(repo_root);
    runner
        .run("git", &["cat-file", "-e", oid], Some(cwd))
        .map(|output| output.success())
        .unwrap_or(false)
}

pub fn rev_list_objects<R: ProcessRunner>(
    runner: &R,
    repo_root: &std::path::Path,
    spec: &str,
) -> Result<Vec<RevListObject>, String> {
    let cwd = path_to_str(repo_root);
    let output = runner
        .run("git", &["rev-list", "--objects", spec], Some(cwd))
        .map_err(|e| e.to_string())?;
    if !output.success() {
        return Err(output.stderr.trim().to_string());
    }
    Ok(parse_rev_list_objects(&output.stdout))
}

pub fn collect_unique_blobs<R: ProcessRunner>(
    runner: &R,
    repo_root: &std::path::Path,
    specs: &[String],
) -> Result<Vec<OutgoingBlob>, String> {
    let mut seen = HashSet::new();
    let mut blobs = Vec::new();

    for spec in specs {
        for object in rev_list_objects(runner, repo_root, spec)? {
            if object.path.is_none() {
                continue;
            }
            if !seen.insert(object.oid.clone()) {
                continue;
            }
            let size = blob_size(runner, repo_root, &object.oid)?;
            let content = blob_content(runner, repo_root, &object.oid)?;
            blobs.push(OutgoingBlob {
                oid: object.oid,
                path: object.path,
                size_bytes: size,
                is_lfs_pointer: is_lfs_pointer(&content),
            });
        }
    }

    Ok(blobs)
}

fn parse_rev_list_objects(stdout: &str) -> Vec<RevListObject> {
    stdout
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.is_empty() {
                return None;
            }
            if let Some((oid, path)) = line.split_once(' ') {
                Some(RevListObject {
                    oid: oid.into(),
                    path: Some(path.into()),
                })
            } else {
                Some(RevListObject {
                    oid: line.into(),
                    path: None,
                })
            }
        })
        .collect()
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

fn path_to_str(path: &std::path::Path) -> &str {
    path.to_str().unwrap_or(".")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_rev_list_object_lines() {
        let objects = parse_rev_list_objects("abc123\n def456 path/file.bin\n");
        assert_eq!(objects.len(), 2);
        assert_eq!(objects[1].path.as_deref(), Some("path/file.bin"));
    }
}
