//! Git LFS consistency validation logic.

use crate::domain::report::Issue;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GitAttributesRule {
    pub pattern: String,
    pub is_filename: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LfsCheckEvaluation {
    pub issues: Vec<Issue>,
    pub violations: usize,
}

pub fn parse_gitattributes(content: &str) -> Vec<GitAttributesRule> {
    let mut rules = Vec::new();
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line.contains("filter=lfs") {
            if let Some(path) = line.split_whitespace().next() {
                rules.push(GitAttributesRule {
                    pattern: path.to_string(),
                    is_filename: !path.contains('*'),
                });
            }
        }
    }
    rules
}

pub fn evaluate_lfs_consistency(
    git_lfs_installed: bool,
    rules: &[GitAttributesRule],
    staged: &[(String, bool, bool)],
) -> LfsCheckEvaluation {
    let mut eval = LfsCheckEvaluation::default();

    if !git_lfs_installed {
        eval.violations += 1;
        eval.issues.push(Issue::error(
            "git_lfs.missing",
            "Git LFS is not installed or not callable",
        ));
    }

    if rules.is_empty() {
        eval.issues.push(Issue::warning(
            "lfs.no_rules",
            "No Git LFS rules found in .gitattributes",
        ));
    }

    for (path, is_lfs_pointer, is_staged) in staged {
        if !is_staged {
            continue;
        }
        let matched = rules.iter().any(|rule| attribute_matches(rule, path));
        if matched && !is_lfs_pointer {
            eval.violations += 1;
            eval.issues.push(Issue::error(
                "lfs.not_pointer",
                format!("{path} is staged as a regular blob but .gitattributes expects Git LFS"),
            ));
        }
    }

    eval
}

fn attribute_matches(rule: &GitAttributesRule, path: &str) -> bool {
    if rule.is_filename {
        path == rule.pattern
    } else {
        // Simple glob: only supports * suffix/prefix patterns used in presets
        if let Some(stripped) = rule.pattern.strip_prefix('*') {
            path.ends_with(stripped)
        } else if let Some(stripped) = rule.pattern.strip_suffix('*') {
            path.starts_with(stripped)
        } else {
            path == rule.pattern
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_lfs_rules_from_gitattributes() {
        let content = "*.zip filter=lfs diff=lfs merge=lfs -text\nbig.bin filter=lfs\n";
        let rules = parse_gitattributes(content);
        assert_eq!(rules.len(), 2);
    }
}
