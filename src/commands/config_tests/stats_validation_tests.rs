//! Tests for stats.report config semantic validation.

use crate::config::Config;

use super::super::*;

#[test]
fn exclude_valid_values() {
    let mut config = Config::default();
    config.stats.report.exclude = vec![
        "summary".to_string(),
        "files".to_string(),
        "breakdown".to_string(),
        "trend".to_string(),
    ];

    let result = validate_config_semantics(&config);
    assert!(result.is_ok());
}

#[test]
fn exclude_case_insensitive() {
    let mut config = Config::default();
    config.stats.report.exclude = vec!["SUMMARY".to_string(), "Trend".to_string()];

    let result = validate_config_semantics(&config);
    assert!(result.is_ok());
}

#[test]
fn exclude_invalid_section() {
    let mut config = Config::default();
    config.stats.report.exclude = vec!["invalid_section".to_string()];

    let result = validate_config_semantics(&config);
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("stats.report.exclude"));
    assert!(err_msg.contains("invalid_section"));
    assert!(err_msg.contains("summary, files, breakdown, trend"));
}

#[test]
fn breakdown_by_valid_lang() {
    let mut config = Config::default();
    config.stats.report.breakdown_by = Some("lang".to_string());

    let result = validate_config_semantics(&config);
    assert!(result.is_ok());
}

#[test]
fn breakdown_by_valid_language() {
    let mut config = Config::default();
    config.stats.report.breakdown_by = Some("language".to_string());

    let result = validate_config_semantics(&config);
    assert!(result.is_ok());
}

#[test]
fn breakdown_by_valid_dir() {
    let mut config = Config::default();
    config.stats.report.breakdown_by = Some("dir".to_string());

    let result = validate_config_semantics(&config);
    assert!(result.is_ok());
}

#[test]
fn breakdown_by_valid_directory() {
    let mut config = Config::default();
    config.stats.report.breakdown_by = Some("directory".to_string());

    let result = validate_config_semantics(&config);
    assert!(result.is_ok());
}

#[test]
fn breakdown_by_case_insensitive() {
    let mut config = Config::default();
    config.stats.report.breakdown_by = Some("LANG".to_string());

    let result = validate_config_semantics(&config);
    assert!(result.is_ok());
}

#[test]
fn breakdown_by_invalid() {
    let mut config = Config::default();
    config.stats.report.breakdown_by = Some("invalid".to_string());

    let result = validate_config_semantics(&config);
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("stats.report.breakdown_by"));
    assert!(err_msg.contains("invalid"));
}

#[test]
fn trend_since_valid() {
    let mut config = Config::default();
    config.stats.report.trend_since = Some("7d".to_string());

    let result = validate_config_semantics(&config);
    assert!(result.is_ok());
}

#[test]
fn trend_since_valid_week() {
    let mut config = Config::default();
    config.stats.report.trend_since = Some("1w".to_string());

    let result = validate_config_semantics(&config);
    assert!(result.is_ok());
}

#[test]
fn trend_since_valid_hours() {
    let mut config = Config::default();
    config.stats.report.trend_since = Some("12h".to_string());

    let result = validate_config_semantics(&config);
    assert!(result.is_ok());
}

#[test]
fn trend_since_invalid() {
    let mut config = Config::default();
    config.stats.report.trend_since = Some("invalid".to_string());

    let result = validate_config_semantics(&config);
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("stats.report.trend_since"));
    assert!(err_msg.contains("invalid"));
}

#[test]
fn trend_since_missing_unit() {
    let mut config = Config::default();
    config.stats.report.trend_since = Some("30".to_string());

    let result = validate_config_semantics(&config);
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("stats.report.trend_since"));
}
