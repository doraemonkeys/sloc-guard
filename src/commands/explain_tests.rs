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
fn format_content_text_shows_global_percentage_source() {
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

    // Should show percentage and global source
    assert!(output.contains("(from [content], 90%)"));
}

#[test]
fn format_content_text_shows_global_absolute_source() {
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

    // Should show global source with absolute
    assert!(output.contains("(from [content], absolute)"));
    assert!(output.contains("400 lines"));
}

#[test]
fn format_content_text_shows_rule_absolute_source_with_index() {
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
    let output = format_content_explanation(&explanation, ExplainFormat::Text).unwrap();

    // Should show rule source with index and absolute
    assert!(output.contains("(from content.rules[0], absolute)"));
    assert!(output.contains("250 lines"));
}

#[test]
fn format_content_text_shows_rule_percentage_source_with_index() {
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
    let output = format_content_explanation(&explanation, ExplainFormat::Text).unwrap();

    // Should show rule source with index and percentage (300 * 0.8 = 240)
    assert!(output.contains("(from content.rules[0], 80%)"));
    assert!(output.contains("240 lines"));
}

#[test]
fn explain_non_existent_path_returns_error() {
    use crate::cli::{Cli, ColorChoice, Commands, ExtendsPolicy};

    let args = crate::cli::ExplainArgs {
        path: Some(PathBuf::from("non-existent-path-XYZ")),
        config: None,
        sources: false,
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
            path: Some(PathBuf::from("non-existent-path-XYZ")),
            config: None,
            sources: false,
            format: ExplainFormat::Text,
        }),
    };

    let result = super::run_explain_impl(&args, &cli);

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("Path not found"));
}

// ============================================================================
// Tests for explain --sources
// ============================================================================

#[test]
fn config_explanation_from_empty_source_chain() {
    use crate::config::LoadResultWithSources;

    let result = LoadResultWithSources {
        config: Config::default(),
        preset_used: None,
        source_chain: vec![],
    };

    let explanation = super::ConfigExplanation::from_load_result(&result);

    assert!(explanation.chain.is_empty());
    assert!(explanation.fields.is_empty());
}

#[test]
fn config_explanation_from_single_source() {
    use crate::config::{LoadResultWithSources, SourcedConfig};
    use crate::error::ConfigSource;

    let config_value: toml::Value = toml::from_str(
        r#"
        [content]
        max_lines = 400
        extensions = ["rs", "go"]
        "#,
    )
    .unwrap();

    let result = LoadResultWithSources {
        config: Config::default(),
        preset_used: None,
        source_chain: vec![SourcedConfig {
            source: ConfigSource::file("test.toml"),
            value: config_value,
        }],
    };

    let explanation = super::ConfigExplanation::from_load_result(&result);

    assert_eq!(explanation.chain.len(), 1);
    assert!(explanation.chain[0].contains("test.toml"));

    // Check that max_lines field is tracked
    let max_lines_field = explanation
        .fields
        .iter()
        .find(|f| f.field == "content.max_lines");
    assert!(max_lines_field.is_some());
    assert_eq!(max_lines_field.unwrap().value, "400");
    assert!(max_lines_field.unwrap().source.contains("test.toml"));
}

#[test]
fn config_explanation_from_inheritance_chain() {
    use crate::config::{LoadResultWithSources, SourcedConfig};
    use crate::error::ConfigSource;

    // Base config (preset-like)
    let base_value: toml::Value = toml::from_str(
        r#"
        [content]
        max_lines = 600
        extensions = ["rs"]
        skip_comments = true
        "#,
    )
    .unwrap();

    // Child config (overrides max_lines)
    let child_value: toml::Value = toml::from_str(
        r"
        [content]
        max_lines = 400
        ",
    )
    .unwrap();

    let result = LoadResultWithSources {
        config: Config::default(),
        preset_used: Some("rust-strict".to_string()),
        source_chain: vec![
            SourcedConfig {
                source: ConfigSource::preset("rust-strict"),
                value: base_value,
            },
            SourcedConfig {
                source: ConfigSource::file("local.toml"),
                value: child_value,
            },
        ],
    };

    let explanation = super::ConfigExplanation::from_load_result(&result);

    assert_eq!(explanation.chain.len(), 2);
    assert_eq!(explanation.chain[0], "preset:rust-strict");
    assert!(explanation.chain[1].contains("local.toml"));

    // max_lines should come from child (local.toml)
    let max_lines = explanation
        .fields
        .iter()
        .find(|f| f.field == "content.max_lines")
        .unwrap();
    assert_eq!(max_lines.value, "400");
    assert!(max_lines.source.contains("local.toml"));

    // skip_comments should come from base (preset)
    let skip_comments = explanation
        .fields
        .iter()
        .find(|f| f.field == "content.skip_comments")
        .unwrap();
    assert_eq!(skip_comments.value, "true");
    assert!(skip_comments.source.contains("rust-strict"));
}

#[test]
fn format_config_explanation_text() {
    use crate::config::{LoadResultWithSources, SourcedConfig};
    use crate::error::ConfigSource;

    let config_value: toml::Value = toml::from_str(
        r"
        [content]
        max_lines = 500
        [structure]
        max_files = 25
        ",
    )
    .unwrap();

    let result = LoadResultWithSources {
        config: Config::default(),
        preset_used: None,
        source_chain: vec![SourcedConfig {
            source: ConfigSource::file("project.toml"),
            value: config_value,
        }],
    };

    let explanation = super::ConfigExplanation::from_load_result(&result);
    let output = super::format_config_text(&explanation);

    assert!(output.contains("Configuration Source Chain"));
    assert!(output.contains("Inheritance Chain"));
    assert!(output.contains("project.toml"));
    assert!(output.contains("Field Sources"));
    assert!(output.contains("[content]"));
    assert!(output.contains("max_lines = 500"));
    assert!(output.contains("[structure]"));
    assert!(output.contains("max_files = 25"));
}

#[test]
fn format_config_explanation_json() {
    use crate::config::{LoadResultWithSources, SourcedConfig};
    use crate::error::ConfigSource;

    let config_value: toml::Value = toml::from_str(
        r"
        [content]
        max_lines = 300
        ",
    )
    .unwrap();

    let result = LoadResultWithSources {
        config: Config::default(),
        preset_used: None,
        source_chain: vec![SourcedConfig {
            source: ConfigSource::file("test.toml"),
            value: config_value,
        }],
    };

    let explanation = super::ConfigExplanation::from_load_result(&result);
    let json = super::format_config_explanation(&explanation, ExplainFormat::Json).unwrap();

    // Parse JSON to validate structure
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert!(parsed.get("chain").is_some());
    assert!(parsed.get("fields").is_some());
    assert!(parsed["chain"].as_array().unwrap().len() == 1);
}

#[test]
fn format_config_explanation_empty_chain() {
    let explanation = super::ConfigExplanation {
        chain: vec![],
        fields: vec![],
    };

    let output = super::format_config_text(&explanation);

    assert!(output.contains("No configuration file found"));
    assert!(output.contains("Using defaults"));
}

// ============================================================================
// Tests for excluded files
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

#[test]
fn format_content_excluded_file_text() {
    let config = Config {
        content: ContentConfig {
            max_lines: 500,
            exclude: vec!["**/*.gen.rs".to_string()],
            ..Default::default()
        },
        ..Default::default()
    };

    let checker = crate::checker::ThresholdChecker::new(config).unwrap();
    let explanation = checker.explain(&PathBuf::from("src/api.gen.rs"));
    let output = format_content_explanation(&explanation, ExplainFormat::Text).unwrap();

    assert!(output.contains("EXCLUDED"));
    assert!(output.contains("**/*.gen.rs"));
    assert!(output.contains("excluded from SLOC counting"));
}

#[test]
fn format_content_excluded_file_json() {
    let config = Config {
        content: ContentConfig {
            max_lines: 500,
            exclude: vec!["**/*.generated.ts".to_string()],
            ..Default::default()
        },
        ..Default::default()
    };

    let checker = crate::checker::ThresholdChecker::new(config).unwrap();
    let explanation = checker.explain(&PathBuf::from("src/models.generated.ts"));
    let output = format_content_explanation(&explanation, ExplainFormat::Json).unwrap();

    // Verify it's valid JSON
    let parsed: serde_json::Value = serde_json::from_str(&output).expect("Invalid JSON output");
    assert!(parsed.get("is_excluded").is_some());
    assert_eq!(parsed["is_excluded"].as_bool(), Some(true));
    assert!(parsed.get("matched_rule").is_some());
}

// ============================================================================
// Tests for structure with no limits configured
// ============================================================================

#[test]
fn format_structure_with_no_limits_shows_none() {
    let config = StructureConfig {
        max_files: None,
        max_dirs: None,
        max_depth: None,
        ..Default::default()
    };

    let checker = crate::checker::StructureChecker::new(&config).unwrap();
    let explanation = checker.explain(&PathBuf::from("src"));
    let output = format_structure_explanation(&explanation, ExplainFormat::Text).unwrap();

    assert!(output.contains("max_files=none"));
    assert!(output.contains("max_dirs=none"));
    assert!(output.contains("max_depth=none"));
}

// ============================================================================
// Tests for get_nested_value and format_toml_value
// ============================================================================

#[test]
fn get_nested_value_returns_correct_value() {
    let value: toml::Value = toml::from_str(
        r"
        [content]
        max_lines = 500
        [structure]
        max_files = 25
        ",
    )
    .unwrap();

    let result = super::get_nested_value(&value, &["content", "max_lines"]);
    assert!(result.is_some());
    assert_eq!(result.unwrap().as_integer(), Some(500));

    let result = super::get_nested_value(&value, &["structure", "max_files"]);
    assert!(result.is_some());
    assert_eq!(result.unwrap().as_integer(), Some(25));
}

#[test]
fn get_nested_value_returns_none_for_missing_path() {
    let value: toml::Value = toml::from_str(
        r"
        [content]
        max_lines = 500
        ",
    )
    .unwrap();

    let result = super::get_nested_value(&value, &["nonexistent", "field"]);
    assert!(result.is_none());
}

#[test]
#[allow(clippy::many_single_char_names, clippy::approx_constant)]
fn format_toml_value_formats_types_correctly() {
    // String
    let string_val = toml::Value::String("hello".to_string());
    assert_eq!(super::format_toml_value(&string_val), "\"hello\"");

    // Integer
    let int_val = toml::Value::Integer(42);
    assert_eq!(super::format_toml_value(&int_val), "42");

    // Float
    let float_val = toml::Value::Float(3.14);
    assert!(super::format_toml_value(&float_val).starts_with("3.14"));

    // Boolean
    let bool_val = toml::Value::Boolean(true);
    assert_eq!(super::format_toml_value(&bool_val), "true");

    // Array
    let arr = toml::Value::Array(vec![
        toml::Value::String("a".to_string()),
        toml::Value::String("b".to_string()),
    ]);
    assert_eq!(super::format_toml_value(&arr), "[\"a\", \"b\"]");

    // Table
    let mut table = toml::map::Map::new();
    table.insert("key".to_string(), toml::Value::Integer(1));
    let table_val = toml::Value::Table(table);
    assert_eq!(super::format_toml_value(&table_val), "{...}");
}

// ============================================================================
// Tests for format_limit helper
// ============================================================================

#[test]
fn format_limit_helper_values() {
    assert_eq!(super::format_limit(None), "none");
    assert_eq!(super::format_limit(Some(-1)), "unlimited");
    assert_eq!(super::format_limit(Some(0)), "0");
    assert_eq!(super::format_limit(Some(50)), "50");
}

// ============================================================================
// Tests for ConfigExplanation with multiple fields
// ============================================================================

#[test]
fn config_explanation_tracks_multiple_field_types() {
    use crate::config::{LoadResultWithSources, SourcedConfig};
    use crate::error::ConfigSource;

    let config_value: toml::Value = toml::from_str(
        r#"
        [content]
        max_lines = 400
        extensions = ["rs", "go"]
        warn_threshold = 0.9
        skip_comments = true
        skip_blank = false
        
        [structure]
        max_files = 50
        max_dirs = 10
        max_depth = 5
        warn_threshold = 0.85
        
        [scanner]
        gitignore = true
        exclude = ["**/vendor/**"]
        
        [check]
        warnings_as_errors = true
        fail_fast = false
        "#,
    )
    .unwrap();

    let result = LoadResultWithSources {
        config: Config::default(),
        preset_used: None,
        source_chain: vec![SourcedConfig {
            source: ConfigSource::file("full-config.toml"),
            value: config_value,
        }],
    };

    let explanation = super::ConfigExplanation::from_load_result(&result);

    // Should have many fields tracked
    assert!(explanation.fields.len() >= 10);

    // Verify specific fields
    let content_max_lines = explanation
        .fields
        .iter()
        .find(|f| f.field == "content.max_lines");
    assert!(content_max_lines.is_some());
    assert_eq!(content_max_lines.unwrap().value, "400");

    let structure_max_files = explanation
        .fields
        .iter()
        .find(|f| f.field == "structure.max_files");
    assert!(structure_max_files.is_some());
    assert_eq!(structure_max_files.unwrap().value, "50");

    let scanner_gitignore = explanation
        .fields
        .iter()
        .find(|f| f.field == "scanner.gitignore");
    assert!(scanner_gitignore.is_some());
    assert_eq!(scanner_gitignore.unwrap().value, "true");
}

// ============================================================================
// Tests for run_explain_impl with directories
// ============================================================================

#[test]
fn run_explain_impl_with_existing_directory() {
    use crate::cli::{Cli, ColorChoice, Commands, ExtendsPolicy};
    use tempfile::TempDir;

    // Create a temporary directory
    let temp_dir = TempDir::new().unwrap();

    let args = crate::cli::ExplainArgs {
        path: Some(temp_dir.path().to_path_buf()),
        config: None,
        sources: false,
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
            path: Some(temp_dir.path().to_path_buf()),
            config: None,
            sources: false,
            format: ExplainFormat::Text,
        }),
    };

    // With no_config=true and no structure rules, should show "No structure rules" message
    let result = super::run_explain_impl(&args, &cli);
    assert!(result.is_ok());
}

#[test]
fn run_explain_impl_directory_with_structure_config() {
    use crate::cli::{Cli, ColorChoice, Commands, ExtendsPolicy};
    use std::io::Write;
    use tempfile::TempDir;

    // Create a temporary directory with a config file
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join(".sloc-guard.toml");
    let mut config_file = std::fs::File::create(&config_path).unwrap();
    writeln!(
        config_file,
        r#"
version = "2"

[structure]
max_files = 50
max_dirs = 10
"#
    )
    .unwrap();

    let args = crate::cli::ExplainArgs {
        path: Some(temp_dir.path().to_path_buf()),
        config: Some(config_path.clone()),
        sources: false,
        format: ExplainFormat::Text,
    };

    let cli = Cli {
        verbose: 0,
        quiet: false,
        color: ColorChoice::Auto,
        no_config: false,
        no_extends: false,
        extends_policy: ExtendsPolicy::Normal,
        command: Commands::Explain(crate::cli::ExplainArgs {
            path: Some(temp_dir.path().to_path_buf()),
            config: Some(config_path),
            sources: false,
            format: ExplainFormat::Text,
        }),
    };

    let result = super::run_explain_impl(&args, &cli);
    assert!(result.is_ok());
}

#[test]
fn run_explain_impl_directory_with_json_format() {
    use crate::cli::{Cli, ColorChoice, Commands, ExtendsPolicy};
    use std::io::Write;
    use tempfile::TempDir;

    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join(".sloc-guard.toml");
    let mut config_file = std::fs::File::create(&config_path).unwrap();
    writeln!(
        config_file,
        r#"
version = "2"

[structure]
max_files = 25
"#
    )
    .unwrap();

    let args = crate::cli::ExplainArgs {
        path: Some(temp_dir.path().to_path_buf()),
        config: Some(config_path.clone()),
        sources: false,
        format: ExplainFormat::Json,
    };

    let cli = Cli {
        verbose: 0,
        quiet: false,
        color: ColorChoice::Auto,
        no_config: false,
        no_extends: false,
        extends_policy: ExtendsPolicy::Normal,
        command: Commands::Explain(crate::cli::ExplainArgs {
            path: Some(temp_dir.path().to_path_buf()),
            config: Some(config_path),
            sources: false,
            format: ExplainFormat::Json,
        }),
    };

    let result = super::run_explain_impl(&args, &cli);
    assert!(result.is_ok());
}

// ============================================================================
// Tests for run_explain_sources
// ============================================================================

#[test]
fn run_explain_impl_sources_with_no_config() {
    use crate::cli::{Cli, ColorChoice, Commands, ExtendsPolicy};

    let args = crate::cli::ExplainArgs {
        path: None,
        config: None,
        sources: true,
        format: ExplainFormat::Text,
    };

    let cli = Cli {
        verbose: 0,
        quiet: false,
        color: ColorChoice::Auto,
        no_config: true, // --no-config flag
        no_extends: false,
        extends_policy: ExtendsPolicy::Normal,
        command: Commands::Explain(crate::cli::ExplainArgs {
            path: None,
            config: None,
            sources: true,
            format: ExplainFormat::Text,
        }),
    };

    let result = super::run_explain_impl(&args, &cli);
    assert!(result.is_ok());
}

#[test]
fn run_explain_impl_sources_with_config_file() {
    use crate::cli::{Cli, ColorChoice, Commands, ExtendsPolicy};
    use std::io::Write;
    use tempfile::TempDir;

    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.toml");
    let mut config_file = std::fs::File::create(&config_path).unwrap();
    writeln!(
        config_file,
        r#"
version = "2"

[content]
max_lines = 300
"#
    )
    .unwrap();

    let args = crate::cli::ExplainArgs {
        path: None,
        config: Some(config_path.clone()),
        sources: true,
        format: ExplainFormat::Text,
    };

    let cli = Cli {
        verbose: 0,
        quiet: false,
        color: ColorChoice::Auto,
        no_config: false,
        no_extends: false,
        extends_policy: ExtendsPolicy::Normal,
        command: Commands::Explain(crate::cli::ExplainArgs {
            path: None,
            config: Some(config_path),
            sources: true,
            format: ExplainFormat::Text,
        }),
    };

    let result = super::run_explain_impl(&args, &cli);
    assert!(result.is_ok());
}

#[test]
fn run_explain_impl_sources_with_no_extends() {
    use crate::cli::{Cli, ColorChoice, Commands, ExtendsPolicy};
    use std::io::Write;
    use tempfile::TempDir;

    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.toml");
    let mut config_file = std::fs::File::create(&config_path).unwrap();
    writeln!(
        config_file,
        r#"
version = "2"
extends = "preset:rust-strict"

[content]
max_lines = 250
"#
    )
    .unwrap();

    let args = crate::cli::ExplainArgs {
        path: None,
        config: Some(config_path.clone()),
        sources: true,
        format: ExplainFormat::Text,
    };

    let cli = Cli {
        verbose: 0,
        quiet: false,
        color: ColorChoice::Auto,
        no_config: false,
        no_extends: true, // --no-extends flag
        extends_policy: ExtendsPolicy::Normal,
        command: Commands::Explain(crate::cli::ExplainArgs {
            path: None,
            config: Some(config_path),
            sources: true,
            format: ExplainFormat::Text,
        }),
    };

    let result = super::run_explain_impl(&args, &cli);
    assert!(result.is_ok());
}

#[test]
fn run_explain_impl_sources_json_format() {
    use crate::cli::{Cli, ColorChoice, Commands, ExtendsPolicy};
    use std::io::Write;
    use tempfile::TempDir;

    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.toml");
    let mut config_file = std::fs::File::create(&config_path).unwrap();
    writeln!(
        config_file,
        r#"
version = "2"

[content]
max_lines = 500
"#
    )
    .unwrap();

    let args = crate::cli::ExplainArgs {
        path: None,
        config: Some(config_path.clone()),
        sources: true,
        format: ExplainFormat::Json,
    };

    let cli = Cli {
        verbose: 0,
        quiet: false,
        color: ColorChoice::Auto,
        no_config: false,
        no_extends: false,
        extends_policy: ExtendsPolicy::Normal,
        command: Commands::Explain(crate::cli::ExplainArgs {
            path: None,
            config: Some(config_path),
            sources: true,
            format: ExplainFormat::Json,
        }),
    };

    let result = super::run_explain_impl(&args, &cli);
    assert!(result.is_ok());
}

#[test]
fn run_explain_impl_sources_no_config_with_no_extends() {
    use crate::cli::{Cli, ColorChoice, Commands, ExtendsPolicy};

    // When no config is specified and no-extends is true, should return empty result
    let args = crate::cli::ExplainArgs {
        path: None,
        config: None,
        sources: true,
        format: ExplainFormat::Text,
    };

    let cli = Cli {
        verbose: 0,
        quiet: false,
        color: ColorChoice::Auto,
        no_config: false, // Allow config discovery
        no_extends: true, // But don't follow extends
        extends_policy: ExtendsPolicy::Normal,
        command: Commands::Explain(crate::cli::ExplainArgs {
            path: None,
            config: None,
            sources: true,
            format: ExplainFormat::Text,
        }),
    };

    let result = super::run_explain_impl(&args, &cli);
    assert!(result.is_ok());
}

// ============================================================================
// Tests for run_explain with file that uses preset
// ============================================================================

#[test]
fn run_explain_impl_file_with_preset_config() {
    use crate::cli::{Cli, ColorChoice, Commands, ExtendsPolicy};
    use std::io::Write;
    use tempfile::TempDir;

    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.toml");
    let mut config_file = std::fs::File::create(&config_path).unwrap();
    writeln!(
        config_file,
        r#"
version = "2"
extends = "preset:rust-strict"

[content]
max_lines = 400
"#
    )
    .unwrap();

    // Create a test file
    let test_file = temp_dir.path().join("test.rs");
    std::fs::write(&test_file, "fn main() {}").unwrap();

    let args = crate::cli::ExplainArgs {
        path: Some(test_file.clone()),
        config: Some(config_path.clone()),
        sources: false,
        format: ExplainFormat::Text,
    };

    let cli = Cli {
        verbose: 0,
        quiet: false,
        color: ColorChoice::Auto,
        no_config: false,
        no_extends: false,
        extends_policy: ExtendsPolicy::Normal,
        command: Commands::Explain(crate::cli::ExplainArgs {
            path: Some(test_file),
            config: Some(config_path),
            sources: false,
            format: ExplainFormat::Text,
        }),
    };

    let result = super::run_explain_impl(&args, &cli);
    assert!(result.is_ok());
}

// ============================================================================
// Tests for format_config_text edge cases
// ============================================================================

#[test]
fn format_config_text_with_empty_fields() {
    let explanation = super::ConfigExplanation {
        chain: vec!["config.toml".to_string()],
        fields: vec![],
    };

    let output = super::format_config_text(&explanation);

    assert!(output.contains("config.toml"));
    assert!(output.contains("no fields configured"));
}

#[test]
fn format_config_text_groups_fields_by_section() {
    use super::FieldWithSource;

    let explanation = super::ConfigExplanation {
        chain: vec!["config.toml".to_string()],
        fields: vec![
            FieldWithSource {
                field: "content.max_lines".to_string(),
                value: "500".to_string(),
                source: "config.toml".to_string(),
            },
            FieldWithSource {
                field: "content.extensions".to_string(),
                value: "[\"rs\"]".to_string(),
                source: "config.toml".to_string(),
            },
            FieldWithSource {
                field: "structure.max_files".to_string(),
                value: "25".to_string(),
                source: "config.toml".to_string(),
            },
        ],
    };

    let output = super::format_config_text(&explanation);

    // Should have section headers
    assert!(output.contains("[content]"));
    assert!(output.contains("[structure]"));
    // Fields should be under their sections
    assert!(output.contains("max_lines = 500"));
    assert!(output.contains("max_files = 25"));
}

// ============================================================================
// KEY_FIELDS validation test
// ============================================================================

/// Validates that all `KEY_FIELDS` paths correspond to actual fields in Config.
///
/// This test ensures the hardcoded `KEY_FIELDS` list stays in sync with the
/// Config struct. If a field is renamed or removed in `model.rs`, this test
/// will fail, alerting developers to update `KEY_FIELDS` accordingly.
#[test]
fn key_fields_match_config_schema() {
    use crate::config::{CheckConfig, Config, ContentConfig, ScannerConfig, StructureConfig};

    // Create a Config with all tracked fields explicitly set to non-default values
    // so they appear in the serialized TOML output.
    let config = Config {
        content: ContentConfig {
            max_lines: 999,
            extensions: vec!["test".to_string()],
            warn_threshold: 0.5,
            skip_comments: false,
            skip_blank: false,
            ..Default::default()
        },
        structure: StructureConfig {
            max_files: Some(123),
            max_dirs: Some(456),
            max_depth: Some(7),
            warn_threshold: Some(0.75),
            ..Default::default()
        },
        scanner: ScannerConfig {
            gitignore: false,
            exclude: vec!["test/**".to_string()],
        },
        check: CheckConfig {
            warnings_as_errors: true,
            fail_fast: true,
        },
        ..Default::default()
    };

    // Serialize to TOML and parse as Value for path lookup
    let toml_str = toml::to_string(&config).expect("Config should serialize to TOML");
    let toml_value: toml::Value =
        toml::from_str(&toml_str).expect("Serialized TOML should parse back");

    // Validate each KEY_FIELDS entry exists in the serialized config
    for (field_path, path_parts) in super::KEY_FIELDS {
        let result = super::get_nested_value(&toml_value, path_parts);
        assert!(
            result.is_some(),
            "KEY_FIELDS entry '{field_path}' (path: {path_parts:?}) not found in Config. \
             This field may have been renamed or removed in model.rs. \
             Update KEY_FIELDS in explain.rs to match the current Config schema."
        );
    }
}
