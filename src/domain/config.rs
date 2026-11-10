//! Versioned `.repo-guard.toml` configuration model and validation.

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Supported configuration schema version.
pub const CURRENT_SCHEMA_VERSION: u32 = 1;

/// Repository policy configuration stored in `.repo-guard.toml`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RepoGuardConfig {
    pub schema_version: u32,
    pub lfs_threshold_mib: u64,
    pub github_safety_ceiling_mib: u64,
    pub block_small_lfs_files: bool,
    pub output_format: String,
    #[serde(default)]
    pub ignored_paths: Vec<String>,
}

impl Default for RepoGuardConfig {
    fn default() -> Self {
        Self {
            schema_version: CURRENT_SCHEMA_VERSION,
            lfs_threshold_mib: 50,
            github_safety_ceiling_mib: 95,
            block_small_lfs_files: false,
            output_format: "text".into(),
            ignored_paths: Vec::new(),
        }
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ConfigValidationError {
    #[error("unsupported schema_version {0}; expected {CURRENT_SCHEMA_VERSION}")]
    UnsupportedSchemaVersion(u32),
    #[error("lfs_threshold_mib must be greater than zero")]
    InvalidLfsThreshold,
    #[error("github_safety_ceiling_mib must be greater than or equal to lfs_threshold_mib")]
    InvalidSafetyCeiling,
    #[error("output_format must be \"text\" or \"json\"")]
    InvalidOutputFormat,
}

impl RepoGuardConfig {
    /// Validate configuration values after parsing.
    pub fn validate(&self) -> Result<(), ConfigValidationError> {
        if self.schema_version != CURRENT_SCHEMA_VERSION {
            return Err(ConfigValidationError::UnsupportedSchemaVersion(
                self.schema_version,
            ));
        }
        if self.lfs_threshold_mib == 0 {
            return Err(ConfigValidationError::InvalidLfsThreshold);
        }
        if self.github_safety_ceiling_mib < self.lfs_threshold_mib {
            return Err(ConfigValidationError::InvalidSafetyCeiling);
        }
        if self.output_format != "text" && self.output_format != "json" {
            return Err(ConfigValidationError::InvalidOutputFormat);
        }
        Ok(())
    }

    /// Serialize to TOML for writing a new configuration file.
    pub fn to_toml_string(&self) -> Result<String, toml::ser::Error> {
        toml::to_string_pretty(self)
    }
}

/// Parse and validate configuration text.
pub fn parse_config(text: &str) -> Result<RepoGuardConfig, ConfigParseError> {
    let config: RepoGuardConfig =
        toml::from_str(text).map_err(|source| ConfigParseError::Parse { source })?;
    config
        .validate()
        .map_err(|source| ConfigParseError::Validation { source })?;
    Ok(config)
}

#[derive(Debug, Error)]
pub enum ConfigParseError {
    #[error("failed to parse .repo-guard.toml: {source}")]
    Parse {
        #[source]
        source: toml::de::Error,
    },
    #[error("invalid .repo-guard.toml: {source}")]
    Validation {
        #[source]
        source: ConfigValidationError,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_is_valid() {
        assert!(RepoGuardConfig::default().validate().is_ok());
    }

    #[test]
    fn rejects_unsupported_schema_version() {
        let config = RepoGuardConfig {
            schema_version: 99,
            ..RepoGuardConfig::default()
        };
        assert_eq!(
            config.validate(),
            Err(ConfigValidationError::UnsupportedSchemaVersion(99))
        );
    }

    #[test]
    fn parses_example_config() {
        let text = r#"
schema_version = 1
lfs_threshold_mib = 50
github_safety_ceiling_mib = 95
block_small_lfs_files = false
output_format = "text"
"#;
        let config = parse_config(text).unwrap();
        assert_eq!(config.lfs_threshold_mib, 50);
    }

    #[test]
    fn legacy_config_with_removed_field_still_parses() {
        let text = r#"
schema_version = 1
lfs_threshold_mib = 50
github_safety_ceiling_mib = 95
require_exact_lfs_paths = true
block_small_lfs_files = false
output_format = "text"
"#;
        let config = parse_config(text).unwrap();
        assert_eq!(config.lfs_threshold_mib, 50);
    }
}
