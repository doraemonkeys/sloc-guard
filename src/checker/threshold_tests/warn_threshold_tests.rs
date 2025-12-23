//! Tests for warning threshold behavior.

use std::path::Path;

use super::*;

#[test]
fn path_rule_warn_threshold_overrides_default() {
    let mut config = default_config();
    config.content.rules.push(crate::config::ContentRule {
        pattern: "src/generated/**".to_string(),
        max_lines: 1000,
        warn_threshold: Some(1.0), // Disable warnings
        skip_comments: None,
        skip_blank: None,
        reason: None,
        expires: None,
    });

    let checker = ThresholdChecker::new(config);
    let stats = stats_with_code(999); // 99.9% of limit

    // With warn_threshold=1.0, should not warn
    let result = checker.check(Path::new("src/generated/parser.rs"), &stats);
    assert!(result.is_passed());
}

#[test]
fn path_rule_without_warn_threshold_uses_default() {
    let mut config = default_config();
    config.content.rules.push(crate::config::ContentRule {
        pattern: "src/generated/**".to_string(),
        max_lines: 1000,
        warn_threshold: None,
        skip_comments: None,
        skip_blank: None,
        reason: None,
        expires: None,
    });

    let checker = ThresholdChecker::new(config).with_warning_threshold(0.9);
    let stats = stats_with_code(950); // 95% of limit, above 90%

    // Without custom warn_threshold, should use default (0.9)
    let result = checker.check(Path::new("src/generated/parser.rs"), &stats);
    assert!(result.is_warning());
}

#[test]
fn rule_warn_threshold_overrides_default() {
    let mut config = default_config();
    // Use V2 content.rules format
    config.content.rules.push(crate::config::ContentRule {
        pattern: "**/*.rs".to_string(),
        max_lines: 500,
        warn_threshold: Some(0.8),
        skip_comments: None,
        skip_blank: None,
        reason: None,
        expires: None,
    });

    let checker = ThresholdChecker::new(config).with_warning_threshold(0.9);
    let stats = stats_with_code(410); // 82% of 500 limit

    // With rule warn_threshold=0.8, should warn at 82%
    let result = checker.check(Path::new("test.rs"), &stats);
    assert!(result.is_warning());
}

#[test]
fn rule_without_warn_threshold_uses_default() {
    let mut config = default_config();
    // Use V2 content.rules format
    config.content.rules.push(crate::config::ContentRule {
        pattern: "**/*.rs".to_string(),
        max_lines: 500,
        warn_threshold: None,
        skip_comments: None,
        skip_blank: None,
        reason: None,
        expires: None,
    });

    let checker = ThresholdChecker::new(config).with_warning_threshold(0.9);
    let stats = stats_with_code(410); // 82% of 500 limit

    // Without rule warn_threshold, should use default 0.9 (no warning at 82%)
    let result = checker.check(Path::new("test.rs"), &stats);
    assert!(result.is_passed());
}

#[test]
fn path_rule_warn_threshold_overrides_extension_rule() {
    let mut config = default_config();
    // Use V2 content.rules format: extension rule first, then specific path rule
    config.content.rules.push(crate::config::ContentRule {
        pattern: "**/*.rs".to_string(),
        max_lines: 500,
        warn_threshold: Some(0.8),
        skip_comments: None,
        skip_blank: None,
        reason: None,
        expires: None,
    });
    config.content.rules.push(crate::config::ContentRule {
        pattern: "src/generated/**".to_string(),
        max_lines: 500,
        warn_threshold: Some(1.0), // Disable warnings
        skip_comments: None,
        skip_blank: None,
        reason: None,
        expires: None,
    });

    let checker = ThresholdChecker::new(config);
    let stats = stats_with_code(450); // 90% of limit

    // path_rule warn_threshold=1.0 should override extension rule's 0.8 (last match wins)
    let result = checker.check(Path::new("src/generated/parser.rs"), &stats);
    assert!(result.is_passed());

    // Non-matching path should use extension rule's warn_threshold=0.8
    let result2 = checker.check(Path::new("src/lib.rs"), &stats);
    assert!(result2.is_warning());
}

#[test]
fn multiple_rules_winner_takes_all_warn_threshold() {
    let mut config = default_config();
    config.content.warn_threshold = 0.9;

    // Rule 1: Warn strict (0.5)
    config.content.rules.push(crate::config::ContentRule {
        pattern: "**/*.rs".to_string(),
        max_lines: 1000,
        warn_threshold: Some(0.5),
        skip_comments: None,
        skip_blank: None,
        reason: None,
        expires: None,
    });

    // Rule 2: Override for specific file, default warn threshold (None)
    config.content.rules.push(crate::config::ContentRule {
        pattern: "src/main.rs".to_string(),
        max_lines: 1000,
        warn_threshold: None,
        skip_comments: None,
        skip_blank: None,
        reason: None,
        expires: None,
    });

    let checker = ThresholdChecker::new(config);
    let stats = stats_with_code(600); // 60% of limit.
    // If inherit 0.5 -> Warn.
    // If winner takes all 0.9 -> Pass.

    let result = checker.check(Path::new("src/main.rs"), &stats);

    // With fix, this should PASS (uses 0.9).
    assert!(result.is_passed());
}
