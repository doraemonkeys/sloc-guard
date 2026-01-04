//! Tests for text and JSON output formatting in explain command.
//!
//! Covers: content/structure explanation formatting, `warn_at` source display,
//! excluded file output, and edge cases like unlimited/none limits.

use std::path::PathBuf;

use crate::cli::ExplainFormat;
use crate::config::{Config, ContentConfig, ContentRule, StructureConfig, StructureRule};

use super::super::{format_content_explanation, format_limit, format_structure_explanation};

// ============================================================================
// Content formatting tests
// ============================================================================

#[test]
fn format_content_text_output_contains_expected_sections() {
    let config = Config {
        content: ContentConfig {
            max_lines: 500,
            ..Default::default()
        },
        ..Default::default()
    };

    let checker = crate::checker::ThresholdChecker::new(config).unwrap();
    let explanation = checker.explain(&PathBuf::from("src/main.rs"));
    let output = format_content_explanation(&explanation, ExplainFormat::Text).unwrap();

    assert!(output.contains("Path:"));
    assert!(output.contains("Content Rules (SLOC Limits):"));
    assert!(output.contains("Matched:"));
    assert!(output.contains("Limit:"));
    assert!(output.contains("Warn at:"));
    assert!(output.contains("Skip:"));
    assert!(output.contains("Rule Chain"));
}

#[test]
fn format_content_json_output_is_valid() {
    let config = Config {
        content: ContentConfig {
            max_lines: 500,
            ..Default::default()
        },
        ..Default::default()
    };

    let checker = crate::checker::ThresholdChecker::new(config).unwrap();
    let explanation = checker.explain(&PathBuf::from("src/main.rs"));
    let output = format_content_explanation(&explanation, ExplainFormat::Json).unwrap();

    // Verify it's valid JSON
    let parsed: serde_json::Value = serde_json::from_str(&output).expect("Invalid JSON output");
    assert!(parsed.get("path").is_some());
    assert!(parsed.get("matched_rule").is_some());
    assert!(parsed.get("effective_limit").is_some());
}

#[test]
fn format_content_with_rule_shows_pattern() {
    let config = Config {
        content: ContentConfig {
            max_lines: 500,
            rules: vec![ContentRule {
                pattern: "**/*.rs".to_string(),
                max_lines: 300,
                warn_threshold: None,
                warn_at: None,
                skip_comments: None,
                skip_blank: None,
                reason: None,
                expires: None,
            }],
            ..Default::default()
        },
        ..Default::default()
    };

    let checker = crate::checker::ThresholdChecker::new(config).unwrap();
    let explanation = checker.explain(&PathBuf::from("src/main.rs"));
    let output = format_content_explanation(&explanation, ExplainFormat::Text).unwrap();

    assert!(output.contains("[[content.rules]]"));
    assert!(output.contains("**/*.rs"));
}

#[test]
fn format_content_with_rule_reason_shows_reason() {
    let config = Config {
        content: ContentConfig {
            max_lines: 500,
            rules: vec![ContentRule {
                pattern: "legacy/**".to_string(),
                max_lines: 2000,
                warn_threshold: None,
                warn_at: None,
                skip_comments: None,
                skip_blank: None,
                reason: Some("Legacy code".to_string()),
                expires: None,
            }],
            ..Default::default()
        },
        ..Default::default()
    };

    let checker = crate::checker::ThresholdChecker::new(config).unwrap();
    let explanation = checker.explain(&PathBuf::from("legacy/old.rs"));
    let output = format_content_explanation(&explanation, ExplainFormat::Text).unwrap();

    assert!(output.contains("[[content.rules]]"));
    assert!(output.contains("Legacy code"));
}

#[test]
fn format_content_excluded_file_text() {
    let config = Config {
        content: ContentConfig {
            max_lines: 500,
            exclude: vec!["**/*.gen.rs".to_string()],
            ..Default::default()
        },
        ..Default::default()
    };

    let checker = crate::checker::ThresholdChecker::new(config).unwrap();
    let explanation = checker.explain(&PathBuf::from("src/api.gen.rs"));
    let output = format_content_explanation(&explanation, ExplainFormat::Text).unwrap();

    assert!(output.contains("EXCLUDED"));
    assert!(output.contains("**/*.gen.rs"));
    assert!(output.contains("excluded from SLOC counting"));
}

#[test]
fn format_content_excluded_file_json() {
    let config = Config {
        content: ContentConfig {
            max_lines: 500,
            exclude: vec!["**/*.generated.ts".to_string()],
            ..Default::default()
        },
        ..Default::default()
    };

    let checker = crate::checker::ThresholdChecker::new(config).unwrap();
    let explanation = checker.explain(&PathBuf::from("src/models.generated.ts"));
    let output = format_content_explanation(&explanation, ExplainFormat::Json).unwrap();

    // Verify it's valid JSON
    let parsed: serde_json::Value = serde_json::from_str(&output).expect("Invalid JSON output");
    assert!(parsed.get("is_excluded").is_some());
    assert_eq!(parsed["is_excluded"].as_bool(), Some(true));
    assert!(parsed.get("matched_rule").is_some());
}

// ============================================================================
// Warn-at source formatting tests
// ============================================================================

#[test]
fn format_content_text_shows_global_percentage_source() {
    let config = Config {
        content: ContentConfig {
            max_lines: 500,
            warn_threshold: 0.9,
            ..Default::default()
        },
        ..Default::default()
    };

    let checker = crate::checker::ThresholdChecker::new(config)
        .unwrap()
        .with_warning_threshold(0.9);
    let explanation = checker.explain(&PathBuf::from("src/main.rs"));
    let output = format_content_explanation(&explanation, ExplainFormat::Text).unwrap();

    // Should show percentage and global source
    assert!(output.contains("(from [content], 90%)"));
}

#[test]
fn format_content_text_shows_global_absolute_source() {
    let config = Config {
        content: ContentConfig {
            max_lines: 500,
            warn_at: Some(400),
            ..Default::default()
        },
        ..Default::default()
    };

    let checker = crate::checker::ThresholdChecker::new(config).unwrap();
    let explanation = checker.explain(&PathBuf::from("src/main.rs"));
    let output = format_content_explanation(&explanation, ExplainFormat::Text).unwrap();

    // Should show global source with absolute
    assert!(output.contains("(from [content], absolute)"));
    assert!(output.contains("400 lines"));
}

#[test]
fn format_content_text_shows_rule_absolute_source_with_index() {
    let config = Config {
        content: ContentConfig {
            max_lines: 500,
            rules: vec![ContentRule {
                pattern: "**/*.rs".to_string(),
                max_lines: 300,
                warn_at: Some(250),
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

    let checker = crate::checker::ThresholdChecker::new(config).unwrap();
    let explanation = checker.explain(&PathBuf::from("src/main.rs"));
    let output = format_content_explanation(&explanation, ExplainFormat::Text).unwrap();

    // Should show rule source with index and absolute
    assert!(output.contains("(from content.rules[0], absolute)"));
    assert!(output.contains("250 lines"));
}

#[test]
fn format_content_text_shows_rule_percentage_source_with_index() {
    let config = Config {
        content: ContentConfig {
            max_lines: 500,
            rules: vec![ContentRule {
                pattern: "**/*.rs".to_string(),
                max_lines: 300,
                warn_at: None,
                warn_threshold: Some(0.8),
                skip_comments: None,
                skip_blank: None,
                reason: None,
                expires: None,
            }],
            ..Default::default()
        },
        ..Default::default()
    };

    let checker = crate::checker::ThresholdChecker::new(config).unwrap();
    let explanation = checker.explain(&PathBuf::from("src/main.rs"));
    let output = format_content_explanation(&explanation, ExplainFormat::Text).unwrap();

    // Should show rule source with index and percentage (300 * 0.8 = 240)
    assert!(output.contains("(from content.rules[0], 80%)"));
    assert!(output.contains("240 lines"));
}

// ============================================================================
// Structure formatting tests
// ============================================================================

#[test]
fn format_structure_text_output_contains_expected_sections() {
    let config = StructureConfig {
        max_files: Some(10),
        max_dirs: Some(5),
        ..Default::default()
    };

    let checker = crate::checker::StructureChecker::new(&config).unwrap();
    let explanation = checker.explain(&PathBuf::from("src"));
    let output = format_structure_explanation(&explanation, ExplainFormat::Text).unwrap();

    assert!(output.contains("Path:"));
    assert!(output.contains("Structure Rules (Directory Limits):"));
    assert!(output.contains("Matched:"));
    assert!(output.contains("Limits:"));
    assert!(output.contains("Warn at:"));
    assert!(output.contains("Rule Chain"));
}

#[test]
fn format_structure_json_output_is_valid() {
    let config = StructureConfig {
        max_files: Some(10),
        max_dirs: Some(5),
        ..Default::default()
    };

    let checker = crate::checker::StructureChecker::new(&config).unwrap();
    let explanation = checker.explain(&PathBuf::from("src"));
    let output = format_structure_explanation(&explanation, ExplainFormat::Json).unwrap();

    // Verify it's valid JSON
    let parsed: serde_json::Value = serde_json::from_str(&output).expect("Invalid JSON output");
    assert!(parsed.get("path").is_some());
    assert!(parsed.get("matched_rule").is_some());
    assert!(parsed.get("effective_max_files").is_some());
}

#[test]
fn format_structure_with_rule_reason_shows_reason() {
    let config = StructureConfig {
        max_files: Some(10),
        max_dirs: Some(5),
        rules: vec![StructureRule {
            scope: "legacy/*".to_string(),
            max_files: Some(100),
            reason: Some("Legacy directory needs more files".to_string()),
            ..Default::default()
        }],
        ..Default::default()
    };

    let checker = crate::checker::StructureChecker::new(&config).unwrap();
    let explanation = checker.explain(&PathBuf::from("legacy/module"));
    let output = format_structure_explanation(&explanation, ExplainFormat::Text).unwrap();

    assert!(output.contains("Reason:"));
    assert!(output.contains("Legacy directory needs more files"));
}

#[test]
fn format_structure_with_unlimited_shows_unlimited() {
    let config = StructureConfig {
        max_files: Some(-1),
        max_dirs: Some(5),
        ..Default::default()
    };

    let checker = crate::checker::StructureChecker::new(&config).unwrap();
    let explanation = checker.explain(&PathBuf::from("src"));
    let output = format_structure_explanation(&explanation, ExplainFormat::Text).unwrap();

    assert!(output.contains("max_files=unlimited"));
}

#[test]
fn format_structure_with_no_limits_shows_none() {
    let config = StructureConfig {
        max_files: None,
        max_dirs: None,
        max_depth: None,
        ..Default::default()
    };

    let checker = crate::checker::StructureChecker::new(&config).unwrap();
    let explanation = checker.explain(&PathBuf::from("src"));
    let output = format_structure_explanation(&explanation, ExplainFormat::Text).unwrap();

    assert!(output.contains("max_files=none"));
    assert!(output.contains("max_dirs=none"));
    assert!(output.contains("max_depth=none"));
}

// ============================================================================
// format_limit helper tests
// ============================================================================

#[test]
fn format_limit_helper_values() {
    assert_eq!(format_limit(None), "none");
    assert_eq!(format_limit(Some(-1)), "unlimited");
    assert_eq!(format_limit(Some(0)), "0");
    assert_eq!(format_limit(Some(50)), "50");
}
