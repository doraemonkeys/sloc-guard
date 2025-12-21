use std::path::PathBuf;

use crate::checker::{ContentRuleMatch, MatchStatus, StructureRuleMatch};
use crate::cli::ExplainFormat;
use crate::config::{
    Config, ContentConfig, ContentOverride, ContentRule, StructureConfig, StructureOverride,
    StructureRule,
};

use super::{format_content_explanation, format_structure_explanation};

#[test]
fn explain_content_override_matches() {
    let config = Config {
        content: ContentConfig {
            max_lines: 500,
            overrides: vec![ContentOverride {
                path: "src/legacy/parser.rs".to_string(),
                max_lines: 1000,
                reason: "Legacy code".to_string(),
            }],
            ..Default::default()
        },
        ..Default::default()
    };

    let checker = crate::checker::ThresholdChecker::new(config);
    let explanation = checker.explain(&PathBuf::from("src/legacy/parser.rs"));

    assert!(matches!(
        explanation.matched_rule,
        ContentRuleMatch::Override { index: 0, .. }
    ));
    assert_eq!(explanation.effective_limit, 1000);
}

#[test]
fn explain_content_rule_matches() {
    let config = Config {
        content: ContentConfig {
            max_lines: 500,
            rules: vec![ContentRule {
                pattern: "**/*.rs".to_string(),
                max_lines: 300,
                warn_threshold: None,
                skip_comments: None,
                skip_blank: None,
            }],
            ..Default::default()
        },
        ..Default::default()
    };

    let checker = crate::checker::ThresholdChecker::new(config);
    let explanation = checker.explain(&PathBuf::from("src/main.rs"));

    assert!(matches!(
        explanation.matched_rule,
        ContentRuleMatch::Rule { index: 0, pattern } if pattern == "**/*.rs"
    ));
    assert_eq!(explanation.effective_limit, 300);
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

    let checker = crate::checker::ThresholdChecker::new(config);
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
                    skip_comments: None,
                    skip_blank: None,
                },
                ContentRule {
                    pattern: "src/generated/**".to_string(),
                    max_lines: 1000,
                    warn_threshold: None,
                    skip_comments: None,
                    skip_blank: None,
                },
            ],
            ..Default::default()
        },
        ..Default::default()
    };

    let checker = crate::checker::ThresholdChecker::new(config);
    let explanation = checker.explain(&PathBuf::from("src/generated/types.rs"));

    // Both rules match, but the last one (index 1) should win
    assert!(matches!(
        explanation.matched_rule,
        ContentRuleMatch::Rule { index: 1, pattern } if pattern == "src/generated/**"
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
fn explain_content_override_beats_rule() {
    let config = Config {
        content: ContentConfig {
            max_lines: 500,
            rules: vec![ContentRule {
                pattern: "**/*.rs".to_string(),
                max_lines: 300,
                warn_threshold: None,
                skip_comments: None,
                skip_blank: None,
            }],
            overrides: vec![ContentOverride {
                path: "src/main.rs".to_string(),
                max_lines: 2000,
                reason: "Special file".to_string(),
            }],
            ..Default::default()
        },
        ..Default::default()
    };

    let checker = crate::checker::ThresholdChecker::new(config);
    let explanation = checker.explain(&PathBuf::from("src/main.rs"));

    // Override should beat rule
    assert!(matches!(
        explanation.matched_rule,
        ContentRuleMatch::Override { index: 0, .. }
    ));
    assert_eq!(explanation.effective_limit, 2000);
}

#[test]
fn explain_structure_override_matches() {
    let config = StructureConfig {
        max_files: Some(10),
        max_dirs: Some(5),
        overrides: vec![StructureOverride {
            path: "src/legacy".to_string(),
            max_files: Some(100),
            max_dirs: None,
            max_depth: None,
            reason: "Legacy directory".to_string(),
        }],
        ..Default::default()
    };

    let checker = crate::checker::StructureChecker::new(&config).unwrap();
    let explanation = checker.explain(&PathBuf::from("src/legacy"));

    assert!(matches!(
        explanation.matched_rule,
        StructureRuleMatch::Override { index: 0, .. }
    ));
    assert_eq!(explanation.effective_max_files, Some(100));
    assert_eq!(
        explanation.override_reason,
        Some("Legacy directory".to_string())
    );
}

#[test]
fn explain_structure_rule_matches() {
    let config = StructureConfig {
        max_files: Some(10),
        max_dirs: Some(5),
        rules: vec![StructureRule {
            pattern: "src/components/*".to_string(),
            max_files: Some(50),
            max_dirs: Some(10),
            max_depth: None,
            warn_threshold: None,
            allow_extensions: vec![],
            allow_patterns: vec![],
            relative_depth: false,
        }],
        ..Default::default()
    };

    let checker = crate::checker::StructureChecker::new(&config).unwrap();
    let explanation = checker.explain(&PathBuf::from("src/components/Button"));

    assert!(matches!(
        explanation.matched_rule,
        StructureRuleMatch::Rule { index: 0, pattern } if pattern == "src/components/*"
    ));
    assert_eq!(explanation.effective_max_files, Some(50));
    assert_eq!(explanation.effective_max_dirs, Some(10));
}

#[test]
fn explain_structure_default_matches() {
    let config = StructureConfig {
        max_files: Some(10),
        max_dirs: Some(5),
        ..Default::default()
    };

    let checker = crate::checker::StructureChecker::new(&config).unwrap();
    let explanation = checker.explain(&PathBuf::from("src"));

    assert!(matches!(
        explanation.matched_rule,
        StructureRuleMatch::Default
    ));
    assert_eq!(explanation.effective_max_files, Some(10));
    assert_eq!(explanation.effective_max_dirs, Some(5));
}

#[test]
fn explain_structure_rule_chain_statuses() {
    let config = StructureConfig {
        max_files: Some(10),
        max_dirs: Some(5),
        rules: vec![
            StructureRule {
                pattern: "src/*".to_string(),
                max_files: Some(20),
                max_dirs: Some(10),
                max_depth: None,
                warn_threshold: None,
                allow_extensions: vec![],
                allow_patterns: vec![],
                relative_depth: false,
            },
            StructureRule {
                pattern: "test/*".to_string(),
                max_files: Some(30),
                max_dirs: Some(15),
                max_depth: None,
                warn_threshold: None,
                allow_extensions: vec![],
                allow_patterns: vec![],
                relative_depth: false,
            },
        ],
        overrides: vec![StructureOverride {
            path: "build".to_string(),
            max_files: Some(100),
            max_dirs: Some(50),
            max_depth: None,
            reason: "Build output".to_string(),
        }],
        ..Default::default()
    };

    let checker = crate::checker::StructureChecker::new(&config).unwrap();

    // Check a path that matches the first rule
    let explanation = checker.explain(&PathBuf::from("src/components"));

    let chain = &explanation.rule_chain;

    // Override should not match
    let override_entry = chain.iter().find(|c| c.source == "structure.overrides[0]");
    assert_eq!(override_entry.unwrap().status, MatchStatus::NoMatch);

    // First rule should match
    let first_rule = chain.iter().find(|c| c.source == "structure.rules[0]");
    assert_eq!(first_rule.unwrap().status, MatchStatus::Matched);

    // Second rule should not match
    let second_rule = chain.iter().find(|c| c.source == "structure.rules[1]");
    assert_eq!(second_rule.unwrap().status, MatchStatus::NoMatch);

    // Default should be superseded
    let default_entry = chain.iter().find(|c| c.source == "structure (default)");
    assert_eq!(default_entry.unwrap().status, MatchStatus::Superseded);
}

// Formatting tests

#[test]
fn format_content_text_output_contains_expected_sections() {
    let config = Config {
        content: ContentConfig {
            max_lines: 500,
            ..Default::default()
        },
        ..Default::default()
    };

    let checker = crate::checker::ThresholdChecker::new(config);
    let explanation = checker.explain(&PathBuf::from("src/main.rs"));
    let output = format_content_explanation(&explanation, ExplainFormat::Text);

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

    let checker = crate::checker::ThresholdChecker::new(config);
    let explanation = checker.explain(&PathBuf::from("src/main.rs"));
    let output = format_content_explanation(&explanation, ExplainFormat::Json);

    // Verify it's valid JSON
    let parsed: serde_json::Value = serde_json::from_str(&output).expect("Invalid JSON output");
    assert!(parsed.get("path").is_some());
    assert!(parsed.get("matched_rule").is_some());
    assert!(parsed.get("effective_limit").is_some());
}

#[test]
fn format_structure_text_output_contains_expected_sections() {
    let config = StructureConfig {
        max_files: Some(10),
        max_dirs: Some(5),
        ..Default::default()
    };

    let checker = crate::checker::StructureChecker::new(&config).unwrap();
    let explanation = checker.explain(&PathBuf::from("src"));
    let output = format_structure_explanation(&explanation, ExplainFormat::Text);

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
    let output = format_structure_explanation(&explanation, ExplainFormat::Json);

    // Verify it's valid JSON
    let parsed: serde_json::Value = serde_json::from_str(&output).expect("Invalid JSON output");
    assert!(parsed.get("path").is_some());
    assert!(parsed.get("matched_rule").is_some());
    assert!(parsed.get("effective_max_files").is_some());
}

#[test]
fn format_structure_with_override_shows_reason() {
    let config = StructureConfig {
        max_files: Some(10),
        max_dirs: Some(5),
        overrides: vec![StructureOverride {
            path: "legacy".to_string(),
            max_files: Some(100),
            max_dirs: None,
            max_depth: None,
            reason: "Legacy directory needs more files".to_string(),
        }],
        ..Default::default()
    };

    let checker = crate::checker::StructureChecker::new(&config).unwrap();
    let explanation = checker.explain(&PathBuf::from("legacy"));
    let output = format_structure_explanation(&explanation, ExplainFormat::Text);

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
    let output = format_structure_explanation(&explanation, ExplainFormat::Text);

    assert!(output.contains("max_files=unlimited"));
}

#[test]
fn format_content_with_override_shows_override_info() {
    let config = Config {
        content: ContentConfig {
            max_lines: 500,
            overrides: vec![ContentOverride {
                path: "legacy.rs".to_string(),
                max_lines: 2000,
                reason: "Legacy code".to_string(),
            }],
            ..Default::default()
        },
        ..Default::default()
    };

    let checker = crate::checker::ThresholdChecker::new(config);
    let explanation = checker.explain(&PathBuf::from("legacy.rs"));
    let output = format_content_explanation(&explanation, ExplainFormat::Text);

    assert!(output.contains("[[content.overrides]]"));
    assert!(output.contains("Legacy code"));
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
                skip_comments: None,
                skip_blank: None,
            }],
            ..Default::default()
        },
        ..Default::default()
    };

    let checker = crate::checker::ThresholdChecker::new(config);
    let explanation = checker.explain(&PathBuf::from("src/main.rs"));
    let output = format_content_explanation(&explanation, ExplainFormat::Text);

    assert!(output.contains("[[content.rules]]"));
    assert!(output.contains("**/*.rs"));
}
