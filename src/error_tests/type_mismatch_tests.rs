use crate::error::{ConfigSource, SlocGuardError};

#[test]
fn display() {
    let err = SlocGuardError::TypeMismatch {
        field: "content.max_lines".to_string(),
        expected: "integer".to_string(),
        actual: "string".to_string(),
        origin: Some(ConfigSource::file("config.toml")),
    };
    let msg = err.to_string();
    assert!(msg.contains("content.max_lines"));
    assert!(msg.contains("integer"));
    assert!(msg.contains("string"));
    assert!(msg.contains("config.toml"));
}

#[test]
fn display_without_origin() {
    let err = SlocGuardError::TypeMismatch {
        field: "content.max_lines".to_string(),
        expected: "integer".to_string(),
        actual: "string".to_string(),
        origin: None,
    };
    let msg = err.to_string();
    assert!(msg.contains("content.max_lines"));
    assert!(!msg.contains("(in"));
}

#[test]
fn error_type() {
    let err = SlocGuardError::TypeMismatch {
        field: "field".to_string(),
        expected: "type".to_string(),
        actual: "other".to_string(),
        origin: None,
    };
    assert_eq!(err.error_type(), "Config");
}

#[test]
fn message() {
    let err = SlocGuardError::TypeMismatch {
        field: "content.warn_threshold".to_string(),
        expected: "float".to_string(),
        actual: "boolean".to_string(),
        origin: None,
    };
    let msg = err.message();
    assert!(msg.contains("content.warn_threshold"));
    assert!(msg.contains("float"));
    assert!(msg.contains("boolean"));
}

#[test]
fn detail_with_origin() {
    let err = SlocGuardError::TypeMismatch {
        field: "field".to_string(),
        expected: "type".to_string(),
        actual: "other".to_string(),
        origin: Some(ConfigSource::preset("rust-strict")),
    };
    let detail = err.detail().unwrap();
    assert!(detail.contains("preset:rust-strict"));
}

#[test]
fn suggestion() {
    let err = SlocGuardError::TypeMismatch {
        field: "field".to_string(),
        expected: "type".to_string(),
        actual: "other".to_string(),
        origin: None,
    };
    let suggestion = err.suggestion().unwrap();
    assert!(suggestion.contains("type") || suggestion.contains("documentation"));
}
