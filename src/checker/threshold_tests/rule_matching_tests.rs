//! Tests for content rules and glob pattern matching.

use std::path::Path;

use super::*;

#[test]
fn path_rule_matches_glob_pattern() {
    let mut config = default_config();
    config.content.rules.push(crate::config::ContentRule {
        pattern: "src/generated/**".to_string(),
        max_lines: 1000,
        warn_threshold: None,
        warn_at: None,
        skip_comments: None,
        skip_blank: None,
        reason: None,
        expires: None,
    });

    let checker = ThresholdChecker::new(config);
    let stats = stats_with_code(800);

    // File matching the glob pattern should use content.rules max_lines
    let result = checker.check(Path::new("src/generated/parser.rs"), &stats, None);
    assert!(result.is_passed());
    assert_eq!(result.limit(), 1000);
}

#[test]
fn path_rule_does_not_match_unrelated_path() {
    let mut config = default_config();
    config.content.rules.push(crate::config::ContentRule {
        pattern: "src/generated/**".to_string(),
        max_lines: 1000,
        warn_threshold: None,
        warn_at: None,
        skip_comments: None,
        skip_blank: None,
        reason: None,
        expires: None,
    });

    let checker = ThresholdChecker::new(config);
    let stats = stats_with_code(600);

    // File not matching the pattern should use default
    let result = checker.check(Path::new("src/lib.rs"), &stats, None);
    assert!(result.is_failed());
    assert_eq!(result.limit(), 500);
}

#[test]
fn later_rule_overrides_earlier_rule() {
    let mut config = default_config();
    config.content.rules.push(crate::config::ContentRule {
        pattern: "src/generated/**".to_string(),
        max_lines: 1000,
        warn_threshold: None,
        warn_at: None,
        skip_comments: None,
        skip_blank: None,
        reason: None,
        expires: None,
    });
    // More specific rule added later - last match wins
    config.content.rules.push(crate::config::ContentRule {
        pattern: "**/special.rs".to_string(),
        max_lines: 2000,
        warn_threshold: None,
        warn_at: None,
        skip_comments: None,
        skip_blank: None,
        reason: None,
        expires: None,
    });

    let checker = ThresholdChecker::new(config);
    let stats = stats_with_code(1500);

    // Later rule should take priority (last match wins)
    let result = checker.check(Path::new("src/generated/special.rs"), &stats, None);
    assert!(result.is_passed());
    assert_eq!(result.limit(), 2000);
}

#[test]
fn path_rule_has_higher_priority_than_extension_rule() {
    let mut config = default_config();
    // Use V2 content.rules format: extension rule first, then specific path rule
    // Since "last match wins", the path rule should override the extension rule
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
    config.content.rules.push(crate::config::ContentRule {
        pattern: "**/proto/**".to_string(),
        max_lines: 800,
        warn_threshold: None,
        warn_at: None,
        skip_comments: None,
        skip_blank: None,
        reason: None,
        expires: None,
    });

    let checker = ThresholdChecker::new(config);
    let stats = stats_with_code(400);

    // path_rule should override extension rule (last match wins)
    let result = checker.check(Path::new("src/proto/messages.rs"), &stats, None);
    assert!(result.is_passed());
    assert_eq!(result.limit(), 800);

    // Non-matching path should use extension rule
    let result2 = checker.check(Path::new("src/lib.rs"), &stats, None);
    assert!(result2.is_failed());
    assert_eq!(result2.limit(), 300);
}

#[test]
fn multiple_path_rules_last_match_wins() {
    let mut config = default_config();
    config.content.rules.push(crate::config::ContentRule {
        pattern: "src/**".to_string(),
        max_lines: 600,
        warn_threshold: None,
        warn_at: None,
        skip_comments: None,
        skip_blank: None,
        reason: None,
        expires: None,
    });
    config.content.rules.push(crate::config::ContentRule {
        pattern: "src/generated/**".to_string(),
        max_lines: 1000,
        warn_threshold: None,
        warn_at: None,
        skip_comments: None,
        skip_blank: None,
        reason: None,
        expires: None,
    });

    let checker = ThresholdChecker::new(config);
    let stats = stats_with_code(700);

    // Last matching rule should be used (1000 limit)
    let result = checker.check(Path::new("src/generated/parser.rs"), &stats, None);
    assert!(result.is_passed()); // 700 < 1000
    assert_eq!(result.limit(), 1000);
}
