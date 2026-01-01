//! Tests for error handling in `ThresholdChecker::new()`.
//!
//! Validates fail-fast behavior for invalid glob patterns per 23.1 spec.

use crate::config::{Config, ContentRule};
use crate::error::SlocGuardError;

use super::ThresholdChecker;

#[test]
fn invalid_content_exclude_pattern_returns_error() {
    let mut config = Config::default();
    // Invalid glob pattern: unclosed bracket
    config.content.exclude = vec!["**/*.rs[".to_string()];

    let result = ThresholdChecker::new(config);

    let Err(err) = result else {
        panic!("Expected error for invalid pattern");
    };
    let SlocGuardError::InvalidPattern { pattern, .. } = &err else {
        panic!("Expected InvalidPattern error, got: {err}");
    };
    assert_eq!(pattern, "**/*.rs[");
}

#[test]
fn invalid_content_rule_pattern_returns_error() {
    let mut config = Config::default();
    config.content.rules.push(ContentRule {
        // Invalid glob pattern: unclosed bracket
        pattern: "src/[invalid".to_string(),
        max_lines: 500,
        warn_threshold: None,
        warn_at: None,
        skip_comments: None,
        skip_blank: None,
        reason: None,
        expires: None,
    });

    let result = ThresholdChecker::new(config);

    let Err(err) = result else {
        panic!("Expected error for invalid pattern");
    };
    let SlocGuardError::InvalidPattern { pattern, .. } = &err else {
        panic!("Expected InvalidPattern error, got: {err}");
    };
    assert_eq!(pattern, "src/[invalid");
}

#[test]
fn first_invalid_pattern_fails_immediately() {
    let mut config = Config::default();
    // First pattern is invalid
    config.content.rules.push(ContentRule {
        pattern: "[invalid1".to_string(),
        max_lines: 500,
        warn_threshold: None,
        warn_at: None,
        skip_comments: None,
        skip_blank: None,
        reason: None,
        expires: None,
    });
    // Second pattern is also invalid (should not be reached)
    config.content.rules.push(ContentRule {
        pattern: "[invalid2".to_string(),
        max_lines: 600,
        warn_threshold: None,
        warn_at: None,
        skip_comments: None,
        skip_blank: None,
        reason: None,
        expires: None,
    });

    let result = ThresholdChecker::new(config);

    // Should fail on first invalid pattern (fail-fast)
    let Err(err) = result else {
        panic!("Expected error for invalid pattern");
    };
    let SlocGuardError::InvalidPattern { pattern, .. } = &err else {
        panic!("Expected InvalidPattern error, got: {err}");
    };
    assert_eq!(pattern, "[invalid1");
}

#[test]
fn valid_patterns_succeed() {
    let mut config = Config::default();
    config.content.exclude = vec!["**/*.generated.ts".to_string(), "**/vendor/**".to_string()];
    config.content.rules.push(ContentRule {
        pattern: "src/**/*.rs".to_string(),
        max_lines: 500,
        warn_threshold: None,
        warn_at: None,
        skip_comments: None,
        skip_blank: None,
        reason: None,
        expires: None,
    });

    let result = ThresholdChecker::new(config);

    assert!(result.is_ok());
}

#[test]
fn error_message_includes_pattern() {
    let mut config = Config::default();
    config.content.exclude = vec!["[bad-pattern".to_string()];

    let Err(err) = ThresholdChecker::new(config) else {
        panic!("Expected error for invalid pattern");
    };
    let message = err.message();
    assert!(
        message.contains("[bad-pattern"),
        "Error message should include the invalid pattern"
    );
}

#[test]
fn error_has_suggestion() {
    let mut config = Config::default();
    config.content.exclude = vec!["[bad-pattern".to_string()];

    let Err(err) = ThresholdChecker::new(config) else {
        panic!("Expected error for invalid pattern");
    };
    assert!(err.suggestion().is_some(), "Error should have a suggestion");
    assert!(
        err.suggestion().unwrap().contains("glob pattern syntax"),
        "Suggestion should mention glob patterns"
    );
}
