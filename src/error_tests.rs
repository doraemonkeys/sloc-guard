use std::path::PathBuf;

use super::*;

#[test]
fn error_display_config() {
    let err = SlocGuardError::Config("invalid threshold".to_string());
    assert_eq!(err.to_string(), "Configuration error: invalid threshold");
}

#[test]
fn error_display_file_read() {
    let err = SlocGuardError::FileRead {
        path: PathBuf::from("test.rs"),
        source: std::io::Error::new(std::io::ErrorKind::NotFound, "file not found"),
    };
    assert!(err.to_string().contains("test.rs"));
}

#[test]
fn error_display_git() {
    let err = SlocGuardError::Git("Failed to get git index".to_string());
    assert_eq!(err.to_string(), "Git error: Failed to get git index");
}

#[test]
fn error_display_git_repo_not_found() {
    let err = SlocGuardError::GitRepoNotFound("not a git repository".to_string());
    assert_eq!(
        err.to_string(),
        "Not a git repository: not a git repository"
    );
}

#[test]
fn error_type_returns_correct_type() {
    assert_eq!(
        SlocGuardError::Config("test".to_string()).error_type(),
        "Config"
    );
    assert_eq!(
        SlocGuardError::FileRead {
            path: PathBuf::from("test.rs"),
            source: std::io::Error::new(std::io::ErrorKind::NotFound, "not found"),
        }
        .error_type(),
        "FileRead"
    );
    assert_eq!(SlocGuardError::Git("test".to_string()).error_type(), "Git");
    assert_eq!(
        SlocGuardError::GitRepoNotFound("test".to_string()).error_type(),
        "Git"
    );
    assert_eq!(
        SlocGuardError::Io(std::io::Error::other("test")).error_type(),
        "IO"
    );
}

#[test]
fn error_message_extracts_message() {
    let err = SlocGuardError::Config("invalid config".to_string());
    assert_eq!(err.message(), "invalid config");

    let err = SlocGuardError::FileRead {
        path: PathBuf::from("test.rs"),
        source: std::io::Error::new(std::io::ErrorKind::NotFound, "not found"),
    };
    assert_eq!(err.message(), "test.rs");

    let err = SlocGuardError::Git("git error".to_string());
    assert_eq!(err.message(), "git error");
}

#[test]
fn error_detail_returns_source_info() {
    let err = SlocGuardError::Config("test".to_string());
    assert!(err.detail().is_none());

    let err = SlocGuardError::FileRead {
        path: PathBuf::from("test.rs"),
        source: std::io::Error::new(std::io::ErrorKind::NotFound, "file not found"),
    };
    let detail = err.detail().unwrap();
    assert!(detail.contains("file not found"));
    // Error kind format varies by platform, just check it contains some error info
    assert!(!detail.is_empty());

    let err = SlocGuardError::RemoteConfigHashMismatch {
        url: "https://example.com".to_string(),
        expected: "abc123".to_string(),
        actual: "def456".to_string(),
    };
    let detail = err.detail().unwrap();
    assert!(detail.contains("abc123"));
    assert!(detail.contains("def456"));
}

#[test]
fn suggestion_config_error() {
    let err = SlocGuardError::Config("invalid threshold".to_string());
    let suggestion = err.suggestion().unwrap();
    assert!(suggestion.contains("config file format"));
}

#[test]
fn suggestion_file_read_not_found() {
    let err = SlocGuardError::FileRead {
        path: PathBuf::from("missing.rs"),
        source: std::io::Error::new(std::io::ErrorKind::NotFound, "not found"),
    };
    let suggestion = err.suggestion().unwrap();
    assert!(suggestion.contains("file path exists"));
}

#[test]
fn suggestion_file_read_permission_denied() {
    let err = SlocGuardError::FileRead {
        path: PathBuf::from("protected.rs"),
        source: std::io::Error::new(std::io::ErrorKind::PermissionDenied, "access denied"),
    };
    let suggestion = err.suggestion().unwrap();
    assert!(suggestion.contains("permissions"));
}

#[test]
fn suggestion_file_read_other_error_has_none() {
    let err = SlocGuardError::FileRead {
        path: PathBuf::from("unknown.rs"),
        source: std::io::Error::other("unknown error"),
    };
    // Other IO errors may not have specific suggestions
    assert!(err.suggestion().is_none());
}

#[test]
fn suggestion_invalid_pattern() {
    let glob_err = globset::Glob::new("[invalid").unwrap_err();
    let err = SlocGuardError::InvalidPattern {
        pattern: "[invalid".to_string(),
        source: glob_err,
    };
    let suggestion = err.suggestion().unwrap();
    assert!(suggestion.contains("glob pattern syntax"));
}

#[test]
fn suggestion_io_error_not_found() {
    let err = SlocGuardError::Io(std::io::Error::new(
        std::io::ErrorKind::NotFound,
        "not found",
    ));
    let suggestion = err.suggestion().unwrap();
    assert!(suggestion.contains("file path exists"));
}

#[test]
fn suggestion_io_error_permission_denied() {
    let err = SlocGuardError::Io(std::io::Error::new(
        std::io::ErrorKind::PermissionDenied,
        "denied",
    ));
    let suggestion = err.suggestion().unwrap();
    assert!(suggestion.contains("permissions"));
}

#[test]
fn suggestion_io_error_other_has_none() {
    let err = SlocGuardError::Io(std::io::Error::other("custom error"));
    assert!(err.suggestion().is_none());
}

#[test]
fn suggestion_toml_parse() {
    let toml_err: std::result::Result<toml::Value, _> = toml::from_str("invalid = [");
    let err = SlocGuardError::TomlParse(toml_err.unwrap_err());
    let suggestion = err.suggestion().unwrap();
    assert!(suggestion.contains("TOML syntax"));
}

#[test]
fn suggestion_json_serialize() {
    // Create a true serialization error using a map with non-string keys
    use std::collections::HashMap;
    let mut map: HashMap<Vec<u8>, i32> = HashMap::new();
    map.insert(vec![1, 2, 3], 42);
    let json_err = serde_json::to_string(&map).unwrap_err();
    let err = SlocGuardError::JsonSerialize(json_err);
    let suggestion = err.suggestion().unwrap();
    assert!(suggestion.contains("non-serializable"));
}

#[test]
fn suggestion_git_error() {
    let err = SlocGuardError::Git("failed to read index".to_string());
    let suggestion = err.suggestion().unwrap();
    assert!(suggestion.contains("git is installed"));
}

#[test]
fn suggestion_git_repo_not_found() {
    let err = SlocGuardError::GitRepoNotFound("/some/path".to_string());
    let suggestion = err.suggestion().unwrap();
    assert!(suggestion.contains("git init"));
}

#[test]
fn suggestion_remote_config_hash_mismatch() {
    let err = SlocGuardError::RemoteConfigHashMismatch {
        url: "https://example.com/config.toml".to_string(),
        expected: "abc123".to_string(),
        actual: "def456".to_string(),
    };
    let suggestion = err.suggestion().unwrap();
    assert!(suggestion.contains("extends_sha256"));
}

#[test]
fn suggestion_io_timeout() {
    let err = SlocGuardError::Io(std::io::Error::new(std::io::ErrorKind::TimedOut, "timeout"));
    let suggestion = err.suggestion().unwrap();
    assert!(suggestion.contains("network connectivity"));
}

#[test]
fn suggestion_io_connection_refused() {
    let err = SlocGuardError::Io(std::io::Error::new(
        std::io::ErrorKind::ConnectionRefused,
        "refused",
    ));
    let suggestion = err.suggestion().unwrap();
    assert!(suggestion.contains("remote server"));
}

#[test]
fn suggestion_io_connection_reset() {
    let err = SlocGuardError::Io(std::io::Error::new(
        std::io::ErrorKind::ConnectionReset,
        "reset",
    ));
    let suggestion = err.suggestion().unwrap();
    assert!(suggestion.contains("remote server"));
}

#[test]
fn suggestion_io_invalid_data() {
    let err = SlocGuardError::Io(std::io::Error::new(
        std::io::ErrorKind::InvalidData,
        "corrupted",
    ));
    let suggestion = err.suggestion().unwrap();
    assert!(suggestion.contains("corrupted"));
}

#[test]
fn suggestion_file_read_invalid_data() {
    let err = SlocGuardError::FileRead {
        path: PathBuf::from("corrupted.rs"),
        source: std::io::Error::new(std::io::ErrorKind::InvalidData, "corrupted"),
    };
    let suggestion = err.suggestion().unwrap();
    assert!(suggestion.contains("corrupted"));
}
