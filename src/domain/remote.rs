//! Remote repository validation logic.

use crate::domain::report::Issue;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RemoteInfo {
    pub name: String,
    pub url: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RemoteEvaluation {
    pub remotes: Vec<RemoteInfo>,
    pub issues: Vec<Issue>,
    pub errors: usize,
}

pub fn evaluate_remotes(
    remotes: Vec<RemoteInfo>,
    require_origin: bool,
    origin_reachable: Option<bool>,
) -> RemoteEvaluation {
    let mut eval = RemoteEvaluation {
        remotes: remotes.clone(),
        ..RemoteEvaluation::default()
    };

    if remotes.is_empty() {
        eval.issues.push(Issue::warning(
            "remote.none",
            "No Git remotes are configured",
        ));
    }

    for remote in &remotes {
        eval.issues.push(Issue::info(
            format!("remote.{}", remote.name),
            format!("{} -> {}", remote.name, remote.url),
        ));
    }

    if require_origin && !remotes.iter().any(|r| r.name == "origin") {
        eval.errors += 1;
        eval.issues.push(Issue::error(
            "remote.origin_missing",
            "Required remote \"origin\" is not configured",
        ));
    }

    if let Some(false) = origin_reachable {
        eval.errors += 1;
        eval.issues.push(Issue::error(
            "remote.origin_unreachable",
            "Could not reach origin with a non-destructive connectivity check",
        ));
    } else if origin_reachable == Some(true) {
        eval.issues.push(Issue::info(
            "remote.origin_reachable",
            "origin responded to git ls-remote --heads origin",
        ));
    }

    eval
}
