use std::path::PathBuf;

use crate::checker::{ContentRuleMatch, MatchStatus, StructureRuleMatch, WarnAtSource};
use crate::cli::ExplainFormat;
use crate::config::{Config, ContentConfig, ContentRule, StructureConfig, StructureRule};

use super::{format_content_explanation, format_structure_explanation};

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

#[test]
fn explain_structure_rule_matches() {
    let config = StructureConfig {
        max_files: Some(10),
        max_dirs: Some(5),
        rules: vec![StructureRule {
            scope: "src/components/*".to_string(),
            max_files: Some(50),
            max_dirs: Some(10),
            ..Default::default()
        }],
        ..Default::default()
    };

    let checker = crate::checker::StructureChecker::new(&config).unwrap();
    let explanation = checker.explain(&PathBuf::from("src/components/Button"));

    assert!(matches!(
        explanation.matched_rule,
        StructureRuleMatch::Rule { index: 0, pattern, .. } if pattern == "src/components/*"
    ));
    assert_eq!(explanation.effective_max_files, Some(50));
    assert_eq!(explanation.effective_max_dirs, Some(10));
}

#[test]
fn explain_structure_rule_with_reason() {
    let config = StructureConfig {
        max_files: Some(10),
        max_dirs: Some(5),
        rules: vec![StructureRule {
            scope: "src/legacy/*".to_string(),
            max_files: Some(100),
            reason: Some("Legacy directory".to_string()),
            ..Default::default()
        }],
        ..Default::default()
    };

    let checker = crate::checker::StructureChecker::new(&config).unwrap();
    let explanation = checker.explain(&PathBuf::from("src/legacy/module"));

    assert!(matches!(
        &explanation.matched_rule,
        StructureRuleMatch::Rule { index: 0, reason, .. } if *reason == Some("Legacy directory".to_string())
    ));
    assert_eq!(explanation.effective_max_files, Some(100));
    assert_eq!(
        explanation.override_reason,
        Some("Legacy directory".to_string())
    );
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
                scope: "src/*".to_string(),
                max_files: Some(20),
                max_dirs: Some(10),
                ..Default::default()
            },
            StructureRule {
                scope: "test/*".to_string(),
                max_files: Some(30),
                max_dirs: Some(15),
                ..Default::default()
            },
        ],
        ..Default::default()
    };

    let checker = crate::checker::StructureChecker::new(&config).unwrap();

    // Check a path that matches the first rule
    let explanation = checker.explain(&PathBuf::from("src/components"));

    let chain = &explanation.rule_chain;

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

    let checker = crate::checker::ThresholdChecker::new(config).unwrap();
    let explanation = checker.explain(&PathBuf::from("src/main.rs"));
    let output = format_content_explanation(&explanation, ExplainFormat::Text).unwrap();

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

    let checker = crate::checker::ThresholdChecker::new(config).unwrap();
    let explanation = checker.explain(&PathBuf::from("src/main.rs"));
    let output = format_content_explanation(&explanation, ExplainFormat::Json).unwrap();

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
    let output = format_structure_explanation(&explanation, ExplainFormat::Text).unwrap();

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
    let output = format_structure_explanation(&explanation, ExplainFormat::Json).unwrap();

    // Verify it's valid JSON
    let parsed: serde_json::Value = serde_json::from_str(&output).expect("Invalid JSON output");
    assert!(parsed.get("path").is_some());
    assert!(parsed.get("matched_rule").is_some());
    assert!(parsed.get("effective_max_files").is_some());
}

#[test]
fn format_structure_with_rule_reason_shows_reason() {
    let config = StructureConfig {
        max_files: Some(10),
        max_dirs: Some(5),
        rules: vec![StructureRule {
            scope: "legacy/*".to_string(),
            max_files: Some(100),
            reason: Some("Legacy directory needs more files".to_string()),
            ..Default::default()
        }],
        ..Default::default()
    };

    let checker = crate::checker::StructureChecker::new(&config).unwrap();
    let explanation = checker.explain(&PathBuf::from("legacy/module"));
    let output = format_structure_explanation(&explanation, ExplainFormat::Text).unwrap();

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
    let output = format_structure_explanation(&explanation, ExplainFormat::Text).unwrap();

    assert!(output.contains("max_files=unlimited"));
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
    let output = format_content_explanation(&explanation, ExplainFormat::Text).unwrap();

    assert!(output.contains("[[content.rules]]"));
    assert!(output.contains("**/*.rs"));
}

#[test]
fn format_content_with_rule_reason_shows_reason() {
    let config = Config {
        content: ContentConfig {
            max_lines: 500,
            rules: vec![ContentRule {
                pattern: "legacy/**".to_string(),
                max_lines: 2000,
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
    let explanation = checker.explain(&PathBuf::from("legacy/old.rs"));
    let output = format_content_explanation(&explanation, ExplainFormat::Text).unwrap();

    assert!(output.contains("[[content.rules]]"));
    assert!(output.contains("Legacy code"));
}

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

#[test]
fn format_content_text_shows_percentage_for_percentage_source() {
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
    let output = format_content_explanation(&explanation, ExplainFormat::Text).unwrap();

    // Should show percentage
    assert!(output.contains("(90%)"));
}

#[test]
fn format_content_text_shows_absolute_for_absolute_source() {
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
    let output = format_content_explanation(&explanation, ExplainFormat::Text).unwrap();

    // Should show (absolute)
    assert!(output.contains("(absolute)"));
    assert!(output.contains("400 lines"));
}

#[test]
fn explain_non_existent_path_returns_error() {
    use crate::cli::{Cli, ColorChoice, Commands, ExtendsPolicy};

    let args = crate::cli::ExplainArgs {
        path: PathBuf::from("non-existent-path-XYZ"),
        config: None,
        format: ExplainFormat::Text,
    };

    let cli = Cli {
        verbose: 0,
        quiet: false,
        color: ColorChoice::Auto,
        no_config: true,
        no_extends: false,
        extends_policy: ExtendsPolicy::Normal,
        command: Commands::Explain(crate::cli::ExplainArgs {
            path: PathBuf::from("non-existent-path-XYZ"),
            config: None,
            format: ExplainFormat::Text,
        }),
    };

    let result = super::run_explain_impl(&args, &cli);

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("Path not found"));
}
