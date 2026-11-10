//! History and outgoing-object validation report construction.

use crate::domain::exit_code::ExitCode;
use crate::domain::policy::{evaluate_staged_blobs, PolicyEvaluation, StagedBlob};
use crate::domain::prepush::{OutgoingRange, OutgoingRefPlan};
use crate::domain::report::{CommandReport, Issue};

/// Policy mode when a remote baseline cannot be resolved locally.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnresolvedPolicy {
    Strict,
    Advisory,
}

/// A blob candidate discovered in outgoing history.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutgoingBlob {
    pub oid: String,
    pub path: Option<String>,
    pub size_bytes: u64,
    pub is_lfs_pointer: bool,
}

impl OutgoingBlob {
    pub fn display_path(&self) -> String {
        self.path.clone().unwrap_or_else(|| self.oid.clone())
    }

    pub fn as_staged_blob(&self) -> StagedBlob {
        StagedBlob {
            path: self.display_path(),
            size_bytes: self.size_bytes,
            is_lfs_pointer: self.is_lfs_pointer,
            is_ignored: false,
        }
    }
}

/// Outcome of planning outgoing object inspection.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct OutgoingInspection {
    pub blobs: Vec<OutgoingBlob>,
    pub unresolved_refs: Vec<OutgoingRefPlan>,
    pub skipped_deletions: usize,
    pub inspected_ranges: Vec<String>,
}

pub fn build_history_report(
    command: &str,
    inspection: &OutgoingInspection,
    evaluation: &PolicyEvaluation,
    unresolved_policy: UnresolvedPolicy,
) -> (CommandReport, ExitCode) {
    let mut report = CommandReport::new(command, ExitCode::Success);
    let mut exit_code = ExitCode::Success;

    for range in &inspection.inspected_ranges {
        report = report.with_issue(Issue::info(
            "history.range_inspected",
            format!("Inspected outgoing range {range}"),
        ));
    }

    if inspection.skipped_deletions > 0 {
        report = report.with_issue(Issue::info(
            "history.deletions_skipped",
            format!(
                "Skipped {} ref deletion(s); deletions are not validated",
                inspection.skipped_deletions
            ),
        ));
    }

    for plan in &inspection.unresolved_refs {
        let message = format!(
            "Could not resolve remote baseline {} for {}; fetch the remote ref or run with explicit advisory mode only when you accept incomplete validation",
            plan.update.remote_sha, plan.update.remote_ref
        );
        match unresolved_policy {
            UnresolvedPolicy::Strict => {
                report = report.with_issue(Issue::error("history.unresolved_baseline", message));
                exit_code = ExitCode::PolicyViolation;
            }
            UnresolvedPolicy::Advisory => {
                report = report.with_issue(Issue::warning("history.unresolved_baseline", message));
            }
        }
    }

    for issue in &evaluation.issues {
        report = report.with_issue(issue.clone());
    }

    if evaluation.has_violations() {
        exit_code = ExitCode::PolicyViolation;
    }

    for command in &evaluation.repair_commands {
        report = report.with_issue(Issue::info("history.repair", command));
    }

    let summary = serde_json::json!({
        "blobs_inspected": inspection.blobs.len(),
        "violations": evaluation.violation_count,
        "unresolved_refs": inspection.unresolved_refs.len(),
        "inspected_ranges": inspection.inspected_ranges,
        "skipped_deletions": inspection.skipped_deletions,
    });

    (report.with_summary(summary).finalize(exit_code), exit_code)
}

pub fn evaluate_outgoing_blobs(
    config: &crate::domain::config::RepoGuardConfig,
    blobs: &[OutgoingBlob],
) -> PolicyEvaluation {
    let staged: Vec<_> = blobs.iter().map(OutgoingBlob::as_staged_blob).collect();
    evaluate_staged_blobs(config, &staged)
}

pub fn merge_outgoing_plans(plans: &[OutgoingRefPlan]) -> Vec<String> {
    let mut specs = Vec::new();
    for plan in plans {
        if let Some(spec) = crate::domain::prepush::rev_list_spec(&plan.range) {
            if !specs.contains(&spec) {
                specs.push(spec);
            }
        }
    }
    specs
}

pub fn count_skipped_deletions(plans: &[OutgoingRefPlan]) -> usize {
    plans
        .iter()
        .filter(|p| matches!(p.range, OutgoingRange::SkipDeletion))
        .count()
}

pub fn unresolved_plans(plans: &[OutgoingRefPlan]) -> Vec<OutgoingRefPlan> {
    plans
        .iter()
        .filter(|p| matches!(p.range, OutgoingRange::UnresolvedBaseline { .. }))
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::config::RepoGuardConfig;
    use crate::domain::prepush::RefUpdate;

    #[test]
    fn strict_unresolved_baseline_fails() {
        let inspection = OutgoingInspection {
            unresolved_refs: vec![OutgoingRefPlan {
                update: RefUpdate {
                    local_ref: "refs/heads/main".into(),
                    local_sha: "abc".into(),
                    remote_ref: "refs/heads/main".into(),
                    remote_sha: "missing".into(),
                },
                range: OutgoingRange::UnresolvedBaseline {
                    remote_sha: "missing".into(),
                    head: "abc".into(),
                },
            }],
            ..Default::default()
        };
        let evaluation = PolicyEvaluation::default();
        let (_, code) = build_history_report(
            "history check",
            &inspection,
            &evaluation,
            UnresolvedPolicy::Strict,
        );
        assert_eq!(code, ExitCode::PolicyViolation);
    }

    #[test]
    fn advisory_unresolved_allows_success_without_blob_violations() {
        let inspection = OutgoingInspection {
            unresolved_refs: vec![OutgoingRefPlan {
                update: RefUpdate {
                    local_ref: "refs/heads/main".into(),
                    local_sha: "abc".into(),
                    remote_ref: "refs/heads/main".into(),
                    remote_sha: "missing".into(),
                },
                range: OutgoingRange::UnresolvedBaseline {
                    remote_sha: "missing".into(),
                    head: "abc".into(),
                },
            }],
            ..Default::default()
        };
        let evaluation = PolicyEvaluation::default();
        let (_, code) = build_history_report(
            "history check",
            &inspection,
            &evaluation,
            UnresolvedPolicy::Advisory,
        );
        assert_eq!(code, ExitCode::Success);
    }

    #[test]
    fn large_outgoing_blob_violates_policy() {
        let config = RepoGuardConfig::default();
        let blobs = vec![OutgoingBlob {
            oid: "abc".into(),
            path: Some("big.bin".into()),
            size_bytes: 60 * 1024 * 1024,
            is_lfs_pointer: false,
        }];
        let evaluation = evaluate_outgoing_blobs(&config, &blobs);
        assert!(evaluation.has_violations());
    }
}
