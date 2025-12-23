//! Tests for basic threshold checking behavior and `CheckResult` methods.

use std::path::{Path, PathBuf};

use super::*;

#[test]
fn check_passes_under_threshold() {
    let checker = ThresholdChecker::new(default_config());
    let stats = stats_with_code(100);

    let result = checker.check(Path::new("test.rs"), &stats);

    assert!(result.is_passed());
    assert_eq!(result.limit(), 500);
}

#[test]
fn check_fails_over_threshold() {
    let checker = ThresholdChecker::new(default_config());
    let stats = stats_with_code(600);

    let result = checker.check(Path::new("test.rs"), &stats);

    assert!(result.is_failed());
}

#[test]
fn check_warns_near_threshold() {
    let checker = ThresholdChecker::new(default_config());
    let stats = stats_with_code(460);

    let result = checker.check(Path::new("test.rs"), &stats);

    assert!(result.is_warning());
}

#[test]
fn check_uses_rule_specific_limit() {
    let mut config = default_config();
    // Use V2 content.rules format instead of legacy config.rules
    config.content.rules.push(crate::config::ContentRule {
        pattern: "**/*.rs".to_string(),
        max_lines: 300,
        warn_threshold: None,
        skip_comments: None,
        skip_blank: None,
        reason: None,
        expires: None,
    });

    let checker = ThresholdChecker::new(config);
    let stats = stats_with_code(350);

    let result = checker.check(Path::new("test.rs"), &stats);

    assert!(result.is_failed());
    assert_eq!(result.limit(), 300);
}

#[test]
fn check_uses_override_for_specific_file() {
    let mut config = default_config();
    config.overrides.push(crate::config::FileOverride {
        path: "legacy.rs".to_string(),
        max_lines: 800,
        reason: Some("Legacy code".to_string()),
    });

    let checker = ThresholdChecker::new(config);
    let stats = stats_with_code(700);

    let result = checker.check(Path::new("src/legacy.rs"), &stats);

    assert!(result.is_passed());
    assert_eq!(result.limit(), 800);
    assert_eq!(result.override_reason(), Some("Legacy code"));
}

#[test]
fn check_override_without_reason() {
    let mut config = default_config();
    config.overrides.push(crate::config::FileOverride {
        path: "special.rs".to_string(),
        max_lines: 800,
        reason: None,
    });

    let checker = ThresholdChecker::new(config);
    let stats = stats_with_code(700);

    let result = checker.check(Path::new("src/special.rs"), &stats);

    assert!(result.is_passed());
    assert_eq!(result.limit(), 800);
    assert_eq!(result.override_reason(), None);
}

#[test]
fn check_no_override_reason_when_using_default() {
    let checker = ThresholdChecker::new(default_config());
    let stats = stats_with_code(100);

    let result = checker.check(Path::new("test.rs"), &stats);

    assert!(result.is_passed());
    assert_eq!(result.override_reason(), None);
}

#[test]
fn check_result_usage_percent() {
    let result = CheckResult::Passed {
        path: PathBuf::from("test.rs"),
        stats: LineStats {
            total: 260,
            code: 250,
            comment: 5,
            blank: 5,
            ignored: 0,
        },
        limit: 500,
        override_reason: None,
        violation_category: None,
    };

    assert!((result.usage_percent() - 50.0).abs() < 0.01);
}

#[test]
fn custom_warning_threshold() {
    let checker = ThresholdChecker::new(default_config()).with_warning_threshold(0.8);
    let stats = stats_with_code(410);

    let result = checker.check(Path::new("test.rs"), &stats);

    assert!(result.is_warning());
}

#[test]
fn check_result_is_methods() {
    let passed = CheckResult::Passed {
        path: PathBuf::from("test.rs"),
        stats: stats_with_code(100),
        limit: 500,
        override_reason: None,
        violation_category: None,
    };
    let failed = CheckResult::Failed {
        path: PathBuf::from("test.rs"),
        stats: stats_with_code(600),
        limit: 500,
        override_reason: None,
        suggestions: None,
        violation_category: None,
    };
    let warning = CheckResult::Warning {
        path: PathBuf::from("test.rs"),
        stats: stats_with_code(450),
        limit: 500,
        override_reason: None,
        suggestions: None,
        violation_category: None,
    };

    assert!(passed.is_passed());
    assert!(!passed.is_failed());
    assert!(failed.is_failed());
    assert!(!failed.is_passed());
    assert!(warning.is_warning());
    assert!(!warning.is_failed());
}
