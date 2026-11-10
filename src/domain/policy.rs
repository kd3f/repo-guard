//! Policy thresholds and violation evaluation.

use crate::domain::config::RepoGuardConfig;
use crate::domain::lfs::repair_commands;
use crate::domain::report::Issue;

const BYTES_PER_MIB: u64 = 1024 * 1024;

/// A staged blob inspected from the Git index.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StagedBlob {
    pub path: String,
    pub size_bytes: u64,
    pub is_lfs_pointer: bool,
    pub is_ignored: bool,
}

/// Policy evaluation outcome for staged content.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PolicyEvaluation {
    pub issues: Vec<Issue>,
    pub repair_commands: Vec<String>,
    pub violation_count: usize,
}

impl PolicyEvaluation {
    pub fn has_violations(&self) -> bool {
        self.violation_count > 0
    }
}

/// Evaluate staged blobs against repository policy.
pub fn evaluate_staged_blobs(config: &RepoGuardConfig, blobs: &[StagedBlob]) -> PolicyEvaluation {
    let lfs_threshold = config.lfs_threshold_mib * BYTES_PER_MIB;
    let safety_ceiling = config.github_safety_ceiling_mib * BYTES_PER_MIB;
    let mut evaluation = PolicyEvaluation::default();

    for blob in blobs {
        if blob.is_lfs_pointer {
            if config.block_small_lfs_files && blob.size_bytes < lfs_threshold {
                evaluation.issues.push(Issue::warning(
                    "lfs.small_pointer",
                    format!(
                        "{} is an LFS pointer below the {} MiB threshold",
                        blob.path, config.lfs_threshold_mib
                    ),
                ));
            }
            continue;
        }

        if blob.size_bytes >= safety_ceiling {
            evaluation.violation_count += 1;
            evaluation.issues.push(
                Issue::error(
                    "blob.github_ceiling",
                    format!(
                        "{} is a {} byte regular Git blob at or above the {} MiB GitHub safety ceiling",
                        blob.path,
                        blob.size_bytes,
                        config.github_safety_ceiling_mib
                    ),
                )
                .with_detail(format!(
                    "Regular Git blobs at or above {} MiB are high-risk and must be stored through Git LFS before committing.",
                    config.github_safety_ceiling_mib
                )),
            );
            evaluation
                .repair_commands
                .extend(repair_commands(&blob.path));
        } else if blob.size_bytes >= lfs_threshold {
            evaluation.violation_count += 1;
            evaluation.issues.push(
                Issue::error(
                    "blob.lfs_required",
                    format!(
                        "{} is a {} byte regular Git blob at or above the {} MiB LFS threshold",
                        blob.path, blob.size_bytes, config.lfs_threshold_mib
                    ),
                )
                .with_detail(format!(
                    "Files at or above {} MiB must be stored through Git LFS.",
                    config.lfs_threshold_mib
                )),
            );
            evaluation
                .repair_commands
                .extend(repair_commands(&blob.path));
        }

        if blob.is_ignored {
            evaluation.violation_count += 1;
            evaluation.issues.push(Issue::error(
                "ignore.staged",
                format!("{} is staged but matches an ignore rule", blob.path),
            ));
            evaluation
                .repair_commands
                .push(format!("git restore --staged -- \"{}\"", blob.path));
            evaluation.repair_commands.push("repo-guard check".into());
        }
    }

    evaluation
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config() -> RepoGuardConfig {
        RepoGuardConfig::default()
    }

    #[test]
    fn blocks_large_regular_blob() {
        let blobs = vec![StagedBlob {
            path: "big.zip".into(),
            size_bytes: 60 * BYTES_PER_MIB,
            is_lfs_pointer: false,
            is_ignored: false,
        }];
        let eval = evaluate_staged_blobs(&config(), &blobs);
        assert!(eval.has_violations());
        assert!(eval.repair_commands.iter().any(|c| c.contains("big.zip")));
    }

    #[test]
    fn accepts_lfs_pointer() {
        let blobs = vec![StagedBlob {
            path: "big.zip".into(),
            size_bytes: 130,
            is_lfs_pointer: true,
            is_ignored: false,
        }];
        let eval = evaluate_staged_blobs(&config(), &blobs);
        assert!(!eval.has_violations());
    }

    #[test]
    fn blocks_github_ceiling_blob() {
        let blobs = vec![StagedBlob {
            path: "huge.bin".into(),
            size_bytes: 100 * BYTES_PER_MIB,
            is_lfs_pointer: false,
            is_ignored: false,
        }];
        let eval = evaluate_staged_blobs(&config(), &blobs);
        assert!(eval.issues.iter().any(|i| i.code == "blob.github_ceiling"));
    }

    #[test]
    fn detects_staged_ignored_path() {
        let blobs = vec![StagedBlob {
            path: "build/output.bin".into(),
            size_bytes: 10,
            is_lfs_pointer: false,
            is_ignored: true,
        }];
        let eval = evaluate_staged_blobs(&config(), &blobs);
        assert!(eval.issues.iter().any(|i| i.code == "ignore.staged"));
    }
}
