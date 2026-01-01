//! Tests for basic threshold checking behavior and `CheckResult` methods.

use std::path::{Path, PathBuf};

use super::*;

#[test]
fn check_passes_under_threshold() {
    let checker = ThresholdChecker::new(default_config()).unwrap();
    let stats = stats_with_code(100);

    let result = checker.check(Path::new("test.rs"), &stats, None);

    assert!(result.is_passed());
    assert_eq!(result.limit(), 600);
}

#[test]
fn check_fails_over_threshold() {
    let checker = ThresholdChecker::new(default_config()).unwrap();
    let stats = stats_with_code(700);

    let result = checker.check(Path::new("test.rs"), &stats, None);

    assert!(result.is_failed());
}

#[test]
fn check_warns_near_threshold() {
    let checker = ThresholdChecker::new(default_config()).unwrap();
    let stats = stats_with_code(550); // 90% of 600 = 540

    let result = checker.check(Path::new("test.rs"), &stats, None);

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
        warn_at: None,
        skip_comments: None,
        skip_blank: None,
        reason: None,
        expires: None,
    });

    let checker = ThresholdChecker::new(config).unwrap();
    let stats = stats_with_code(350);

    let result = checker.check(Path::new("test.rs"), &stats, None);

    assert!(result.is_failed());
    assert_eq!(result.limit(), 300);
}

#[test]
fn check_uses_rule_with_reason() {
    let mut config = default_config();
    config.content.rules.push(crate::config::ContentRule {
        pattern: "**/legacy.rs".to_string(),
        max_lines: 800,
        warn_threshold: None,
        warn_at: None,
        skip_comments: None,
        skip_blank: None,
        reason: Some("Legacy code".to_string()),
        expires: None,
    });

    let checker = ThresholdChecker::new(config).unwrap();
    let stats = stats_with_code(700);

    let result = checker.check(Path::new("src/legacy.rs"), &stats, None);

    assert!(result.is_passed());
    assert_eq!(result.limit(), 800);
    assert_eq!(result.override_reason(), Some("Legacy code"));
}

#[test]
fn check_rule_without_reason() {
    let mut config = default_config();
    config.content.rules.push(crate::config::ContentRule {
        pattern: "**/special.rs".to_string(),
        max_lines: 800,
        warn_threshold: None,
        warn_at: None,
        skip_comments: None,
        skip_blank: None,
        reason: None,
        expires: None,
    });

    let checker = ThresholdChecker::new(config).unwrap();
    let stats = stats_with_code(700);

    let result = checker.check(Path::new("src/special.rs"), &stats, None);

    assert!(result.is_passed());
    assert_eq!(result.limit(), 800);
    assert_eq!(result.override_reason(), None);
}

#[test]
fn check_no_override_reason_when_using_default() {
    let checker = ThresholdChecker::new(default_config()).unwrap();
    let stats = stats_with_code(100);

    let result = checker.check(Path::new("test.rs"), &stats, None);

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
        raw_stats: None,
        limit: 500,
        override_reason: None,
        violation_category: None,
    };

    assert!((result.usage_percent() - 50.0).abs() < 0.01);
}

#[test]
fn custom_warning_threshold() {
    let checker = ThresholdChecker::new(default_config())
        .unwrap()
        .with_warning_threshold(0.8);
    let stats = stats_with_code(490); // 80% of 600 = 480

    let result = checker.check(Path::new("test.rs"), &stats, None);

    assert!(result.is_warning());
}

#[test]
fn check_result_is_methods() {
    let passed = CheckResult::Passed {
        path: PathBuf::from("test.rs"),
        stats: stats_with_code(100),
        raw_stats: None,
        limit: 500,
        override_reason: None,
        violation_category: None,
    };
    let failed = CheckResult::Failed {
        path: PathBuf::from("test.rs"),
        stats: stats_with_code(600),
        raw_stats: None,
        limit: 500,
        override_reason: None,
        suggestions: None,
        violation_category: None,
    };
    let warning = CheckResult::Warning {
        path: PathBuf::from("test.rs"),
        stats: stats_with_code(450),
        raw_stats: None,
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

#[test]
fn raw_stats_returns_raw_when_present() {
    let raw = LineStats {
        total: 160,
        code: 100,
        comment: 50,
        blank: 10,
        ignored: 0,
    };
    let effective = LineStats {
        total: 100,
        code: 100,
        comment: 0,
        blank: 0,
        ignored: 0,
    };

    let result = CheckResult::Passed {
        path: PathBuf::from("test.rs"),
        stats: effective,
        raw_stats: Some(raw),
        limit: 500,
        override_reason: None,
        violation_category: None,
    };

    assert_eq!(result.raw_stats().comment, 50);
    assert_eq!(result.raw_stats().blank, 10);
    assert_eq!(result.raw_stats().code, 100);
}

#[test]
fn raw_stats_falls_back_to_stats_when_none() {
    let result = CheckResult::Passed {
        path: PathBuf::from("test.rs"),
        stats: LineStats {
            total: 135,
            code: 100,
            comment: 25,
            blank: 10,
            ignored: 0,
        },
        raw_stats: None,
        limit: 500,
        override_reason: None,
        violation_category: None,
    };

    assert_eq!(result.raw_stats().comment, 25);
    assert_eq!(result.raw_stats().blank, 10);
    assert_eq!(result.raw_stats().code, 100);
}

#[test]
fn raw_stats_works_for_all_variants() {
    let raw = LineStats {
        total: 160,
        code: 100,
        comment: 50,
        blank: 10,
        ignored: 0,
    };

    // Test Warning variant
    let warning = CheckResult::Warning {
        path: PathBuf::from("test.rs"),
        stats: stats_with_code(100),
        raw_stats: Some(raw.clone()),
        limit: 500,
        override_reason: None,
        suggestions: None,
        violation_category: None,
    };
    assert_eq!(warning.raw_stats().comment, 50);

    // Test Failed variant
    let failed = CheckResult::Failed {
        path: PathBuf::from("test.rs"),
        stats: stats_with_code(100),
        raw_stats: Some(raw.clone()),
        limit: 500,
        override_reason: None,
        suggestions: None,
        violation_category: None,
    };
    assert_eq!(failed.raw_stats().comment, 50);

    // Test Grandfathered variant
    let grandfathered = CheckResult::Grandfathered {
        path: PathBuf::from("test.rs"),
        stats: stats_with_code(100),
        raw_stats: Some(raw),
        limit: 500,
        override_reason: None,
        violation_category: None,
    };
    assert_eq!(grandfathered.raw_stats().comment, 50);
}
