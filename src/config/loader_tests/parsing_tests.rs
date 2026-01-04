//! Tests for config parsing, explicit path loading, and error handling.

use std::path::Path;

use crate::config::FileConfigLoader;
use crate::config::loader::ConfigLoader;
use crate::error::{ConfigSource, SlocGuardError};

use super::mock_fs::MockFileSystem;

#[test]
fn load_from_explicit_path() {
    let config_content = r#"
version = "2"

[content]
max_lines = 700
extensions = ["rs", "py"]
"#;

    let fs = MockFileSystem::new().with_file("/custom/path/config.toml", config_content);

    let loader = FileConfigLoader::with_fs(fs);
    let result = loader
        .load_from_path(Path::new("/custom/path/config.toml"))
        .unwrap();

    assert_eq!(result.config.content.max_lines, 700);
    assert_eq!(result.config.content.extensions, vec!["rs", "py"]);
}

#[test]
fn returns_syntax_error_for_invalid_toml_without_extends() {
    // Single-file mode: should return Syntax error with precise location
    let invalid_content = "line1 = 1\nline2 = [";

    let fs = MockFileSystem::new().with_file("/project/.sloc-guard.toml", invalid_content);

    let loader = FileConfigLoader::with_fs(fs);
    let result = loader.load_from_path(Path::new("/project/.sloc-guard.toml"));

    assert!(result.is_err());
    let err = result.unwrap_err();

    // Should be a Syntax error with line/column, not TomlParse
    match err {
        SlocGuardError::Syntax {
            origin,
            line,
            column,
            message,
        } => {
            assert!(origin.is_some());
            // The unclosed array on line 2 is detected at line 2 (1-based)
            assert_eq!(line, 2, "Expected error on line 2 (unclosed array)");
            assert!(
                column >= 1,
                "Column should be valid (1-based), got {column}"
            );
            assert!(!message.is_empty());
        }
        _ => panic!("Expected Syntax error, got: {err:?}"),
    }
}

#[test]
fn syntax_error_includes_file_origin() {
    let invalid_content = "invalid[[[";

    let fs = MockFileSystem::new().with_file("/custom/config.toml", invalid_content);

    let loader = FileConfigLoader::with_fs(fs);
    let result = loader.load_from_path(Path::new("/custom/config.toml"));

    let err = result.unwrap_err();
    match err {
        SlocGuardError::Syntax { origin, .. } => {
            let origin = origin.expect("Should have origin");
            match origin {
                ConfigSource::File { path } => {
                    assert!(path.to_string_lossy().contains("config.toml"));
                }
                _ => panic!("Expected File origin"),
            }
        }
        _ => panic!("Expected Syntax error"),
    }
}

#[test]
fn syntax_error_has_correct_line_number() {
    // Error on line 3
    let invalid_content = r#"version = "2"

[content
max_lines = 500
"#;

    let fs = MockFileSystem::new().with_file("/project/.sloc-guard.toml", invalid_content);

    let loader = FileConfigLoader::with_fs(fs);
    let result = loader.load_from_path(Path::new("/project/.sloc-guard.toml"));

    let err = result.unwrap_err();
    match err {
        SlocGuardError::Syntax { line, column, .. } => {
            // The unclosed bracket on line 3 causes the parser to report error on line 3 or 4
            assert!(line >= 3, "Expected line >= 3, got {line}");
            assert!(
                column >= 1,
                "Column should be valid (1-based), got {column}"
            );
        }
        _ => panic!("Expected Syntax error, got: {err:?}"),
    }
}

#[test]
fn load_without_extends_returns_syntax_error() {
    let invalid_content = "bad = [[[";

    let fs = MockFileSystem::new().with_file("/project/.sloc-guard.toml", invalid_content);

    let loader = FileConfigLoader::with_fs(fs);
    let result = loader.load_from_path_without_extends(Path::new("/project/.sloc-guard.toml"));

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(matches!(err, SlocGuardError::Syntax { .. }));
}

#[test]
fn returns_error_for_nonexistent_explicit_path() {
    let fs = MockFileSystem::new();

    let loader = FileConfigLoader::with_fs(fs);
    let result = loader.load_from_path(Path::new("/does/not/exist.toml"));

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(matches!(err, SlocGuardError::FileAccess { .. }));
}

#[test]
fn syntax_error_includes_remote_origin() {
    // Verify that syntax errors from remote configs include Remote origin
    // This tests the parse_value_with_location path with ConfigSource::Remote
    let invalid_content = "invalid[[[";
    let source = ConfigSource::Remote {
        url: "https://example.com/config.toml".to_string(),
    };

    // Call the internal parsing function via a type alias to verify behavior
    let result: Result<toml::Value, SlocGuardError> = toml::from_str(invalid_content)
        .map_err(|e| SlocGuardError::syntax_from_toml(&e, invalid_content, Some(source.clone())));

    assert!(result.is_err());
    let err = result.unwrap_err();
    match err {
        SlocGuardError::Syntax { origin, line, .. } => {
            let origin = origin.expect("Should have origin");
            match origin {
                ConfigSource::Remote { url } => {
                    assert_eq!(url, "https://example.com/config.toml");
                }
                _ => panic!("Expected Remote origin, got {origin:?}"),
            }
            assert_eq!(line, 1, "Error should be on line 1");
        }
        _ => panic!("Expected Syntax error, got: {err:?}"),
    }
}

// ===== Line number preservation tests =====
// These tests verify that type errors report correct line numbers
// by parsing directly from original content (not via toml::Value re-serialization)

#[test]
fn type_error_has_precise_line_number_single_file() {
    // Type error on line 8 (max_lines expects integer, got string)
    // The blank lines and comments should NOT affect the line number
    let config_content = r#"version = "2"

# This is a comment
[content]

# Another comment
extensions = ["rs"]
max_lines = "not_a_number"
"#;

    let fs = MockFileSystem::new().with_file("/project/.sloc-guard.toml", config_content);

    let loader = FileConfigLoader::with_fs(fs);
    let result = loader.load_from_path(Path::new("/project/.sloc-guard.toml"));

    assert!(result.is_err());
    let err = result.unwrap_err();

    // Check error message contains the correct line number
    let msg = err.to_string();
    assert!(
        msg.contains("line 8"),
        "Expected error on line 8, got: {msg}"
    );
}

#[test]
fn type_error_with_blank_lines_preserved() {
    // Error is on line 10 due to blank lines before it
    let config_content = r#"version = "2"

[scanner]
gitignore = true

[content]
extensions = ["rs"]

# Type error: warn_at expects usize, got negative
warn_at = -900
"#;

    let fs = MockFileSystem::new().with_file("/project/.sloc-guard.toml", config_content);

    let loader = FileConfigLoader::with_fs(fs);
    let result = loader.load_from_path(Path::new("/project/.sloc-guard.toml"));

    assert!(result.is_err());
    let err = result.unwrap_err();

    // Line 10 is where warn_at = -900 is
    let msg = err.to_string();
    assert!(
        msg.contains("line 10"),
        "Expected error on line 10, got: {msg}"
    );
}

#[test]
fn type_error_in_content_rule_has_correct_line() {
    // Error on line 9 (max_lines in rule expects integer)
    let config_content = r#"version = "2"

[content]
extensions = ["rs"]

[[content.rules]]
pattern = "**/*.rs"
max_lines = "bad"
"#;

    let fs = MockFileSystem::new().with_file("/project/.sloc-guard.toml", config_content);

    let loader = FileConfigLoader::with_fs(fs);
    let result = loader.load_from_path(Path::new("/project/.sloc-guard.toml"));

    assert!(result.is_err());
    let err = result.unwrap_err();

    let msg = err.to_string();
    assert!(
        msg.contains("line 8"),
        "Expected error on line 8, got: {msg}"
    );
}

#[test]
fn type_error_with_reset_marker_still_works() {
    // Config with $reset marker - line numbers may shift but should not crash
    let config_content = r#"version = "2"

[scanner]
exclude = ["$reset", "**/build/**"]

[content]
max_lines = "not_a_number"
"#;

    let fs = MockFileSystem::new().with_file("/project/.sloc-guard.toml", config_content);

    let loader = FileConfigLoader::with_fs(fs);
    let result = loader.load_from_path(Path::new("/project/.sloc-guard.toml"));

    assert!(result.is_err());
    let err = result.unwrap_err();

    // With reset markers, we go through Value path - line may differ
    // Just verify it produces a parseable error message
    let msg = err.to_string();
    assert!(
        msg.contains("max_lines") || msg.contains("integer") || msg.contains("string"),
        "Expected type error mentioning field or types, got: {msg}"
    );
}

#[test]
fn line_number_preserved_with_multiline_arrays() {
    // Error on line 12 after multiline array
    let config_content = r#"version = "2"

[scanner]
exclude = [
    "**/target/**",
    "**/vendor/**",
    "**/node_modules/**"
]

[content]
max_lines = "invalid"
"#;

    let fs = MockFileSystem::new().with_file("/project/.sloc-guard.toml", config_content);

    let loader = FileConfigLoader::with_fs(fs);
    let result = loader.load_from_path(Path::new("/project/.sloc-guard.toml"));

    assert!(result.is_err());
    let err = result.unwrap_err();

    let msg = err.to_string();
    assert!(
        msg.contains("line 11"),
        "Expected error on line 11, got: {msg}"
    );
}

#[test]
fn line_number_preserved_in_load_without_extends() {
    // Verify load_from_path_without_extends also preserves line numbers
    let config_content = r#"version = "2"

# Comment
[content]
max_lines = "bad"
"#;

    let fs = MockFileSystem::new().with_file("/project/.sloc-guard.toml", config_content);

    let loader = FileConfigLoader::with_fs(fs);
    let result = loader.load_from_path_without_extends(Path::new("/project/.sloc-guard.toml"));

    assert!(result.is_err());
    let err = result.unwrap_err();

    let msg = err.to_string();
    assert!(
        msg.contains("line 5"),
        "Expected error on line 5, got: {msg}"
    );
}

#[test]
fn parses_full_v2_config() {
    let config_content = r#"
version = "2"

[scanner]
gitignore = true
exclude = ["**/target/**", "**/vendor/**"]

[content]
max_lines = 500
extensions = ["rs", "go"]
skip_comments = true
skip_blank = true
warn_threshold = 0.85

[[content.rules]]
pattern = "**/*.rs"
max_lines = 300
reason = "Rust files"

[[content.rules]]
pattern = "src/legacy.rs"
max_lines = 800
reason = "Legacy code"
"#;

    let fs = MockFileSystem::new().with_file("/config.toml", config_content);

    let loader = FileConfigLoader::with_fs(fs);
    let result = loader.load_from_path(Path::new("/config.toml")).unwrap();
    let config = result.config;

    assert_eq!(config.content.max_lines, 500);
    assert_eq!(config.content.extensions, vec!["rs", "go"]);
    assert!(config.content.skip_comments);
    assert!(config.content.skip_blank);
    assert!((config.content.warn_threshold - 0.85).abs() < f64::EPSILON);

    assert_eq!(config.scanner.exclude, vec!["**/target/**", "**/vendor/**"]);

    assert_eq!(config.content.rules.len(), 2);
    assert_eq!(config.content.rules[0].pattern, "**/*.rs");
    assert_eq!(config.content.rules[0].max_lines, 300);
    assert_eq!(config.content.rules[1].pattern, "src/legacy.rs");
    assert_eq!(config.content.rules[1].max_lines, 800);
    assert_eq!(
        config.content.rules[1].reason,
        Some("Legacy code".to_string())
    );
}
