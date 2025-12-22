//! Tests for content.languages expansion into content.rules.

use std::path::Path;

use crate::config::FileConfigLoader;
use crate::config::loader::ConfigLoader;

use super::mock_fs::MockFileSystem;

#[test]
fn expand_language_rules_creates_glob_pattern() {
    let config_content = r#"
version = "2"

[content]
max_lines = 500

[content.languages.rs]
max_lines = 300
warn_threshold = 0.8
skip_comments = false
skip_blank = false
"#;

    let fs = MockFileSystem::new().with_file("/config.toml", config_content);

    let loader = FileConfigLoader::with_fs(fs);
    let config = loader.load_from_path(Path::new("/config.toml")).unwrap();

    // Language rules should be expanded into content.rules
    assert_eq!(config.content.rules.len(), 1);
    assert_eq!(config.content.rules[0].pattern, "**/*.rs");
    assert_eq!(config.content.rules[0].max_lines, 300);
    assert_eq!(config.content.rules[0].warn_threshold, Some(0.8));
    assert_eq!(config.content.rules[0].skip_comments, Some(false));
    assert_eq!(config.content.rules[0].skip_blank, Some(false));

    // Languages map should be cleared after expansion
    assert!(config.content.languages.is_empty());
}

#[test]
fn expand_language_rules_uses_global_default_for_missing_max_lines() {
    let config_content = r#"
version = "2"

[content]
max_lines = 500

[content.languages.go]
warn_threshold = 0.9
"#;

    let fs = MockFileSystem::new().with_file("/config.toml", config_content);

    let loader = FileConfigLoader::with_fs(fs);
    let config = loader.load_from_path(Path::new("/config.toml")).unwrap();

    // Should use content.max_lines when language rule omits max_lines
    assert_eq!(config.content.rules.len(), 1);
    assert_eq!(config.content.rules[0].pattern, "**/*.go");
    assert_eq!(config.content.rules[0].max_lines, 500);
    assert_eq!(config.content.rules[0].warn_threshold, Some(0.9));
}

#[test]
fn expand_language_rules_inserted_at_head() {
    let config_content = r#"
version = "2"

[content]
max_lines = 500

[content.languages.rs]
max_lines = 300

[[content.rules]]
pattern = "**/*.rs"
max_lines = 600
"#;

    let fs = MockFileSystem::new().with_file("/config.toml", config_content);

    let loader = FileConfigLoader::with_fs(fs);
    let config = loader.load_from_path(Path::new("/config.toml")).unwrap();

    // Expanded language rule at HEAD, explicit rule after
    assert_eq!(config.content.rules.len(), 2);
    assert_eq!(config.content.rules[0].max_lines, 300); // Language rule (HEAD)
    assert_eq!(config.content.rules[1].max_lines, 600); // Explicit rule (after)
}

#[test]
fn explicit_rules_override_language_rules() {
    // This test verifies that since language rules are at HEAD,
    // explicit [[content.rules]] that come after will override them
    // because ThresholdChecker uses "last match wins" semantics.
    let config_content = r#"
version = "2"

[content]
max_lines = 500

[content.languages.rs]
max_lines = 300

[[content.rules]]
pattern = "**/*.rs"
max_lines = 600
"#;

    let fs = MockFileSystem::new().with_file("/config.toml", config_content);

    let loader = FileConfigLoader::with_fs(fs);
    let config = loader.load_from_path(Path::new("/config.toml")).unwrap();

    // When ThresholdChecker checks "test.rs", it will:
    // 1. Match rules[0] (pattern="**/*.rs", max_lines=300) - from language
    // 2. Match rules[1] (pattern="**/*.rs", max_lines=600) - explicit rule
    // Using "last match wins", rules[1] should be used (600 lines)
    // This is tested in threshold_tests.rs
    assert_eq!(config.content.rules.len(), 2);
}

#[test]
fn expand_language_rules_sorted_by_extension() {
    let config_content = r#"
version = "2"

[content]
max_lines = 500

[content.languages.ts]
max_lines = 400

[content.languages.go]
max_lines = 300

[content.languages.rs]
max_lines = 200
"#;

    let fs = MockFileSystem::new().with_file("/config.toml", config_content);

    let loader = FileConfigLoader::with_fs(fs);
    let config = loader.load_from_path(Path::new("/config.toml")).unwrap();

    // Extensions should be sorted alphabetically for deterministic order
    assert_eq!(config.content.rules.len(), 3);
    assert_eq!(config.content.rules[0].pattern, "**/*.go");
    assert_eq!(config.content.rules[1].pattern, "**/*.rs");
    assert_eq!(config.content.rules[2].pattern, "**/*.ts");
}
