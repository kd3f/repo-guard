//! Git pre-push stdin parsing and outgoing range planning (pure logic).

pub const ZERO_SHA: &str = "0000000000000000000000000000000000000000";

/// One ref update line from Git's pre-push stdin protocol.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RefUpdate {
    pub local_ref: String,
    pub local_sha: String,
    pub remote_ref: String,
    pub remote_sha: String,
}

/// How an outgoing ref range should be traversed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OutgoingRange {
    SkipDeletion,
    NewBranch { head: String },
    Incremental { base: String, head: String },
    UnresolvedBaseline { remote_sha: String, head: String },
}

/// Planned outgoing inspection for one ref update.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutgoingRefPlan {
    pub update: RefUpdate,
    pub range: OutgoingRange,
}

/// Parse Git pre-push stdin ref updates.
pub fn parse_pre_push_stdin(input: &str) -> Vec<RefUpdate> {
    input.lines().filter_map(parse_pre_push_line).collect()
}

fn parse_pre_push_line(line: &str) -> Option<RefUpdate> {
    let line = line.trim();
    if line.is_empty() {
        return None;
    }
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() != 4 {
        return None;
    }
    Some(RefUpdate {
        local_ref: parts[0].into(),
        local_sha: parts[1].into(),
        remote_ref: parts[2].into(),
        remote_sha: parts[3].into(),
    })
}

pub fn is_deletion(update: &RefUpdate) -> bool {
    update.local_sha == ZERO_SHA
}

pub fn is_new_remote_branch(update: &RefUpdate) -> bool {
    update.remote_sha == ZERO_SHA && update.local_sha != ZERO_SHA
}

/// Build outgoing range plans from parsed ref updates and local object availability.
pub fn plan_outgoing_refs(
    updates: &[RefUpdate],
    remote_sha_exists: impl Fn(&str) -> bool,
) -> Vec<OutgoingRefPlan> {
    updates
        .iter()
        .map(|update| OutgoingRefPlan {
            update: update.clone(),
            range: plan_single_ref(update, &remote_sha_exists),
        })
        .collect()
}

fn plan_single_ref(update: &RefUpdate, remote_sha_exists: &impl Fn(&str) -> bool) -> OutgoingRange {
    if is_deletion(update) {
        return OutgoingRange::SkipDeletion;
    }
    if is_new_remote_branch(update) {
        return OutgoingRange::NewBranch {
            head: update.local_sha.clone(),
        };
    }
    if remote_sha_exists(&update.remote_sha) {
        return OutgoingRange::Incremental {
            base: update.remote_sha.clone(),
            head: update.local_sha.clone(),
        };
    }
    OutgoingRange::UnresolvedBaseline {
        remote_sha: update.remote_sha.clone(),
        head: update.local_sha.clone(),
    }
}

/// Convert an outgoing range to a `git rev-list` range argument.
pub fn rev_list_spec(range: &OutgoingRange) -> Option<String> {
    match range {
        OutgoingRange::SkipDeletion => None,
        OutgoingRange::NewBranch { head } => Some(head.clone()),
        OutgoingRange::Incremental { base, head } => Some(format!("{base}..{head}")),
        OutgoingRange::UnresolvedBaseline { .. } => None,
    }
}

/// Parse a user-supplied history range such as `main..feature`.
pub fn parse_history_range(input: &str) -> Result<(String, String), String> {
    let trimmed = input.trim();
    let Some((base, head)) = trimmed.split_once("..") else {
        return Err(format!(
            "Invalid range \"{input}\"; expected format <base>..<head>"
        ));
    };
    if base.is_empty() || head.is_empty() {
        return Err(format!(
            "Invalid range \"{input}\"; base and head must both be non-empty"
        ));
    }
    Ok((base.into(), head.into()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_pre_push_lines() {
        let input = "refs/heads/main abc111 refs/heads/main def222\n";
        let updates = parse_pre_push_stdin(input);
        assert_eq!(updates.len(), 1);
        assert_eq!(updates[0].local_sha, "abc111");
    }

    #[test]
    fn skips_deletions_in_plan() {
        let update = RefUpdate {
            local_ref: "refs/heads/main".into(),
            local_sha: ZERO_SHA.into(),
            remote_ref: "refs/heads/main".into(),
            remote_sha: "abc".into(),
        };
        let plans = plan_outgoing_refs(&[update], |_| true);
        assert!(matches!(plans[0].range, OutgoingRange::SkipDeletion));
    }

    #[test]
    fn new_branch_uses_head_only_traversal() {
        let update = RefUpdate {
            local_ref: "refs/heads/feature".into(),
            local_sha: "local".into(),
            remote_ref: "refs/heads/feature".into(),
            remote_sha: ZERO_SHA.into(),
        };
        let plans = plan_outgoing_refs(&[update], |_| false);
        assert!(matches!(plans[0].range, OutgoingRange::NewBranch { .. }));
    }

    #[test]
    fn missing_remote_sha_is_unresolved() {
        let update = RefUpdate {
            local_ref: "refs/heads/main".into(),
            local_sha: "local".into(),
            remote_ref: "refs/heads/main".into(),
            remote_sha: "missing".into(),
        };
        let plans = plan_outgoing_refs(&[update], |_| false);
        assert!(matches!(
            plans[0].range,
            OutgoingRange::UnresolvedBaseline { .. }
        ));
    }

    #[test]
    fn parses_history_range() {
        let (base, head) = parse_history_range("main..feature").unwrap();
        assert_eq!(base, "main");
        assert_eq!(head, "feature");
    }
}
