//! Stable process exit codes for repo-guard commands.

/// Exit code returned on success.
pub const SUCCESS: i32 = 0;
/// Policy violation detected.
pub const POLICY_VIOLATION: i32 = 2;
/// Required external dependency is missing.
pub const MISSING_DEPENDENCY: i32 = 3;
/// Repository or configuration error.
pub const REPO_OR_CONFIG_ERROR: i32 = 4;
/// Git or Git LFS command failure.
pub const GIT_COMMAND_FAILURE: i32 = 5;
/// Unexpected internal error.
pub const INTERNAL_ERROR: i32 = 6;

/// Named exit codes used across commands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum ExitCode {
    Success,
    PolicyViolation,
    MissingDependency,
    RepoOrConfigError,
    GitCommandFailure,
    InternalError,
}

impl ExitCode {
    /// Numeric value suitable for `std::process::exit`.
    pub fn as_i32(self) -> i32 {
        match self {
            Self::Success => SUCCESS,
            Self::PolicyViolation => POLICY_VIOLATION,
            Self::MissingDependency => MISSING_DEPENDENCY,
            Self::RepoOrConfigError => REPO_OR_CONFIG_ERROR,
            Self::GitCommandFailure => GIT_COMMAND_FAILURE,
            Self::InternalError => INTERNAL_ERROR,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exit_codes_match_project_spec() {
        assert_eq!(ExitCode::Success.as_i32(), 0);
        assert_eq!(ExitCode::PolicyViolation.as_i32(), 2);
        assert_eq!(ExitCode::MissingDependency.as_i32(), 3);
        assert_eq!(ExitCode::RepoOrConfigError.as_i32(), 4);
        assert_eq!(ExitCode::GitCommandFailure.as_i32(), 5);
        assert_eq!(ExitCode::InternalError.as_i32(), 6);
    }
}
