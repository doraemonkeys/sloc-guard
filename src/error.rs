use std::path::PathBuf;

use thiserror::Error;

/// Represents the origin of a configuration value.
///
/// Used to track where configuration settings came from during extends resolution,
/// enabling precise error messages that identify which config file caused an issue.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigSource {
    /// Configuration from a local file.
    File { path: PathBuf },
    /// Configuration from a remote URL.
    Remote { url: String },
    /// Configuration from a built-in preset.
    Preset { name: String },
}

impl std::fmt::Display for ConfigSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::File { path } => write!(f, "{}", path.display()),
            Self::Remote { url } => write!(f, "{url}"),
            Self::Preset { name } => write!(f, "preset:{name}"),
        }
    }
}

impl ConfigSource {
    /// Create a file source.
    #[must_use]
    pub fn file(path: impl Into<PathBuf>) -> Self {
        Self::File { path: path.into() }
    }

    /// Create a remote source.
    #[must_use]
    pub fn remote(url: impl Into<String>) -> Self {
        Self::Remote { url: url.into() }
    }

    /// Create a preset source.
    #[must_use]
    pub fn preset(name: impl Into<String>) -> Self {
        Self::Preset { name: name.into() }
    }
}

#[derive(Error, Debug)]
pub enum SlocGuardError {
    #[error("Configuration error: {0}")]
    Config(String),

    /// Circular extends chain detected.
    #[error("Circular extends detected: {}", format_chain(chain))]
    CircularExtends {
        /// The chain of configs that led to the cycle (e.g., `a.toml` → `b.toml` → `a.toml`).
        chain: Vec<String>,
    },

    /// Extends chain exceeds maximum depth.
    #[error("Extends chain too deep: depth {depth} exceeds maximum {max}")]
    ExtendsTooDeep {
        /// Current depth when the limit was hit.
        depth: usize,
        /// Maximum allowed depth.
        max: usize,
        /// The chain of configs up to the error point.
        chain: Vec<String>,
    },

    /// Cannot resolve extends path relative to base.
    #[error("Cannot resolve extends path '{path}' from {base}")]
    ExtendsResolution {
        /// The relative path that couldn't be resolved.
        path: String,
        /// Description of the base (e.g., "remote config" or source path).
        base: String,
    },

    /// Config field has wrong type.
    #[error("Type mismatch in '{field}': expected {expected}, got {actual}{}", format_origin(origin.as_ref()))]
    TypeMismatch {
        /// The field path (e.g., `content.max_lines`).
        field: String,
        /// Expected type description.
        expected: String,
        /// Actual type found.
        actual: String,
        /// Which config the error originated from.
        origin: Option<ConfigSource>,
    },

    /// Semantic validation error (valid type but invalid value/constraint).
    #[error("Invalid configuration for '{field}': {message}{}", format_origin(origin.as_ref()))]
    Semantic {
        /// The field path (e.g., `content.warn_threshold`).
        field: String,
        /// Description of the semantic error.
        message: String,
        /// Which config the error originated from.
        origin: Option<ConfigSource>,
        /// Actionable suggestion for fixing the error.
        suggestion: Option<String>,
    },

    /// TOML syntax error with precise location.
    ///
    /// Used when parsing a single config file without extends inheritance,
    /// where line/column information is meaningful and accurate.
    #[error("Syntax error at line {line}, column {column}{}: {message}", format_origin(origin.as_ref()))]
    Syntax {
        /// The config origin that contains the syntax error.
        origin: Option<ConfigSource>,
        /// 1-based line number where the error occurred.
        line: usize,
        /// 1-based column number where the error occurred.
        column: usize,
        /// The error message describing what went wrong.
        message: String,
    },

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

/// Format a chain of config sources for display.
fn format_chain(chain: &[String]) -> String {
    chain.join(" → ")
}

/// Format an optional origin source for error messages.
fn format_origin(origin: Option<&ConfigSource>) -> String {
    origin.map_or_else(String::new, |source| format!(" (in {source})"))
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

    /// Creates a Syntax error from a TOML parse error and the original content.
    ///
    /// Extracts precise line/column information from the error's byte span.
    #[must_use]
    pub fn syntax_from_toml(
        error: &toml::de::Error,
        content: &str,
        origin: Option<ConfigSource>,
    ) -> Self {
        let (line, column) = error
            .span()
            .map_or((1, 1), |span| span_to_line_col(content, span.start));

        Self::Syntax {
            origin,
            line,
            column,
            message: error.message().to_string(),
        }
    }

    /// Returns the error type as a short string identifier.
    #[must_use]
    pub const fn error_type(&self) -> &'static str {
        match self {
            Self::Config(_)
            | Self::CircularExtends { .. }
            | Self::ExtendsTooDeep { .. }
            | Self::ExtendsResolution { .. }
            | Self::TypeMismatch { .. }
            | Self::Semantic { .. }
            | Self::Syntax { .. } => "Config",
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
            Self::CircularExtends { chain } => format!("circular extends: {}", format_chain(chain)),
            Self::ExtendsTooDeep { depth, max, .. } => {
                format!("extends chain depth {depth} exceeds maximum {max}")
            }
            Self::ExtendsResolution { path, base } => {
                format!("cannot resolve '{path}' from {base}")
            }
            Self::TypeMismatch {
                field,
                expected,
                actual,
                ..
            } => {
                format!("'{field}': expected {expected}, got {actual}")
            }
            Self::Semantic { field, message, .. } => format!("'{field}': {message}"),
            Self::Syntax {
                line,
                column,
                message,
                ..
            } => format!("line {line}, column {column}: {message}"),
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
            Self::CircularExtends { chain } | Self::ExtendsTooDeep { chain, .. } => {
                Some(format!("chain: {}", format_chain(chain)))
            }
            Self::TypeMismatch { origin, .. }
            | Self::Semantic { origin, .. }
            | Self::Syntax { origin, .. } => origin.as_ref().map(|o| format!("in {o}")),
            _ => None,
        }
    }

    /// Returns an actionable suggestion for resolving the error.
    #[must_use]
    pub fn suggestion(&self) -> Option<&str> {
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
            Self::TomlParse(_) | Self::Syntax { .. } => {
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
            Self::CircularExtends { .. } => {
                Some("Review extends chain to remove the circular reference")
            }
            Self::ExtendsTooDeep { .. } => {
                Some("Reduce inheritance depth by flattening config hierarchy or using presets")
            }
            Self::ExtendsResolution { .. } => Some(
                "Use absolute path or ensure relative path is valid from the base config location",
            ),
            Self::TypeMismatch { .. } => {
                Some("Check the expected type in the config documentation")
            }
            Self::Semantic { suggestion, .. } => suggestion.as_deref(),
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

/// Convert a byte position to 1-based line and column numbers.
///
/// Used to extract precise error locations from TOML parse errors.
/// Clamps the byte position to content length for safety.
#[must_use]
pub fn span_to_line_col(content: &str, byte_pos: usize) -> (usize, usize) {
    let clamped_pos = byte_pos.min(content.len());
    let prefix = &content[..clamped_pos];
    let line = prefix.matches('\n').count() + 1; // 1-based
    let last_newline = prefix.rfind('\n').map_or(0, |pos| pos + 1);
    let column = clamped_pos.saturating_sub(last_newline) + 1; // 1-based
    (line, column)
}

#[cfg(test)]
#[path = "error_tests.rs"]
mod tests;
