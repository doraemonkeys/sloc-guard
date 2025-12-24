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
