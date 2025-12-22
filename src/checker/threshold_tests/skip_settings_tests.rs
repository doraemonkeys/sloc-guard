//! Tests for `skip_comments` and `skip_blank` settings.

use std::path::Path;

use super::*;

#[test]
fn get_skip_settings_from_path_rule() {
    let mut config = default_config();
    config.content.skip_comments = true;
    config.content.skip_blank = true;
    config.content.rules.push(crate::config::ContentRule {
        pattern: "src/generated/**".to_string(),
        max_lines: 1000,
        warn_threshold: None,
        skip_comments: Some(false),
        skip_blank: Some(false),
    });

    let checker = ThresholdChecker::new(config);

    // Matching path should use rule's skip settings
    let (skip_comments, skip_blank) =
        checker.get_skip_settings_for_path(Path::new("src/generated/parser.rs"));
    assert!(!skip_comments);
    assert!(!skip_blank);
}

#[test]
fn get_skip_settings_falls_back_to_global() {
    let mut config = default_config();
    config.content.skip_comments = true;
    config.content.skip_blank = false;
    config.content.rules.push(crate::config::ContentRule {
        pattern: "src/generated/**".to_string(),
        max_lines: 1000,
        warn_threshold: None,
        skip_comments: None, // Not specified
        skip_blank: None,    // Not specified
    });

    let checker = ThresholdChecker::new(config);

    // Should use global defaults when rule doesn't specify
    let (skip_comments, skip_blank) =
        checker.get_skip_settings_for_path(Path::new("src/generated/parser.rs"));
    assert!(skip_comments); // from global
    assert!(!skip_blank); // from global
}

#[test]
fn get_skip_settings_uses_global_when_no_rule_matches() {
    let mut config = default_config();
    config.content.skip_comments = false;
    config.content.skip_blank = true;

    let checker = ThresholdChecker::new(config);

    // Non-matching path should use global defaults
    let (skip_comments, skip_blank) = checker.get_skip_settings_for_path(Path::new("src/lib.rs"));
    assert!(!skip_comments);
    assert!(skip_blank);
}

#[test]
fn get_skip_settings_last_match_wins() {
    let mut config = default_config();
    config.content.skip_comments = true;
    config.content.skip_blank = true;
    config.content.rules.push(crate::config::ContentRule {
        pattern: "src/**".to_string(),
        max_lines: 500,
        warn_threshold: None,
        skip_comments: Some(false),
        skip_blank: Some(false),
    });
    config.content.rules.push(crate::config::ContentRule {
        pattern: "src/generated/**".to_string(),
        max_lines: 1000,
        warn_threshold: None,
        skip_comments: Some(true),
        skip_blank: Some(false),
    });

    let checker = ThresholdChecker::new(config);

    // Last matching rule should be used
    let (skip_comments, skip_blank) =
        checker.get_skip_settings_for_path(Path::new("src/generated/parser.rs"));
    assert!(skip_comments); // From last matching rule
    assert!(!skip_blank); // From last matching rule

    // First rule only for non-generated
    let (skip_comments2, skip_blank2) = checker.get_skip_settings_for_path(Path::new("src/lib.rs"));
    assert!(!skip_comments2);
    assert!(!skip_blank2);
}

#[test]
fn language_rule_expanded_skip_settings_work() {
    // This tests that language rules (expanded to content.rules by loader)
    // correctly apply skip_comments/skip_blank
    let mut config = default_config();
    config.content.skip_comments = true;
    config.content.skip_blank = true;

    // Simulate expanded language rule (what loader does)
    config.content.rules.push(crate::config::ContentRule {
        pattern: "**/*.rs".to_string(),
        max_lines: 300,
        warn_threshold: None,
        skip_comments: Some(false),
        skip_blank: Some(true),
    });

    let checker = ThresholdChecker::new(config);

    let (skip_comments, skip_blank) = checker.get_skip_settings_for_path(Path::new("src/lib.rs"));
    assert!(!skip_comments); // From rule
    assert!(skip_blank); // From rule
}
