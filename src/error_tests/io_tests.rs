use std::path::PathBuf;

use crate::error::SlocGuardError;

// =============================================================================
// FileAccess Error Tests
// =============================================================================

#[test]
fn file_access_display() {
    let err = SlocGuardError::FileAccess {
        path: PathBuf::from("test.rs"),
        source: std::io::Error::new(std::io::ErrorKind::NotFound, "file not found"),
    };
    assert!(err.to_string().contains("test.rs"));
}

#[test]
fn file_access_error_type() {
    assert_eq!(
        SlocGuardError::FileAccess {
            path: PathBuf::from("test.rs"),
            source: std::io::Error::new(std::io::ErrorKind::NotFound, "not found"),
        }
        .error_type(),
        "FileAccess"
    );
}

#[test]
fn file_access_message() {
    let err = SlocGuardError::FileAccess {
        path: PathBuf::from("test.rs"),
        source: std::io::Error::new(std::io::ErrorKind::NotFound, "not found"),
    };
    let message = err.message();
    assert!(message.contains("test.rs"));
    // Error kind format varies by platform (NotFound vs "not found")
    // Just verify the message is longer than just the path
    assert!(message.len() > "test.rs".len());
}

#[test]
fn file_access_detail() {
    let err = SlocGuardError::FileAccess {
        path: PathBuf::from("test.rs"),
        source: std::io::Error::new(std::io::ErrorKind::NotFound, "file not found"),
    };
    let detail = err.detail().unwrap();
    assert!(detail.contains("file not found"));
    // Error kind format varies by platform, just check it contains some error info
    assert!(!detail.is_empty());
}

#[test]
fn file_access_suggestion_not_found() {
    let err = SlocGuardError::FileAccess {
        path: PathBuf::from("missing.rs"),
        source: std::io::Error::new(std::io::ErrorKind::NotFound, "not found"),
    };
    let suggestion = err.suggestion().unwrap();
    assert!(suggestion.contains("file path exists"));
}

#[test]
fn file_access_suggestion_permission_denied() {
    let err = SlocGuardError::FileAccess {
        path: PathBuf::from("protected.rs"),
        source: std::io::Error::new(std::io::ErrorKind::PermissionDenied, "access denied"),
    };
    let suggestion = err.suggestion().unwrap();
    assert!(suggestion.contains("permissions"));
}

#[test]
fn file_access_suggestion_other_error_has_none() {
    let err = SlocGuardError::FileAccess {
        path: PathBuf::from("unknown.rs"),
        source: std::io::Error::other("unknown error"),
    };
    // Other IO errors may not have specific suggestions
    assert!(err.suggestion().is_none());
}

#[test]
fn file_access_suggestion_invalid_data() {
    let err = SlocGuardError::FileAccess {
        path: PathBuf::from("corrupted.rs"),
        source: std::io::Error::new(std::io::ErrorKind::InvalidData, "corrupted"),
    };
    let suggestion = err.suggestion().unwrap();
    assert!(suggestion.contains("corrupted"));
}

// =============================================================================
// IO Error Tests
// =============================================================================

#[test]
fn io_error_type() {
    assert_eq!(
        SlocGuardError::from(std::io::Error::other("test")).error_type(),
        "IO"
    );
}

#[test]
fn io_suggestion_not_found() {
    let err = SlocGuardError::from(std::io::Error::new(
        std::io::ErrorKind::NotFound,
        "not found",
    ));
    let suggestion = err.suggestion().unwrap();
    assert!(suggestion.contains("file path exists"));
}

#[test]
fn io_suggestion_permission_denied() {
    let err = SlocGuardError::from(std::io::Error::new(
        std::io::ErrorKind::PermissionDenied,
        "denied",
    ));
    let suggestion = err.suggestion().unwrap();
    assert!(suggestion.contains("permissions"));
}

#[test]
fn io_suggestion_other_has_none() {
    let err = SlocGuardError::from(std::io::Error::other("custom error"));
    assert!(err.suggestion().is_none());
}

#[test]
fn io_suggestion_timeout() {
    let err = SlocGuardError::from(std::io::Error::new(std::io::ErrorKind::TimedOut, "timeout"));
    let suggestion = err.suggestion().unwrap();
    assert!(suggestion.contains("network connectivity"));
}

#[test]
fn io_suggestion_connection_refused() {
    let err = SlocGuardError::from(std::io::Error::new(
        std::io::ErrorKind::ConnectionRefused,
        "refused",
    ));
    let suggestion = err.suggestion().unwrap();
    assert!(suggestion.contains("remote server"));
}

#[test]
fn io_suggestion_connection_reset() {
    let err = SlocGuardError::from(std::io::Error::new(
        std::io::ErrorKind::ConnectionReset,
        "reset",
    ));
    let suggestion = err.suggestion().unwrap();
    assert!(suggestion.contains("remote server"));
}

#[test]
fn io_suggestion_invalid_data() {
    let err = SlocGuardError::from(std::io::Error::new(
        std::io::ErrorKind::InvalidData,
        "corrupted",
    ));
    let suggestion = err.suggestion().unwrap();
    assert!(suggestion.contains("corrupted"));
}

// =============================================================================
// IO Context Tests (io_with_path, io_with_context)
// =============================================================================

#[test]
fn io_with_path_includes_path_in_message() {
    let err = SlocGuardError::io_with_path(
        std::io::Error::new(std::io::ErrorKind::NotFound, "not found"),
        PathBuf::from("config.toml"),
    );
    let message = err.message();
    assert!(message.contains("config.toml"));
    assert!(message.contains("not found"));
}

#[test]
fn io_with_context_includes_path_and_operation() {
    let err = SlocGuardError::io_with_context(
        std::io::Error::new(std::io::ErrorKind::PermissionDenied, "denied"),
        PathBuf::from("secret.key"),
        "reading",
    );
    let message = err.message();
    assert!(message.contains("secret.key"));
    assert!(message.contains("reading"));
    assert!(message.contains("denied"));
}

#[test]
fn io_without_context_shows_error_only() {
    let err = SlocGuardError::from(std::io::Error::other("generic error"));
    let message = err.message();
    assert_eq!(message, "generic error");
}

#[test]
fn io_detail_includes_path_operation_and_kind() {
    let err = SlocGuardError::io_with_context(
        std::io::Error::new(std::io::ErrorKind::NotFound, "file missing"),
        PathBuf::from("data.json"),
        "opening",
    );
    let detail = err.detail().unwrap();
    assert!(detail.contains("data.json"));
    assert!(detail.contains("opening"));
    assert!(detail.contains("file missing"));
    // Error kind is included but format varies by platform
    assert!(detail.len() > "opening 'data.json': file missing".len());
}

#[test]
fn io_detail_without_context_shows_kind() {
    let err = SlocGuardError::from(std::io::Error::new(
        std::io::ErrorKind::PermissionDenied,
        "access denied",
    ));
    let detail = err.detail().unwrap();
    assert!(detail.contains("access denied"));
    // Error kind is included but format varies by platform
    assert!(detail.len() > "access denied".len());
}

#[test]
fn io_display_with_full_context() {
    let err = SlocGuardError::io_with_context(
        std::io::Error::new(std::io::ErrorKind::NotFound, "missing"),
        PathBuf::from("cache.json"),
        "writing",
    );
    let display = err.to_string();
    assert!(display.contains("IO error"));
    assert!(display.contains("cache.json"));
    assert!(display.contains("writing"));
}

#[test]
fn io_display_with_path_only() {
    let err = SlocGuardError::io_with_path(
        std::io::Error::new(std::io::ErrorKind::NotFound, "missing"),
        PathBuf::from("data.txt"),
    );
    let display = err.to_string();
    assert!(display.contains("data.txt"));
    assert!(display.contains("missing"));
}

#[test]
fn io_from_conversion_preserves_error() {
    let io_err = std::io::Error::new(std::io::ErrorKind::InvalidInput, "bad input");
    let err: SlocGuardError = io_err.into();
    assert_eq!(err.error_type(), "IO");
    assert!(err.message().contains("bad input"));
}

// =============================================================================
// JsonSerialize Tests
// =============================================================================

#[test]
fn json_serialize_suggestion() {
    // Create a true serialization error using a map with non-string keys
    use std::collections::HashMap;
    let mut map: HashMap<Vec<u8>, i32> = HashMap::new();
    map.insert(vec![1, 2, 3], 42);
    let json_err = serde_json::to_string(&map).unwrap_err();
    let err = SlocGuardError::JsonSerialize(json_err);
    let suggestion = err.suggestion().unwrap();
    assert!(suggestion.contains("non-serializable"));
}

