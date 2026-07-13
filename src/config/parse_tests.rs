//! Tests for per-source config parsing and schema enforcement.

use crate::error::{ConfigSource, SlocGuardError};

use super::{finalize_merged_config, parse_source, validate_version};

fn file_origin() -> ConfigSource {
    ConfigSource::file("/project/.sloc-guard.toml")
}

#[test]
fn parse_source_returns_value_and_config() {
    let content = "version = \"2\"\n\n[content]\nmax_lines = 300\n";

    let parsed = parse_source(content, &file_origin()).unwrap();

    assert_eq!(parsed.config.content.max_lines, 300);
    assert!(parsed.value.get("content").is_some());
}

#[test]
fn parse_source_syntax_error_carries_origin() {
    let err = parse_source("bad = [", &file_origin()).unwrap_err();

    match err {
        SlocGuardError::Syntax { origin, .. } => assert_eq!(origin, Some(file_origin())),
        other => panic!("Expected Syntax error, got: {other:?}"),
    }
}

#[test]
fn parse_source_unknown_field_reports_origin_and_line() {
    let content = "version = \"2\"\n\n[content]\nmax_linez = 300\n";

    let err = parse_source(content, &file_origin()).unwrap_err();

    match err {
        SlocGuardError::Syntax {
            origin,
            line,
            message,
            ..
        } => {
            assert_eq!(origin, Some(file_origin()));
            assert_eq!(line, 4, "expected error at the unknown key's line");
            assert!(message.contains("max_linez"), "got: {message}");
        }
        other => panic!("Expected Syntax error, got: {other:?}"),
    }
}

#[test]
fn parse_source_strips_reset_markers_from_config_only() {
    let content = "[scanner]\nexclude = [\"$reset\", \"build/**\"]\n";

    let parsed = parse_source(content, &file_origin()).unwrap();

    assert_eq!(parsed.config.scanner.exclude, vec!["build/**"]);
    // The raw value keeps the marker: it is merge-pipeline input.
    let raw = parsed.value["scanner"]["exclude"].as_array().unwrap();
    assert_eq!(raw.len(), 2);
}

#[test]
fn parse_source_unknown_field_detected_alongside_reset_markers() {
    let content = "[scanner]\nexclude = [\"$reset\"]\nbogus = true\n";

    let err = parse_source(content, &file_origin()).unwrap_err();

    match err {
        SlocGuardError::Syntax {
            origin, message, ..
        } => {
            assert_eq!(origin, Some(file_origin()));
            assert!(message.contains("bogus"), "got: {message}");
        }
        other => panic!("Expected Syntax error, got: {other:?}"),
    }
}

#[test]
fn parse_source_rejects_misplaced_reset_marker() {
    let content = "[scanner]\nexclude = [\"build/**\", \"$reset\"]\n";

    let err = parse_source(content, &file_origin()).unwrap_err();

    assert!(matches!(
        err,
        SlocGuardError::Config(msg) if msg.contains("$reset") && msg.contains("first element")
    ));
}

#[test]
fn finalize_merged_config_parses_and_accepts_current_version() {
    let value: toml::Value =
        toml::from_str("version = \"2\"\n\n[content]\nmax_lines = 100\n").unwrap();

    let config = finalize_merged_config(value, &file_origin()).unwrap();

    assert_eq!(config.content.max_lines, 100);
}

#[test]
fn finalize_merged_config_rejects_unsupported_version() {
    let value: toml::Value = toml::from_str("version = \"99\"\n").unwrap();

    let err = finalize_merged_config(value, &file_origin()).unwrap_err();

    assert!(matches!(
        err,
        SlocGuardError::Config(msg) if msg.contains("Unsupported config version")
    ));
}

#[test]
fn validate_version_accepts_missing_and_current() {
    let config = crate::config::Config::default();
    assert!(validate_version(&config).is_ok());

    let config = crate::config::Config {
        version: Some("2".to_string()),
        ..Default::default()
    };
    assert!(validate_version(&config).is_ok());
}

#[test]
fn validate_version_rejects_other_versions() {
    let config = crate::config::Config {
        version: Some("1".to_string()),
        ..Default::default()
    };

    let err = validate_version(&config).unwrap_err();

    assert!(err.to_string().contains("Unsupported config version"));
}
