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
