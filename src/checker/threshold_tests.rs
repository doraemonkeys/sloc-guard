use std::path::Path;
use std::path::PathBuf;

use super::*;

fn default_config() -> Config {
    Config::default()
}

fn stats_with_code(code: usize) -> LineStats {
    LineStats {
        total: code + 10,
        code,
        comment: 5,
        blank: 5,
    }
}

#[test]
fn check_passes_under_threshold() {
    let checker = ThresholdChecker::new(default_config());
    let stats = stats_with_code(100);

    let result = checker.check(Path::new("test.rs"), &stats);

    assert!(result.is_passed());
    assert_eq!(result.limit, 500);
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
    config.rules.insert(
        "rust".to_string(),
        crate::config::RuleConfig {
            extensions: vec!["rs".to_string()],
            max_lines: Some(300),
            skip_comments: None,
            skip_blank: None,
        },
    );

    let checker = ThresholdChecker::new(config);
    let stats = stats_with_code(350);

    let result = checker.check(Path::new("test.rs"), &stats);

    assert!(result.is_failed());
    assert_eq!(result.limit, 300);
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
    assert_eq!(result.limit, 800);
}

#[test]
fn check_result_usage_percent() {
    let result = CheckResult {
        path: PathBuf::from("test.rs"),
        status: CheckStatus::Passed,
        stats: LineStats {
            total: 260,
            code: 250,
            comment: 5,
            blank: 5,
        },
        limit: 500,
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
fn check_status_equality() {
    assert_eq!(CheckStatus::Passed, CheckStatus::Passed);
    assert_ne!(CheckStatus::Passed, CheckStatus::Failed);
    assert_ne!(CheckStatus::Warning, CheckStatus::Failed);
}
