use std::path::PathBuf;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum SlocGuardError {
    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Failed to read file: {path}")]
    FileRead {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("Invalid glob pattern: {pattern}")]
    InvalidPattern {
        pattern: String,
        #[source]
        source: globset::Error,
    },

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("TOML parse error: {0}")]
    TomlParse(#[from] toml::de::Error),

    #[error("JSON serialization error: {0}")]
    JsonSerialize(#[from] serde_json::Error),

    #[error("Git error: {0}")]
    Git(String),

    #[error("Not a git repository: {0}")]
    GitRepoNotFound(String),

    #[error("Remote config hash mismatch for {url}: expected {expected}, got {actual}")]
    RemoteConfigHashMismatch {
        url: String,
        expected: String,
        actual: String,
    },
}

pub type Result<T> = std::result::Result<T, SlocGuardError>;

#[cfg(test)]
#[path = "error_tests.rs"]
mod tests;
