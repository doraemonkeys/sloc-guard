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
        ignored: 0,
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
            warn_threshold: None,
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
    assert_eq!(result.override_reason, Some("Legacy code".to_string()));
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
    assert_eq!(result.limit, 800);
    assert_eq!(result.override_reason, None);
}

#[test]
fn check_no_override_reason_when_using_default() {
    let checker = ThresholdChecker::new(default_config());
    let stats = stats_with_code(100);

    let result = checker.check(Path::new("test.rs"), &stats);

    assert!(result.is_passed());
    assert_eq!(result.override_reason, None);
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
            blank: 5, ignored: 0,
        },
        limit: 500,
        override_reason: None,
        suggestions: None,
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
fn override_does_not_match_partial_filename() {
    let mut config = default_config();
    config.overrides.push(crate::config::FileOverride {
        path: "parser.rs".to_string(),
        max_lines: 800,
        reason: None,
    });

    let checker = ThresholdChecker::new(config);
    let stats = stats_with_code(600);

    // "my_parser.rs" should NOT match override for "parser.rs"
    let result = checker.check(Path::new("src/my_parser.rs"), &stats);

    assert!(result.is_failed());
    assert_eq!(result.limit, 500); // default limit, not override
}

#[test]
fn override_matches_exact_filename() {
    let mut config = default_config();
    config.overrides.push(crate::config::FileOverride {
        path: "parser.rs".to_string(),
        max_lines: 800,
        reason: None,
    });

    let checker = ThresholdChecker::new(config);
    let stats = stats_with_code(600);

    // "parser.rs" should match override for "parser.rs"
    let result = checker.check(Path::new("src/parser.rs"), &stats);

    assert!(result.is_passed());
    assert_eq!(result.limit, 800);
}

#[test]
fn override_matches_full_path() {
    let mut config = default_config();
    config.overrides.push(crate::config::FileOverride {
        path: "src/legacy/parser.rs".to_string(),
        max_lines: 800,
        reason: None,
    });

    let checker = ThresholdChecker::new(config);
    let stats = stats_with_code(600);

    // "src/legacy/parser.rs" should match
    let result = checker.check(Path::new("src/legacy/parser.rs"), &stats);
    assert!(result.is_passed());
    assert_eq!(result.limit, 800);

    // "other/src/legacy/parser.rs" should also match (ends with)
    let result2 = checker.check(Path::new("other/src/legacy/parser.rs"), &stats);
    assert!(result2.is_passed());
    assert_eq!(result2.limit, 800);

    // "legacy/parser.rs" should NOT match (missing "src" component)
    let result3 = checker.check(Path::new("legacy/parser.rs"), &stats);
    assert!(result3.is_failed());
    assert_eq!(result3.limit, 500);
}

#[test]
fn check_status_equality() {
    assert_eq!(CheckStatus::Passed, CheckStatus::Passed);
    assert_ne!(CheckStatus::Passed, CheckStatus::Failed);
    assert_ne!(CheckStatus::Warning, CheckStatus::Failed);
}

#[test]
fn path_rule_matches_glob_pattern() {
    let mut config = default_config();
    config.path_rules.push(crate::config::PathRule {
        pattern: "src/generated/**".to_string(),
        max_lines: 1000,
        warn_threshold: None,
    });

    let checker = ThresholdChecker::new(config);
    let stats = stats_with_code(800);

    // File matching the glob pattern should use path_rule max_lines
    let result = checker.check(Path::new("src/generated/parser.rs"), &stats);
    assert!(result.is_passed());
    assert_eq!(result.limit, 1000);
}

#[test]
fn path_rule_does_not_match_unrelated_path() {
    let mut config = default_config();
    config.path_rules.push(crate::config::PathRule {
        pattern: "src/generated/**".to_string(),
        max_lines: 1000,
        warn_threshold: None,
    });

    let checker = ThresholdChecker::new(config);
    let stats = stats_with_code(600);

    // File not matching the pattern should use default
    let result = checker.check(Path::new("src/lib.rs"), &stats);
    assert!(result.is_failed());
    assert_eq!(result.limit, 500);
}

#[test]
fn path_rule_has_lower_priority_than_override() {
    let mut config = default_config();
    config.path_rules.push(crate::config::PathRule {
        pattern: "src/generated/**".to_string(),
        max_lines: 1000,
        warn_threshold: None,
    });
    config.overrides.push(crate::config::FileOverride {
        path: "special.rs".to_string(),
        max_lines: 2000,
        reason: None,
    });

    let checker = ThresholdChecker::new(config);
    let stats = stats_with_code(1500);

    // Override should take priority over path_rule
    let result = checker.check(Path::new("src/generated/special.rs"), &stats);
    assert!(result.is_passed());
    assert_eq!(result.limit, 2000);
}

#[test]
fn path_rule_has_higher_priority_than_extension_rule() {
    let mut config = default_config();
    config.rules.insert(
        "rust".to_string(),
        crate::config::RuleConfig {
            extensions: vec!["rs".to_string()],
            max_lines: Some(300),
            skip_comments: None,
            skip_blank: None,
            warn_threshold: None,
        },
    );
    config.path_rules.push(crate::config::PathRule {
        pattern: "**/proto/**".to_string(),
        max_lines: 800,
        warn_threshold: None,
    });

    let checker = ThresholdChecker::new(config);
    let stats = stats_with_code(400);

    // path_rule should override extension rule
    let result = checker.check(Path::new("src/proto/messages.rs"), &stats);
    assert!(result.is_passed());
    assert_eq!(result.limit, 800);

    // Non-matching path should use extension rule
    let result2 = checker.check(Path::new("src/lib.rs"), &stats);
    assert!(result2.is_failed());
    assert_eq!(result2.limit, 300);
}

#[test]
fn path_rule_warn_threshold_overrides_default() {
    let mut config = default_config();
    config.path_rules.push(crate::config::PathRule {
        pattern: "src/generated/**".to_string(),
        max_lines: 1000,
        warn_threshold: Some(1.0), // Disable warnings
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
    config.path_rules.push(crate::config::PathRule {
        pattern: "src/generated/**".to_string(),
        max_lines: 1000,
        warn_threshold: None,
    });

    let checker = ThresholdChecker::new(config).with_warning_threshold(0.9);
    let stats = stats_with_code(950); // 95% of limit, above 90%

    // Without custom warn_threshold, should use default (0.9)
    let result = checker.check(Path::new("src/generated/parser.rs"), &stats);
    assert!(result.is_warning());
}

#[test]
fn multiple_path_rules_first_match_wins() {
    let mut config = default_config();
    config.path_rules.push(crate::config::PathRule {
        pattern: "src/**".to_string(),
        max_lines: 600,
        warn_threshold: None,
    });
    config.path_rules.push(crate::config::PathRule {
        pattern: "src/generated/**".to_string(),
        max_lines: 1000,
        warn_threshold: None,
    });

    let checker = ThresholdChecker::new(config);
    let stats = stats_with_code(700);

    // First matching rule should be used
    let result = checker.check(Path::new("src/generated/parser.rs"), &stats);
    assert!(result.is_failed()); // 700 > 600
    assert_eq!(result.limit, 600);
}

#[test]
fn rule_warn_threshold_overrides_default() {
    let mut config = default_config();
    config.rules.insert(
        "rust".to_string(),
        crate::config::RuleConfig {
            extensions: vec!["rs".to_string()],
            max_lines: Some(500),
            skip_comments: None,
            skip_blank: None,
            warn_threshold: Some(0.8),
        },
    );

    let checker = ThresholdChecker::new(config).with_warning_threshold(0.9);
    let stats = stats_with_code(410); // 82% of 500 limit

    // With rule warn_threshold=0.8, should warn at 82%
    let result = checker.check(Path::new("test.rs"), &stats);
    assert!(result.is_warning());
}

#[test]
fn rule_without_warn_threshold_uses_default() {
    let mut config = default_config();
    config.rules.insert(
        "rust".to_string(),
        crate::config::RuleConfig {
            extensions: vec!["rs".to_string()],
            max_lines: Some(500),
            skip_comments: None,
            skip_blank: None,
            warn_threshold: None,
        },
    );

    let checker = ThresholdChecker::new(config).with_warning_threshold(0.9);
    let stats = stats_with_code(410); // 82% of 500 limit

    // Without rule warn_threshold, should use default 0.9 (no warning at 82%)
    let result = checker.check(Path::new("test.rs"), &stats);
    assert!(result.is_passed());
}

#[test]
fn path_rule_warn_threshold_overrides_extension_rule() {
    let mut config = default_config();
    config.rules.insert(
        "rust".to_string(),
        crate::config::RuleConfig {
            extensions: vec!["rs".to_string()],
            max_lines: Some(500),
            skip_comments: None,
            skip_blank: None,
            warn_threshold: Some(0.8),
        },
    );
    config.path_rules.push(crate::config::PathRule {
        pattern: "src/generated/**".to_string(),
        max_lines: 500,
        warn_threshold: Some(1.0), // Disable warnings
    });

    let checker = ThresholdChecker::new(config);
    let stats = stats_with_code(450); // 90% of limit

    // path_rule warn_threshold=1.0 should override extension rule's 0.8
    let result = checker.check(Path::new("src/generated/parser.rs"), &stats);
    assert!(result.is_passed());

    // Non-matching path should use extension rule's warn_threshold=0.8
    let result2 = checker.check(Path::new("src/lib.rs"), &stats);
    assert!(result2.is_warning());
}
