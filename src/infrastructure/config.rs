//! Configuration file loading and persistence.

use std::fs;
use std::path::{Path, PathBuf};

use thiserror::Error;

use crate::domain::config::{parse_config, ConfigParseError, RepoGuardConfig};
use crate::support::constants::CONFIG_FILE_NAME;

#[derive(Debug, Error)]
pub enum ConfigIoError {
    #[error("failed to read {path}: {source}")]
    Read {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to write {path}: {source}")]
    Write {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("{0}")]
    Parse(#[from] ConfigParseError),
}

/// Load and validate configuration from a repository root.
pub fn load_config(repo_root: &Path) -> Result<RepoGuardConfig, ConfigIoError> {
    let path = repo_root.join(CONFIG_FILE_NAME);
    let text = fs::read_to_string(&path).map_err(|source| ConfigIoError::Read {
        path: path.clone(),
        source,
    })?;
    parse_config(&text).map_err(ConfigIoError::from)
}

/// Write configuration to a repository root, creating the file.
pub fn write_config(repo_root: &Path, config: &RepoGuardConfig) -> Result<(), ConfigIoError> {
    let path = repo_root.join(CONFIG_FILE_NAME);
    let text = config
        .to_toml_string()
        .map_err(|source| ConfigIoError::Write {
            path: path.clone(),
            source: std::io::Error::new(std::io::ErrorKind::InvalidData, source),
        })?;
    fs::write(&path, text).map_err(|source| ConfigIoError::Write { path, source })?;
    Ok(())
}

/// Try to load config; returns None if file is missing.
pub fn try_load_config(repo_root: &Path) -> Result<Option<RepoGuardConfig>, ConfigIoError> {
    let path = repo_root.join(CONFIG_FILE_NAME);
    if !path.is_file() {
        return Ok(None);
    }
    load_config(repo_root).map(Some)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::config::RepoGuardConfig;
    use tempfile::tempdir;

    #[test]
    fn round_trip_config_file() {
        let dir = tempdir().unwrap();
        let config = RepoGuardConfig::default();
        write_config(dir.path(), &config).unwrap();
        let loaded = load_config(dir.path()).unwrap();
        assert_eq!(loaded, config);
    }
}
