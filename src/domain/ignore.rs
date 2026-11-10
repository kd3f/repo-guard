//! Ignore-path validation logic and near-match detection.

use std::path::Path;

use crate::domain::report::Issue;
use walkdir::WalkDir;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct IgnoreEvaluation {
    pub issues: Vec<Issue>,
    pub warnings: usize,
    pub errors: usize,
    pub near_matches: Vec<NearMatch>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct NearMatch {
    pub configured: String,
    pub candidate: String,
}

pub fn evaluate_ignore_paths(
    _configured_paths: &[String],
    existence: &[(String, bool)],
    ignored_by_git: &[(String, bool)],
    staged_despite_ignore: &[String],
    near_matches: &[NearMatch],
) -> IgnoreEvaluation {
    let mut eval = IgnoreEvaluation {
        near_matches: near_matches.to_vec(),
        ..IgnoreEvaluation::default()
    };

    for (path, exists) in existence {
        if !*exists {
            eval.warnings += 1;
            eval.issues.push(Issue::warning(
                "ignore.missing_path",
                format!("Configured ignore path does not exist: {path}"),
            ));
        }
    }

    for (path, ignored) in ignored_by_git {
        let exists = existence
            .iter()
            .find(|(p, _)| p == path)
            .map(|(_, e)| *e)
            .unwrap_or(false);
        if !exists {
            continue;
        }
        if *ignored {
            eval.issues.push(Issue::info(
                "ignore.confirmed",
                format!("Git ignores configured path: {path}"),
            ));
        } else {
            eval.warnings += 1;
            eval.issues.push(Issue::warning(
                "ignore.not_ignored",
                format!("Configured path exists but is not ignored by Git: {path}"),
            ));
        }
    }

    for near in near_matches {
        eval.warnings += 1;
        eval.issues.push(Issue::warning(
            "ignore.near_match",
            format!(
                "Configured path \"{}\" was not found; similar path \"{}\" exists",
                near.configured, near.candidate
            ),
        ));
    }

    for path in staged_despite_ignore {
        eval.errors += 1;
        eval.issues.push(Issue::error(
            "ignore.staged",
            format!("{path} is staged despite matching an ignore rule"),
        ));
    }

    eval
}

/// Normalize configured ignore paths for comparison.
pub fn normalize_ignore_path(path: &str) -> String {
    path.replace('\\', "/").trim().trim_matches('/').to_string()
}

/// Collect directory paths relative to the repository root.
pub fn collect_repository_directories(repo_root: &Path) -> Vec<String> {
    let mut dirs = Vec::new();
    for entry in WalkDir::new(repo_root)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_dir())
    {
        let path = entry.path();
        if path == repo_root {
            continue;
        }
        if let Ok(rel) = path.strip_prefix(repo_root) {
            let normalized = rel
                .components()
                .map(|c| c.as_os_str().to_string_lossy().into_owned())
                .collect::<Vec<_>>()
                .join("/");
            if !normalized.is_empty() && !normalized.starts_with(".git/") {
                dirs.push(normalized);
            }
        }
    }
    dirs
}

/// Find likely path mismatches when a configured directory does not exist.
pub fn find_near_match_candidates(
    configured: &str,
    existing_directories: &[String],
) -> Vec<String> {
    let configured = normalize_ignore_path(configured);
    if configured.is_empty() {
        return Vec::new();
    }

    let leaf = configured
        .rsplit('/')
        .next()
        .unwrap_or(configured.as_str())
        .to_string();

    let mut candidates: Vec<String> = existing_directories
        .iter()
        .filter(|dir| {
            let normalized = normalize_ignore_path(dir);
            normalized != configured
                && (normalized.ends_with(&leaf)
                    || normalized.ends_with(&format!("{leaf}/"))
                    || normalized.split('/').any(|part| part == leaf))
        })
        .cloned()
        .collect();

    candidates.sort();
    candidates.dedup();

    // Prefer candidates that share a prefix with the configured path.
    candidates.sort_by_key(|candidate| {
        let shared = shared_prefix_components(&configured, candidate);
        std::cmp::Reverse(shared)
    });

    candidates.truncate(3);
    candidates
}

fn shared_prefix_components(a: &str, b: &str) -> usize {
    a.split('/')
        .zip(b.split('/'))
        .take_while(|(x, y)| x == y)
        .count()
}

pub fn build_near_matches(configured_paths: &[String], repo_root: &Path) -> Vec<NearMatch> {
    let existing = collect_repository_directories(repo_root);
    let mut matches = Vec::new();

    for configured in configured_paths {
        let normalized = normalize_ignore_path(configured);
        if normalized.is_empty() {
            continue;
        }
        if repo_root.join(&normalized).exists() {
            continue;
        }
        for candidate in find_near_match_candidates(configured, &existing) {
            matches.push(NearMatch {
                configured: configured.clone(),
                candidate,
            });
        }
    }

    matches
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_duplicate_directory_segment_candidate() {
        let existing = vec![
            "linceo".into(),
            "linceo/SEAF".into(),
            "linceo/SEAF/SEAF".into(),
            "linceo/SEAF/SEAF/Binari".into(),
        ];
        let candidates = find_near_match_candidates("linceo/SEAF/Binari/", &existing);
        assert!(candidates.iter().any(|c| c == "linceo/SEAF/SEAF/Binari"));
    }

    #[test]
    fn missing_path_does_not_emit_not_ignored() {
        let existence = vec![("missing/build/".into(), false)];
        let ignored = vec![("missing/build/".into(), false)];
        let eval =
            evaluate_ignore_paths(&["missing/build/".into()], &existence, &ignored, &[], &[]);
        assert!(eval.issues.iter().any(|i| i.code == "ignore.missing_path"));
        assert!(!eval.issues.iter().any(|i| i.code == "ignore.not_ignored"));
    }
}
