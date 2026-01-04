//! Tests for config sources explanation (--sources flag).
//!
//! Covers: `ConfigExplanation` construction, inheritance chain tracking,
//! field source attribution, and helper functions for TOML value handling.

use crate::cli::ExplainFormat;
use crate::config::{CheckConfig, Config, ContentConfig, ScannerConfig, StructureConfig};

use super::super::{
    ConfigExplanation, FieldWithSource, KEY_FIELDS, format_config_explanation, format_config_text,
    format_toml_value, get_nested_value,
};

// ============================================================================
// ConfigExplanation construction tests
// ============================================================================

#[test]
fn config_explanation_from_empty_source_chain() {
    use crate::config::LoadResultWithSources;

    let result = LoadResultWithSources {
        config: Config::default(),
        preset_used: None,
        source_chain: vec![],
    };

    let explanation = ConfigExplanation::from_load_result(&result);

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

    let explanation = ConfigExplanation::from_load_result(&result);

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

    let explanation = ConfigExplanation::from_load_result(&result);

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

    let explanation = ConfigExplanation::from_load_result(&result);

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
// Config explanation formatting tests
// ============================================================================

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

    let explanation = ConfigExplanation::from_load_result(&result);
    let output = format_config_text(&explanation);

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

    let explanation = ConfigExplanation::from_load_result(&result);
    let json = format_config_explanation(&explanation, ExplainFormat::Json).unwrap();

    // Parse JSON to validate structure
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert!(parsed.get("chain").is_some());
    assert!(parsed.get("fields").is_some());
    assert!(parsed["chain"].as_array().unwrap().len() == 1);
}

#[test]
fn format_config_explanation_empty_chain() {
    let explanation = ConfigExplanation {
        chain: vec![],
        fields: vec![],
    };

    let output = format_config_text(&explanation);

    assert!(output.contains("No configuration file found"));
    assert!(output.contains("Using defaults"));
}

#[test]
fn format_config_text_with_empty_fields() {
    let explanation = ConfigExplanation {
        chain: vec!["config.toml".to_string()],
        fields: vec![],
    };

    let output = format_config_text(&explanation);

    assert!(output.contains("config.toml"));
    assert!(output.contains("no fields configured"));
}

#[test]
fn format_config_text_groups_fields_by_section() {
    let explanation = ConfigExplanation {
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

    let output = format_config_text(&explanation);

    // Should have section headers
    assert!(output.contains("[content]"));
    assert!(output.contains("[structure]"));
    // Fields should be under their sections
    assert!(output.contains("max_lines = 500"));
    assert!(output.contains("max_files = 25"));
}

// ============================================================================
// TOML helper function tests
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

    let result = get_nested_value(&value, &["content", "max_lines"]);
    assert!(result.is_some());
    assert_eq!(result.unwrap().as_integer(), Some(500));

    let result = get_nested_value(&value, &["structure", "max_files"]);
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

    let result = get_nested_value(&value, &["nonexistent", "field"]);
    assert!(result.is_none());
}

#[test]
#[allow(clippy::many_single_char_names, clippy::approx_constant)]
fn format_toml_value_formats_types_correctly() {
    // String
    let string_val = toml::Value::String("hello".to_string());
    assert_eq!(format_toml_value(&string_val), "\"hello\"");

    // Integer
    let int_val = toml::Value::Integer(42);
    assert_eq!(format_toml_value(&int_val), "42");

    // Float
    let float_val = toml::Value::Float(3.14);
    assert!(format_toml_value(&float_val).starts_with("3.14"));

    // Boolean
    let bool_val = toml::Value::Boolean(true);
    assert_eq!(format_toml_value(&bool_val), "true");

    // Array
    let arr = toml::Value::Array(vec![
        toml::Value::String("a".to_string()),
        toml::Value::String("b".to_string()),
    ]);
    assert_eq!(format_toml_value(&arr), "[\"a\", \"b\"]");

    // Table
    let mut table = toml::map::Map::new();
    table.insert("key".to_string(), toml::Value::Integer(1));
    let table_val = toml::Value::Table(table);
    assert_eq!(format_toml_value(&table_val), "{...}");
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
    for (field_path, path_parts) in KEY_FIELDS {
        let result = get_nested_value(&toml_value, path_parts);
        assert!(
            result.is_some(),
            "KEY_FIELDS entry '{field_path}' (path: {path_parts:?}) not found in Config. \
             This field may have been renamed or removed in model.rs. \
             Update KEY_FIELDS in explain.rs to match the current Config schema."
        );
    }
}
