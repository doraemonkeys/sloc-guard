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

impl SlocGuardError {
    /// Returns the error type as a short string identifier.
    #[must_use]
    pub const fn error_type(&self) -> &'static str {
        match self {
            Self::Config(_) => "Config",
            Self::FileRead { .. } => "FileRead",
            Self::InvalidPattern { .. } => "InvalidPattern",
            Self::Io(_) => "IO",
            Self::TomlParse(_) => "TOML",
            Self::JsonSerialize(_) => "JSON",
            Self::Git(_) | Self::GitRepoNotFound(_) => "Git",
            Self::RemoteConfigHashMismatch { .. } => "RemoteConfig",
        }
    }

    /// Returns the error message without the type prefix.
    #[must_use]
    pub fn message(&self) -> String {
        match self {
            Self::FileRead { path, .. } => path.display().to_string(),
            Self::InvalidPattern { pattern, .. } => pattern.clone(),
            Self::Io(e) => e.to_string(),
            Self::TomlParse(e) => e.to_string(),
            Self::JsonSerialize(e) => e.to_string(),
            Self::Config(msg) | Self::Git(msg) | Self::GitRepoNotFound(msg) => msg.clone(),
            Self::RemoteConfigHashMismatch { url, .. } => format!("hash mismatch for {url}"),
        }
    }

    /// Returns optional detail information (source error details).
    #[must_use]
    pub fn detail(&self) -> Option<String> {
        match self {
            Self::FileRead { source, .. } => Some(format!("{} ({})", source, source.kind())),
            Self::InvalidPattern { source, .. } => Some(source.to_string()),
            Self::RemoteConfigHashMismatch {
                expected, actual, ..
            } => Some(format!("expected {expected}, got {actual}")),
            _ => None,
        }
    }
}

pub type Result<T> = std::result::Result<T, SlocGuardError>;

#[cfg(test)]
#[path = "error_tests.rs"]
mod tests;
