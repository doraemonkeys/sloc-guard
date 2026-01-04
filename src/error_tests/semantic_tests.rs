use crate::error::{ConfigSource, SlocGuardError};

// =============================================================================
// Semantic Error Tests
// =============================================================================

#[test]
fn semantic_display() {
    let err = SlocGuardError::Semantic {
        field: "content.warn_threshold".to_string(),
        message: "must be between 0.0 and 1.0".to_string(),
        origin: Some(ConfigSource::remote("https://example.com/config.toml")),
        suggestion: Some("Use a value like 0.8 for 80% warning threshold".to_string()),
    };
    let msg = err.to_string();
    assert!(msg.contains("content.warn_threshold"));
    assert!(msg.contains("must be between 0.0 and 1.0"));
    assert!(msg.contains("https://example.com/config.toml"));
}

#[test]
fn semantic_display_without_origin() {
    let err = SlocGuardError::Semantic {
        field: "field".to_string(),
        message: "invalid value".to_string(),
        origin: None,
        suggestion: None,
    };
    let msg = err.to_string();
    assert!(!msg.contains("(in"));
}

#[test]
fn semantic_error_type() {
    let err = SlocGuardError::Semantic {
        field: "field".to_string(),
        message: "error".to_string(),
        origin: None,
        suggestion: None,
    };
    assert_eq!(err.error_type(), "Config");
}

#[test]
fn semantic_message() {
    let err = SlocGuardError::Semantic {
        field: "structure.max_depth".to_string(),
        message: "cannot be negative".to_string(),
        origin: None,
        suggestion: None,
    };
    let msg = err.message();
    assert!(msg.contains("structure.max_depth"));
    assert!(msg.contains("cannot be negative"));
}

#[test]
fn semantic_suggestion_from_field() {
    let err = SlocGuardError::Semantic {
        field: "field".to_string(),
        message: "error".to_string(),
        origin: None,
        suggestion: Some("Try this instead".to_string()),
    };
    let suggestion = err.suggestion().unwrap();
    assert_eq!(suggestion, "Try this instead");
}

#[test]
fn semantic_suggestion_none() {
    let err = SlocGuardError::Semantic {
        field: "field".to_string(),
        message: "error".to_string(),
        origin: None,
        suggestion: None,
    };
    assert!(err.suggestion().is_none());
}

// =============================================================================
// Basic Config Error Tests
// =============================================================================

#[test]
fn config_display() {
    let err = SlocGuardError::Config("invalid threshold".to_string());
    assert_eq!(err.to_string(), "Configuration error: invalid threshold");
}

#[test]
fn config_error_type() {
    assert_eq!(
        SlocGuardError::Config("test".to_string()).error_type(),
        "Config"
    );
}

#[test]
fn config_message() {
    let err = SlocGuardError::Config("invalid config".to_string());
    assert_eq!(err.message(), "invalid config");
}

#[test]
fn config_detail_none() {
    let err = SlocGuardError::Config("test".to_string());
    assert!(err.detail().is_none());
}

#[test]
fn config_suggestion() {
    let err = SlocGuardError::Config("invalid threshold".to_string());
    let suggestion = err.suggestion().unwrap();
    assert!(suggestion.contains("config file format"));
}
