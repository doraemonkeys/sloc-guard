//! Tests for content config semantic validation (`warn_threshold`, `warn_at`, glob patterns).

use crate::config::{Config, ContentConfig, ContentRule};

use super::super::*;

#[test]
fn warn_threshold_too_high() {
    let mut config = Config::default();
    config.content.warn_threshold = 1.5;

    let result = validate_config_semantics(&config);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("warn_threshold"));
}

#[test]
fn warn_threshold_negative() {
    let mut config = Config::default();
    config.content.warn_threshold = -0.1;

    let result = validate_config_semantics(&config);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("warn_threshold"));
}

#[test]
fn warn_threshold_valid_boundaries() {
    let mut config = Config::default();

    config.content.warn_threshold = 0.0;
    assert!(validate_config_semantics(&config).is_ok());

    config.content.warn_threshold = 1.0;
    assert!(validate_config_semantics(&config).is_ok());
}

#[test]
fn invalid_scanner_exclude_pattern() {
    let mut config = Config::default();
    config.scanner.exclude = vec!["[invalid".to_string()];

    let result = validate_config_semantics(&config);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Invalid glob"));
}

#[test]
fn invalid_content_exclude_pattern() {
    let mut config = Config::default();
    config.content.exclude = vec!["[invalid".to_string()];

    let result = validate_config_semantics(&config);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Invalid glob"));
}

#[test]
fn warn_at_greater_than_max_lines() {
    let config = Config {
        content: ContentConfig {
            max_lines: 500,
            warn_at: Some(600), // warn_at > max_lines is invalid
            ..Default::default()
        },
        ..Default::default()
    };

    let result = validate_config_semantics(&config);
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("content.warn_at"));
    assert!(err_msg.contains("must be less than"));
}

#[test]
fn warn_at_equal_to_max_lines() {
    let config = Config {
        content: ContentConfig {
            max_lines: 500,
            warn_at: Some(500), // warn_at == max_lines is invalid
            ..Default::default()
        },
        ..Default::default()
    };

    let result = validate_config_semantics(&config);
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("content.warn_at"));
    assert!(err_msg.contains("must be less than"));
}

#[test]
fn warn_at_less_than_max_lines_is_valid() {
    let config = Config {
        content: ContentConfig {
            max_lines: 500,
            warn_at: Some(400), // warn_at < max_lines is valid
            ..Default::default()
        },
        ..Default::default()
    };

    let result = validate_config_semantics(&config);
    assert!(result.is_ok());
}

#[test]
fn rule_warn_at_greater_than_rule_max_lines() {
    let config = Config {
        content: ContentConfig {
            rules: vec![ContentRule {
                pattern: "**/*.rs".to_string(),
                max_lines: 300,
                warn_at: Some(400), // warn_at > max_lines is invalid
                warn_threshold: None,
                skip_comments: None,
                skip_blank: None,
                reason: None,
                expires: None,
            }],
            ..Default::default()
        },
        ..Default::default()
    };

    let result = validate_config_semantics(&config);
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("content.rules[0].warn_at"));
    assert!(err_msg.contains("must be less than"));
}

#[test]
fn rule_warn_at_less_than_rule_max_lines_is_valid() {
    let config = Config {
        content: ContentConfig {
            rules: vec![ContentRule {
                pattern: "**/*.rs".to_string(),
                max_lines: 300,
                warn_at: Some(250), // warn_at < max_lines is valid
                warn_threshold: None,
                skip_comments: None,
                skip_blank: None,
                reason: None,
                expires: None,
            }],
            ..Default::default()
        },
        ..Default::default()
    };

    let result = validate_config_semantics(&config);
    assert!(result.is_ok());
}
