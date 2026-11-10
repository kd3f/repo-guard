//! Git remote probing.

use crate::domain::remote::RemoteInfo;
use crate::infrastructure::process::ProcessRunner;

pub fn list_remotes<R: ProcessRunner>(
    runner: &R,
    repo_root: &std::path::Path,
) -> Result<Vec<RemoteInfo>, String> {
    let cwd = repo_root.to_str().unwrap_or(".");
    let output = runner
        .run("git", &["remote", "-v"], Some(cwd))
        .map_err(|e| e.to_string())?;
    if !output.success() {
        return Err(output.stderr);
    }

    let mut remotes = Vec::new();
    for line in output.stdout.lines() {
        let mut parts = line.split_whitespace();
        let name = parts.next().unwrap_or_default().to_string();
        let url = parts.next().unwrap_or_default().to_string();
        if name.is_empty() || url.is_empty() {
            continue;
        }
        if remotes.iter().any(|r: &RemoteInfo| r.name == name) {
            continue;
        }
        remotes.push(RemoteInfo { name, url });
    }
    Ok(remotes)
}

pub fn check_origin_reachable<R: ProcessRunner>(
    runner: &R,
    repo_root: &std::path::Path,
) -> Option<bool> {
    let cwd = repo_root.to_str().unwrap_or(".");
    runner
        .run("git", &["ls-remote", "--heads", "origin"], Some(cwd))
        .ok()
        .map(|o| o.success())
}
