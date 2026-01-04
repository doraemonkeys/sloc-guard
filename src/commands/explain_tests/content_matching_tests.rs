//! Tests for content rule matching in explain command.
//!
//! Covers: rule pattern matching, last-rule-wins semantics, excluded files,
//! and `warn_at` source tracking (global vs rule, absolute vs percentage).

use std::path::PathBuf;

use crate::checker::{ContentRuleMatch, MatchStatus, WarnAtSource};
use crate::config::{Config, ContentConfig, ContentRule};

#[test]
fn explain_content_rule_matches() {
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

    assert!(matches!(
        explanation.matched_rule,
        ContentRuleMatch::Rule { index: 0, pattern, .. } if pattern == "**/*.rs"
    ));
    assert_eq!(explanation.effective_limit, 300);
}

#[test]
fn explain_content_rule_with_reason() {
    let config = Config {
        content: ContentConfig {
            max_lines: 500,
            rules: vec![ContentRule {
                pattern: "src/legacy/**".to_string(),
                max_lines: 1000,
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
    let explanation = checker.explain(&PathBuf::from("src/legacy/parser.rs"));

    assert!(matches!(
        &explanation.matched_rule,
        ContentRuleMatch::Rule { index: 0, reason, .. } if *reason == Some("Legacy code".to_string())
    ));
    assert_eq!(explanation.effective_limit, 1000);
}

#[test]
fn explain_content_default_matches() {
    let config = Config {
        content: ContentConfig {
            max_lines: 500,
            ..Default::default()
        },
        ..Default::default()
    };

    let checker = crate::checker::ThresholdChecker::new(config).unwrap();
    let explanation = checker.explain(&PathBuf::from("src/main.rs"));

    assert!(matches!(
        explanation.matched_rule,
        ContentRuleMatch::Default
    ));
    assert_eq!(explanation.effective_limit, 500);
}

#[test]
fn explain_content_last_rule_wins() {
    let config = Config {
        content: ContentConfig {
            max_lines: 500,
            rules: vec![
                ContentRule {
                    pattern: "**/*.rs".to_string(),
                    max_lines: 300,
                    warn_threshold: None,
                    warn_at: None,
                    skip_comments: None,
                    skip_blank: None,
                    reason: None,
                    expires: None,
                },
                ContentRule {
                    pattern: "src/generated/**".to_string(),
                    max_lines: 1000,
                    warn_threshold: None,
                    warn_at: None,
                    skip_comments: None,
                    skip_blank: None,
                    reason: None,
                    expires: None,
                },
            ],
            ..Default::default()
        },
        ..Default::default()
    };

    let checker = crate::checker::ThresholdChecker::new(config).unwrap();
    let explanation = checker.explain(&PathBuf::from("src/generated/types.rs"));

    // Both rules match, but the last one (index 1) should win
    assert!(matches!(
        explanation.matched_rule,
        ContentRuleMatch::Rule { index: 1, pattern, .. } if pattern == "src/generated/**"
    ));
    assert_eq!(explanation.effective_limit, 1000);

    // Check rule chain status
    let chain = &explanation.rule_chain;
    // Find the rules in the chain
    let first_rule = chain.iter().find(|c| c.source == "content.rules[0]");
    let second_rule = chain.iter().find(|c| c.source == "content.rules[1]");

    assert!(first_rule.is_some());
    assert!(second_rule.is_some());
    assert_eq!(first_rule.unwrap().status, MatchStatus::Superseded);
    assert_eq!(second_rule.unwrap().status, MatchStatus::Matched);
}

#[test]
fn explain_content_specific_rule_beats_general() {
    let config = Config {
        content: ContentConfig {
            max_lines: 500,
            rules: vec![
                ContentRule {
                    pattern: "**/*.rs".to_string(),
                    max_lines: 300,
                    warn_threshold: None,
                    warn_at: None,
                    skip_comments: None,
                    skip_blank: None,
                    reason: None,
                    expires: None,
                },
                ContentRule {
                    pattern: "src/main.rs".to_string(),
                    max_lines: 2000,
                    warn_threshold: None,
                    warn_at: None,
                    skip_comments: None,
                    skip_blank: None,
                    reason: Some("Special file".to_string()),
                    expires: None,
                },
            ],
            ..Default::default()
        },
        ..Default::default()
    };

    let checker = crate::checker::ThresholdChecker::new(config).unwrap();
    let explanation = checker.explain(&PathBuf::from("src/main.rs"));

    // Last matching rule should win
    assert!(matches!(
        explanation.matched_rule,
        ContentRuleMatch::Rule { index: 1, .. }
    ));
    assert_eq!(explanation.effective_limit, 2000);
}

// ============================================================================
// Excluded files
// ============================================================================

#[test]
fn explain_content_excluded_file() {
    let config = Config {
        content: ContentConfig {
            max_lines: 500,
            exclude: vec!["**/*.generated.rs".to_string()],
            ..Default::default()
        },
        ..Default::default()
    };

    let checker = crate::checker::ThresholdChecker::new(config).unwrap();
    let explanation = checker.explain(&PathBuf::from("src/types.generated.rs"));

    assert!(explanation.is_excluded);
    assert!(matches!(
        explanation.matched_rule,
        ContentRuleMatch::Excluded { .. }
    ));
}

// ============================================================================
// Warn-at source tracking
// ============================================================================

#[test]
fn explain_content_warn_at_source_global_percentage() {
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

    // Default case: warn_at derived from global percentage threshold
    assert!(matches!(
        explanation.warn_at_source,
        WarnAtSource::GlobalPercentage { threshold } if (threshold - 0.9).abs() < 0.01
    ));
}

#[test]
fn explain_content_warn_at_source_global_absolute() {
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

    // Global absolute warn_at
    assert!(matches!(
        explanation.warn_at_source,
        WarnAtSource::GlobalAbsolute
    ));
    assert_eq!(explanation.effective_warn_at, 400);
}

#[test]
fn explain_content_warn_at_source_rule_absolute() {
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

    // Rule absolute warn_at
    assert!(matches!(
        explanation.warn_at_source,
        WarnAtSource::RuleAbsolute { index: 0 }
    ));
    assert_eq!(explanation.effective_warn_at, 250);
}

#[test]
fn explain_content_warn_at_source_rule_percentage() {
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

    // Rule percentage threshold
    assert!(matches!(
        explanation.warn_at_source,
        WarnAtSource::RulePercentage { index: 0, threshold } if (threshold - 0.8).abs() < 0.01
    ));
    // 300 * 0.8 = 240
    assert_eq!(explanation.effective_warn_at, 240);
}
