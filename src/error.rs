use std::path::PathBuf;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum SlocGuardError {
    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Failed to access file: {path}")]
    FileAccess {
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

    #[error("{}", format_io_error(source, path, operation))]
    Io {
        #[source]
        source: std::io::Error,
        path: Option<PathBuf>,
        operation: Option<&'static str>,
    },

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

/// Formats IO error with optional context for display.
/// Uses references to Options as required by thiserror's `#[error(...)]` macro expansion.
#[allow(clippy::ref_option, clippy::ref_option_ref)]
fn format_io_error(
    source: &std::io::Error,
    path: &Option<PathBuf>,
    operation: &Option<&'static str>,
) -> String {
    match (path.as_ref(), *operation) {
        (Some(p), Some(op)) => format!("IO error ({op} '{}'): {source}", p.display()),
        (Some(p), None) => format!("IO error ('{}'): {source}", p.display()),
        (None, Some(op)) => format!("IO error ({op}): {source}"),
        (None, None) => format!("IO error: {source}"),
    }
}

impl From<std::io::Error> for SlocGuardError {
    fn from(e: std::io::Error) -> Self {
        Self::Io {
            source: e,
            path: None,
            operation: None,
        }
    }
}

impl SlocGuardError {
    /// Creates an IO error with path context.
    #[must_use]
    pub const fn io_with_path(source: std::io::Error, path: PathBuf) -> Self {
        Self::Io {
            source,
            path: Some(path),
            operation: None,
        }
    }

    /// Creates an IO error with path and operation context.
    #[must_use]
    pub const fn io_with_context(
        source: std::io::Error,
        path: PathBuf,
        operation: &'static str,
    ) -> Self {
        Self::Io {
            source,
            path: Some(path),
            operation: Some(operation),
        }
    }

    /// Returns the error type as a short string identifier.
    #[must_use]
    pub const fn error_type(&self) -> &'static str {
        match self {
            Self::Config(_) => "Config",
            Self::FileAccess { .. } => "FileAccess",
            Self::InvalidPattern { .. } => "InvalidPattern",
            Self::Io { .. } => "IO",
            Self::TomlParse(_) => "TOML",
            Self::JsonSerialize(_) => "JSON",
            Self::Git(_) | Self::GitRepoNotFound(_) => "Git",
            Self::RemoteConfigHashMismatch { .. } => "RemoteConfig",
        }
    }

    /// Returns the error message without the type prefix.
    /// Includes error kind for `FileAccess` and glob error for `InvalidPattern`.
    #[must_use]
    pub fn message(&self) -> String {
        match self {
            Self::FileAccess { path, source } => {
                format!("{} ({})", path.display(), source.kind())
            }
            Self::InvalidPattern { pattern, source } => {
                format!("{pattern}: {source}")
            }
            Self::Io {
                source,
                path,
                operation,
            } => match (path, operation) {
                (Some(p), Some(op)) => format!("{op} '{}': {source}", p.display()),
                (Some(p), None) => format!("'{}': {source}", p.display()),
                (None, Some(op)) => format!("{op}: {source}"),
                (None, None) => source.to_string(),
            },
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
            Self::FileAccess { source, .. } => Some(format!("{source} ({})", source.kind())),
            Self::InvalidPattern { source, .. } => Some(source.to_string()),
            Self::Io {
                source,
                path,
                operation,
            } => {
                let kind = source.kind();
                match (path, operation) {
                    (Some(p), Some(op)) => {
                        Some(format!("{op} '{}': {source} ({kind})", p.display()))
                    }
                    (Some(p), None) => Some(format!("'{}': {source} ({kind})", p.display())),
                    (None, Some(op)) => Some(format!("{op}: {source} ({kind})")),
                    (None, None) => Some(format!("{source} ({kind})")),
                }
            }
            Self::RemoteConfigHashMismatch {
                expected, actual, ..
            } => Some(format!("expected {expected}, got {actual}")),
            _ => None,
        }
    }

    /// Returns an actionable suggestion for resolving the error.
    #[must_use]
    pub fn suggestion(&self) -> Option<&'static str> {
        match self {
            Self::Config(_) => {
                Some("Check the config file format and value ranges in .sloc-guard.toml")
            }
            Self::FileAccess { source, .. } | Self::Io { source, .. } => {
                Self::io_suggestion(source.kind())
            }
            Self::InvalidPattern { .. } => Some(
                "Check glob pattern syntax: use '*' for wildcards, '**' for recursive matching",
            ),
            Self::TomlParse(_) => {
                Some("Check TOML syntax: ensure proper quoting and bracket matching")
            }
            Self::JsonSerialize(_) => {
                Some("Check for non-serializable data types or malformed structures")
            }
            Self::Git(_) => Some("Ensure git is installed and the repository is accessible"),
            Self::GitRepoNotFound(_) => Some(
                "Run 'git init' to create a repository, or run from within an existing git repository",
            ),
            Self::RemoteConfigHashMismatch { .. } => {
                Some("Update extends_sha256 in config, or verify the remote config URL is correct")
            }
        }
    }

    /// Returns a suggestion based on IO error kind.
    const fn io_suggestion(kind: std::io::ErrorKind) -> Option<&'static str> {
        match kind {
            std::io::ErrorKind::NotFound => Some("Verify the file path exists"),
            std::io::ErrorKind::PermissionDenied => {
                Some("Check file permissions or run with appropriate access rights")
            }
            std::io::ErrorKind::InvalidData => {
                Some("The file may be corrupted or in an unexpected format")
            }
            std::io::ErrorKind::TimedOut => Some("Check network connectivity or increase timeout"),
            std::io::ErrorKind::ConnectionRefused | std::io::ErrorKind::ConnectionReset => {
                Some("Check network connectivity and ensure the remote server is available")
            }
            _ => None,
        }
    }
}

pub type Result<T> = std::result::Result<T, SlocGuardError>;

#[cfg(test)]
#[path = "error_tests.rs"]
mod tests;
