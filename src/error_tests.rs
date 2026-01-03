use std::path::PathBuf;

use super::*;

// =============================================================================
// ConfigSource Tests
// =============================================================================

#[test]
fn config_source_file_display() {
    let source = ConfigSource::file(PathBuf::from("config.toml"));
    assert_eq!(source.to_string(), "config.toml");
}

#[test]
fn config_source_remote_display() {
    let source = ConfigSource::remote("https://example.com/config.toml");
    assert_eq!(source.to_string(), "https://example.com/config.toml");
}

#[test]
fn config_source_preset_display() {
    let source = ConfigSource::preset("rust-strict");
    assert_eq!(source.to_string(), "preset:rust-strict");
}

#[test]
fn config_source_constructors() {
    let file = ConfigSource::file("/path/to/config.toml");
    let expected_path: &std::path::Path = std::path::Path::new("/path/to/config.toml");
    assert!(matches!(&file, ConfigSource::File { path } if path == expected_path));

    let remote = ConfigSource::remote("https://example.com");
    assert!(matches!(&remote, ConfigSource::Remote { url } if url == "https://example.com"));

    let preset = ConfigSource::preset("node-strict");
    assert!(matches!(&preset, ConfigSource::Preset { name } if name == "node-strict"));
}

// =============================================================================
// Structured Config Error Tests
// =============================================================================

#[test]
fn circular_extends_error_display() {
    let err = SlocGuardError::CircularExtends {
        chain: vec![
            "a.toml".to_string(),
            "b.toml".to_string(),
            "a.toml".to_string(),
        ],
    };
    let msg = err.to_string();
    assert!(msg.contains("Circular extends"));
    assert!(msg.contains("a.toml"));
    assert!(msg.contains("b.toml"));
}

#[test]
fn circular_extends_error_type() {
    let err = SlocGuardError::CircularExtends {
        chain: vec!["a.toml".to_string()],
    };
    assert_eq!(err.error_type(), "Config");
}

#[test]
fn circular_extends_error_message() {
    let err = SlocGuardError::CircularExtends {
        chain: vec!["a.toml".to_string(), "b.toml".to_string()],
    };
    let msg = err.message();
    assert!(msg.contains("circular extends"));
    assert!(msg.contains("a.toml → b.toml"));
}

#[test]
fn circular_extends_error_detail() {
    let err = SlocGuardError::CircularExtends {
        chain: vec!["a.toml".to_string(), "b.toml".to_string()],
    };
    let detail = err.detail().unwrap();
    assert!(detail.contains("chain:"));
}

#[test]
fn circular_extends_error_suggestion() {
    let err = SlocGuardError::CircularExtends {
        chain: vec!["a.toml".to_string()],
    };
    let suggestion = err.suggestion().unwrap();
    assert!(suggestion.contains("circular reference"));
}

#[test]
fn extends_too_deep_error_display() {
    let err = SlocGuardError::ExtendsTooDeep {
        depth: 11,
        max: 10,
        chain: vec!["config_0.toml".to_string(), "config_1.toml".to_string()],
    };
    let msg = err.to_string();
    assert!(msg.contains("too deep"));
    assert!(msg.contains("11"));
    assert!(msg.contains("10"));
}

#[test]
fn extends_too_deep_error_type() {
    let err = SlocGuardError::ExtendsTooDeep {
        depth: 11,
        max: 10,
        chain: vec![],
    };
    assert_eq!(err.error_type(), "Config");
}

#[test]
fn extends_too_deep_error_message() {
    let err = SlocGuardError::ExtendsTooDeep {
        depth: 15,
        max: 10,
        chain: vec![],
    };
    let msg = err.message();
    assert!(msg.contains("15"));
    assert!(msg.contains("10"));
}

#[test]
fn extends_too_deep_error_detail() {
    let err = SlocGuardError::ExtendsTooDeep {
        depth: 11,
        max: 10,
        chain: vec!["a.toml".to_string(), "b.toml".to_string()],
    };
    let detail = err.detail().unwrap();
    assert!(detail.contains("chain:"));
    assert!(detail.contains("a.toml"));
}

#[test]
fn extends_too_deep_error_suggestion() {
    let err = SlocGuardError::ExtendsTooDeep {
        depth: 11,
        max: 10,
        chain: vec![],
    };
    let suggestion = err.suggestion().unwrap();
    assert!(suggestion.contains("flatten") || suggestion.contains("presets"));
}

#[test]
fn extends_resolution_error_display() {
    let err = SlocGuardError::ExtendsResolution {
        path: "../base.toml".to_string(),
        base: "remote config".to_string(),
    };
    let msg = err.to_string();
    assert!(msg.contains("../base.toml"));
    assert!(msg.contains("remote config"));
}

#[test]
fn extends_resolution_error_type() {
    let err = SlocGuardError::ExtendsResolution {
        path: "relative.toml".to_string(),
        base: "https://example.com".to_string(),
    };
    assert_eq!(err.error_type(), "Config");
}

#[test]
fn extends_resolution_error_message() {
    let err = SlocGuardError::ExtendsResolution {
        path: "../base.toml".to_string(),
        base: "remote config".to_string(),
    };
    let msg = err.message();
    assert!(msg.contains("../base.toml"));
    assert!(msg.contains("remote config"));
}

#[test]
fn extends_resolution_error_suggestion() {
    let err = SlocGuardError::ExtendsResolution {
        path: "../base.toml".to_string(),
        base: "remote config".to_string(),
    };
    let suggestion = err.suggestion().unwrap();
    assert!(suggestion.contains("absolute path") || suggestion.contains("relative path"));
}

#[test]
fn type_mismatch_error_display() {
    let err = SlocGuardError::TypeMismatch {
        field: "content.max_lines".to_string(),
        expected: "integer".to_string(),
        actual: "string".to_string(),
        origin: Some(ConfigSource::file("config.toml")),
    };
    let msg = err.to_string();
    assert!(msg.contains("content.max_lines"));
    assert!(msg.contains("integer"));
    assert!(msg.contains("string"));
    assert!(msg.contains("config.toml"));
}

#[test]
fn type_mismatch_error_without_origin() {
    let err = SlocGuardError::TypeMismatch {
        field: "content.max_lines".to_string(),
        expected: "integer".to_string(),
        actual: "string".to_string(),
        origin: None,
    };
    let msg = err.to_string();
    assert!(msg.contains("content.max_lines"));
    assert!(!msg.contains("(in"));
}

#[test]
fn type_mismatch_error_type() {
    let err = SlocGuardError::TypeMismatch {
        field: "field".to_string(),
        expected: "type".to_string(),
        actual: "other".to_string(),
        origin: None,
    };
    assert_eq!(err.error_type(), "Config");
}

#[test]
fn type_mismatch_error_message() {
    let err = SlocGuardError::TypeMismatch {
        field: "content.warn_threshold".to_string(),
        expected: "float".to_string(),
        actual: "boolean".to_string(),
        origin: None,
    };
    let msg = err.message();
    assert!(msg.contains("content.warn_threshold"));
    assert!(msg.contains("float"));
    assert!(msg.contains("boolean"));
}

#[test]
fn type_mismatch_error_detail_with_origin() {
    let err = SlocGuardError::TypeMismatch {
        field: "field".to_string(),
        expected: "type".to_string(),
        actual: "other".to_string(),
        origin: Some(ConfigSource::preset("rust-strict")),
    };
    let detail = err.detail().unwrap();
    assert!(detail.contains("preset:rust-strict"));
}

#[test]
fn type_mismatch_error_suggestion() {
    let err = SlocGuardError::TypeMismatch {
        field: "field".to_string(),
        expected: "type".to_string(),
        actual: "other".to_string(),
        origin: None,
    };
    let suggestion = err.suggestion().unwrap();
    assert!(suggestion.contains("type") || suggestion.contains("documentation"));
}

#[test]
fn semantic_error_display() {
    let err = SlocGuardError::Semantic {
        field: "content.warn_threshold".to_string(),
        message: "must be between 0.0 and 1.0".to_string(),
        origin: Some(ConfigSource::remote("https://example.com/config.toml")),
        suggestion: Some("Use a value like 0.8 for 80% warning threshold".to_string()),
    };
    let msg = err.to_string();
    assert!(msg.contains("content.warn_threshold"));
    assert!(msg.contains("must be between 0.0 and 1.0"));
    assert!(msg.contains("https://example.com/config.toml"));
}

#[test]
fn semantic_error_without_origin() {
    let err = SlocGuardError::Semantic {
        field: "field".to_string(),
        message: "invalid value".to_string(),
        origin: None,
        suggestion: None,
    };
    let msg = err.to_string();
    assert!(!msg.contains("(in"));
}

#[test]
fn semantic_error_type() {
    let err = SlocGuardError::Semantic {
        field: "field".to_string(),
        message: "error".to_string(),
        origin: None,
        suggestion: None,
    };
    assert_eq!(err.error_type(), "Config");
}

#[test]
fn semantic_error_message() {
    let err = SlocGuardError::Semantic {
        field: "structure.max_depth".to_string(),
        message: "cannot be negative".to_string(),
        origin: None,
        suggestion: None,
    };
    let msg = err.message();
    assert!(msg.contains("structure.max_depth"));
    assert!(msg.contains("cannot be negative"));
}

#[test]
fn semantic_error_suggestion_from_field() {
    let err = SlocGuardError::Semantic {
        field: "field".to_string(),
        message: "error".to_string(),
        origin: None,
        suggestion: Some("Try this instead".to_string()),
    };
    let suggestion = err.suggestion().unwrap();
    assert_eq!(suggestion, "Try this instead");
}

#[test]
fn semantic_error_suggestion_none() {
    let err = SlocGuardError::Semantic {
        field: "field".to_string(),
        message: "error".to_string(),
        origin: None,
        suggestion: None,
    };
    assert!(err.suggestion().is_none());
}

// =============================================================================
// Syntax Error Tests
// =============================================================================

#[test]
fn syntax_error_display_with_origin() {
    let err = SlocGuardError::Syntax {
        origin: Some(ConfigSource::file("config.toml")),
        line: 3,
        column: 11,
        message: "unclosed array".to_string(),
    };
    let msg = err.to_string();
    assert!(msg.contains("line 3"));
    assert!(msg.contains("column 11"));
    assert!(msg.contains("config.toml"));
    assert!(msg.contains("unclosed array"));
}

#[test]
fn syntax_error_display_without_origin() {
    let err = SlocGuardError::Syntax {
        origin: None,
        line: 5,
        column: 1,
        message: "unexpected character".to_string(),
    };
    let msg = err.to_string();
    assert!(msg.contains("line 5"));
    assert!(msg.contains("column 1"));
    assert!(!msg.contains(" (in ")); // No origin suffix
    assert!(msg.contains("unexpected character"));
}

#[test]
fn syntax_error_display_with_remote_origin() {
    let err = SlocGuardError::Syntax {
        origin: Some(ConfigSource::remote("https://example.com/config.toml")),
        line: 10,
        column: 5,
        message: "invalid key".to_string(),
    };
    let msg = err.to_string();
    assert!(msg.contains("https://example.com/config.toml"));
}

#[test]
fn syntax_error_type() {
    let err = SlocGuardError::Syntax {
        origin: None,
        line: 1,
        column: 1,
        message: "error".to_string(),
    };
    assert_eq!(err.error_type(), "Config");
}

#[test]
fn syntax_error_message() {
    let err = SlocGuardError::Syntax {
        origin: None,
        line: 7,
        column: 15,
        message: "expected '='".to_string(),
    };
    let msg = err.message();
    assert!(msg.contains("line 7"));
    assert!(msg.contains("column 15"));
    assert!(msg.contains("expected '='"));
}

#[test]
fn syntax_error_detail_with_origin() {
    let err = SlocGuardError::Syntax {
        origin: Some(ConfigSource::file("local.toml")),
        line: 1,
        column: 1,
        message: "error".to_string(),
    };
    let detail = err.detail().unwrap();
    assert!(detail.contains("local.toml"));
}

#[test]
fn syntax_error_detail_without_origin() {
    let err = SlocGuardError::Syntax {
        origin: None,
        line: 1,
        column: 1,
        message: "error".to_string(),
    };
    assert!(err.detail().is_none());
}

#[test]
fn syntax_error_suggestion() {
    let err = SlocGuardError::Syntax {
        origin: None,
        line: 1,
        column: 1,
        message: "error".to_string(),
    };
    let suggestion = err.suggestion().unwrap();
    assert!(suggestion.contains("TOML syntax"));
}

// =============================================================================
// span_to_line_col Tests
// =============================================================================

#[test]
fn span_to_line_col_first_line() {
    let content = "line1 = 1";
    let (line, col) = span_to_line_col(content, 0);
    assert_eq!(line, 1);
    assert_eq!(col, 1);
}

#[test]
fn span_to_line_col_middle_of_first_line() {
    let content = "line1 = 1";
    let (line, col) = span_to_line_col(content, 5);
    assert_eq!(line, 1);
    assert_eq!(col, 6); // 0-indexed position 5 = column 6
}

#[test]
fn span_to_line_col_second_line() {
    let content = "line1 = 1\nline2 = 2";
    let (line, col) = span_to_line_col(content, 10);
    assert_eq!(line, 2);
    assert_eq!(col, 1); // First character of second line
}

#[test]
fn span_to_line_col_third_line_middle() {
    let content = "a\nb\ncdef";
    // Positions: a(0) \n(1) b(2) \n(3) c(4) d(5) e(6) f(7)
    let (line, col) = span_to_line_col(content, 6);
    assert_eq!(line, 3);
    assert_eq!(col, 3); // 'e' is at column 3 of line 3
}

#[test]
fn span_to_line_col_at_newline() {
    let content = "ab\ncd";
    // Position 2 is the newline character
    let (line, col) = span_to_line_col(content, 2);
    assert_eq!(line, 1);
    assert_eq!(col, 3);
}

#[test]
fn span_to_line_col_beyond_content() {
    let content = "abc";
    // Position beyond content length is clamped to content.len()
    let (line, col) = span_to_line_col(content, 100);
    assert_eq!(line, 1);
    // Column = clamped_pos - last_newline + 1 = 3 - 0 + 1 = 4
    assert_eq!(col, 4);
}

#[test]
fn span_to_line_col_empty_content() {
    let content = "";
    let (line, col) = span_to_line_col(content, 0);
    assert_eq!(line, 1);
    assert_eq!(col, 1);
}

#[test]
fn span_to_line_col_multiline_config() {
    let content = r#"version = "2"

[content]
max_lines = [
"#;
    // The error might be at the unclosed bracket
    // Line 4 starts at position after "max_lines = "
    assert_eq!(content.lines().count(), 4);

    // Find position of '[' on line 4
    let pos = content.find("max_lines = [").unwrap() + "max_lines = ".len();
    let (line, col) = span_to_line_col(content, pos);
    assert_eq!(line, 4);
    assert_eq!(col, 13); // Position of '['
}

// =============================================================================
// syntax_from_toml Tests
// =============================================================================

#[test]
fn syntax_from_toml_extracts_location() {
    let content = "line1 = 1\nline2 = [\nline3";
    let err = toml::from_str::<toml::Value>(content).unwrap_err();
    let sloc_err =
        SlocGuardError::syntax_from_toml(&err, content, Some(ConfigSource::file("test.toml")));

    if let SlocGuardError::Syntax {
        origin,
        line,
        message,
        ..
    } = &sloc_err
    {
        assert!(origin.is_some());
        assert!(*line >= 2); // Error is on line 2 or later (unclosed array)
        assert!(!message.is_empty());
    } else {
        panic!("Expected Syntax error");
    }
}

#[test]
fn syntax_from_toml_without_origin() {
    let content = "invalid[[[";
    let err = toml::from_str::<toml::Value>(content).unwrap_err();
    let sloc_err = SlocGuardError::syntax_from_toml(&err, content, None);

    if let SlocGuardError::Syntax { origin, .. } = &sloc_err {
        assert!(origin.is_none());
    } else {
        panic!("Expected Syntax error");
    }
}

#[test]
fn error_display_config() {
    let err = SlocGuardError::Config("invalid threshold".to_string());
    assert_eq!(err.to_string(), "Configuration error: invalid threshold");
}

#[test]
fn error_display_file_read() {
    let err = SlocGuardError::FileAccess {
        path: PathBuf::from("test.rs"),
        source: std::io::Error::new(std::io::ErrorKind::NotFound, "file not found"),
    };
    assert!(err.to_string().contains("test.rs"));
}

#[test]
fn error_display_git() {
    let err = SlocGuardError::Git("Failed to get git index".to_string());
    assert_eq!(err.to_string(), "Git error: Failed to get git index");
}

#[test]
fn error_display_git_repo_not_found() {
    let err = SlocGuardError::GitRepoNotFound("not a git repository".to_string());
    assert_eq!(
        err.to_string(),
        "Not a git repository: not a git repository"
    );
}

#[test]
fn error_type_returns_correct_type() {
    assert_eq!(
        SlocGuardError::Config("test".to_string()).error_type(),
        "Config"
    );
    assert_eq!(
        SlocGuardError::FileAccess {
            path: PathBuf::from("test.rs"),
            source: std::io::Error::new(std::io::ErrorKind::NotFound, "not found"),
        }
        .error_type(),
        "FileAccess"
    );
    assert_eq!(SlocGuardError::Git("test".to_string()).error_type(), "Git");
    assert_eq!(
        SlocGuardError::GitRepoNotFound("test".to_string()).error_type(),
        "Git"
    );
    assert_eq!(
        SlocGuardError::from(std::io::Error::other("test")).error_type(),
        "IO"
    );
}

#[test]
fn error_message_extracts_message() {
    let err = SlocGuardError::Config("invalid config".to_string());
    assert_eq!(err.message(), "invalid config");

    let err = SlocGuardError::Git("git error".to_string());
    assert_eq!(err.message(), "git error");
}

#[test]
fn error_message_file_read_includes_error_kind() {
    let err = SlocGuardError::FileAccess {
        path: PathBuf::from("test.rs"),
        source: std::io::Error::new(std::io::ErrorKind::NotFound, "not found"),
    };
    let message = err.message();
    assert!(message.contains("test.rs"));
    // Error kind format varies by platform (NotFound vs "not found")
    // Just verify the message is longer than just the path
    assert!(message.len() > "test.rs".len());
}

#[test]
fn error_message_invalid_pattern_includes_glob_error() {
    let glob_err = globset::Glob::new("[invalid").unwrap_err();
    let err = SlocGuardError::InvalidPattern {
        pattern: "[invalid".to_string(),
        source: glob_err,
    };
    let message = err.message();
    assert!(message.contains("[invalid"));
    // Glob error message should be included
    assert!(message.len() > "[invalid".len());
}

#[test]
fn error_detail_returns_source_info() {
    let err = SlocGuardError::Config("test".to_string());
    assert!(err.detail().is_none());

    let err = SlocGuardError::FileAccess {
        path: PathBuf::from("test.rs"),
        source: std::io::Error::new(std::io::ErrorKind::NotFound, "file not found"),
    };
    let detail = err.detail().unwrap();
    assert!(detail.contains("file not found"));
    // Error kind format varies by platform, just check it contains some error info
    assert!(!detail.is_empty());

    let err = SlocGuardError::RemoteConfigHashMismatch {
        url: "https://example.com".to_string(),
        expected: "abc123".to_string(),
        actual: "def456".to_string(),
    };
    let detail = err.detail().unwrap();
    assert!(detail.contains("abc123"));
    assert!(detail.contains("def456"));
}

#[test]
fn suggestion_config_error() {
    let err = SlocGuardError::Config("invalid threshold".to_string());
    let suggestion = err.suggestion().unwrap();
    assert!(suggestion.contains("config file format"));
}

#[test]
fn suggestion_file_read_not_found() {
    let err = SlocGuardError::FileAccess {
        path: PathBuf::from("missing.rs"),
        source: std::io::Error::new(std::io::ErrorKind::NotFound, "not found"),
    };
    let suggestion = err.suggestion().unwrap();
    assert!(suggestion.contains("file path exists"));
}

#[test]
fn suggestion_file_read_permission_denied() {
    let err = SlocGuardError::FileAccess {
        path: PathBuf::from("protected.rs"),
        source: std::io::Error::new(std::io::ErrorKind::PermissionDenied, "access denied"),
    };
    let suggestion = err.suggestion().unwrap();
    assert!(suggestion.contains("permissions"));
}

#[test]
fn suggestion_file_read_other_error_has_none() {
    let err = SlocGuardError::FileAccess {
        path: PathBuf::from("unknown.rs"),
        source: std::io::Error::other("unknown error"),
    };
    // Other IO errors may not have specific suggestions
    assert!(err.suggestion().is_none());
}

#[test]
fn suggestion_invalid_pattern() {
    let glob_err = globset::Glob::new("[invalid").unwrap_err();
    let err = SlocGuardError::InvalidPattern {
        pattern: "[invalid".to_string(),
        source: glob_err,
    };
    let suggestion = err.suggestion().unwrap();
    assert!(suggestion.contains("glob pattern syntax"));
}

#[test]
fn suggestion_io_error_not_found() {
    let err = SlocGuardError::from(std::io::Error::new(
        std::io::ErrorKind::NotFound,
        "not found",
    ));
    let suggestion = err.suggestion().unwrap();
    assert!(suggestion.contains("file path exists"));
}

#[test]
fn suggestion_io_error_permission_denied() {
    let err = SlocGuardError::from(std::io::Error::new(
        std::io::ErrorKind::PermissionDenied,
        "denied",
    ));
    let suggestion = err.suggestion().unwrap();
    assert!(suggestion.contains("permissions"));
}

#[test]
fn suggestion_io_error_other_has_none() {
    let err = SlocGuardError::from(std::io::Error::other("custom error"));
    assert!(err.suggestion().is_none());
}

#[test]
fn suggestion_toml_parse() {
    let toml_err: std::result::Result<toml::Value, _> = toml::from_str("invalid = [");
    let err = SlocGuardError::TomlParse(toml_err.unwrap_err());
    let suggestion = err.suggestion().unwrap();
    assert!(suggestion.contains("TOML syntax"));
}

#[test]
fn suggestion_json_serialize() {
    // Create a true serialization error using a map with non-string keys
    use std::collections::HashMap;
    let mut map: HashMap<Vec<u8>, i32> = HashMap::new();
    map.insert(vec![1, 2, 3], 42);
    let json_err = serde_json::to_string(&map).unwrap_err();
    let err = SlocGuardError::JsonSerialize(json_err);
    let suggestion = err.suggestion().unwrap();
    assert!(suggestion.contains("non-serializable"));
}

#[test]
fn suggestion_git_error() {
    let err = SlocGuardError::Git("failed to read index".to_string());
    let suggestion = err.suggestion().unwrap();
    assert!(suggestion.contains("git is installed"));
}

#[test]
fn suggestion_git_repo_not_found() {
    let err = SlocGuardError::GitRepoNotFound("/some/path".to_string());
    let suggestion = err.suggestion().unwrap();
    assert!(suggestion.contains("git init"));
}

#[test]
fn suggestion_remote_config_hash_mismatch() {
    let err = SlocGuardError::RemoteConfigHashMismatch {
        url: "https://example.com/config.toml".to_string(),
        expected: "abc123".to_string(),
        actual: "def456".to_string(),
    };
    let suggestion = err.suggestion().unwrap();
    assert!(suggestion.contains("extends_sha256"));
}

#[test]
fn suggestion_io_timeout() {
    let err = SlocGuardError::from(std::io::Error::new(std::io::ErrorKind::TimedOut, "timeout"));
    let suggestion = err.suggestion().unwrap();
    assert!(suggestion.contains("network connectivity"));
}

#[test]
fn suggestion_io_connection_refused() {
    let err = SlocGuardError::from(std::io::Error::new(
        std::io::ErrorKind::ConnectionRefused,
        "refused",
    ));
    let suggestion = err.suggestion().unwrap();
    assert!(suggestion.contains("remote server"));
}

#[test]
fn suggestion_io_connection_reset() {
    let err = SlocGuardError::from(std::io::Error::new(
        std::io::ErrorKind::ConnectionReset,
        "reset",
    ));
    let suggestion = err.suggestion().unwrap();
    assert!(suggestion.contains("remote server"));
}

#[test]
fn suggestion_io_invalid_data() {
    let err = SlocGuardError::from(std::io::Error::new(
        std::io::ErrorKind::InvalidData,
        "corrupted",
    ));
    let suggestion = err.suggestion().unwrap();
    assert!(suggestion.contains("corrupted"));
}

#[test]
fn suggestion_file_read_invalid_data() {
    let err = SlocGuardError::FileAccess {
        path: PathBuf::from("corrupted.rs"),
        source: std::io::Error::new(std::io::ErrorKind::InvalidData, "corrupted"),
    };
    let suggestion = err.suggestion().unwrap();
    assert!(suggestion.contains("corrupted"));
}

// Tests for IO error context enrichment (Task 15.3)

#[test]
fn io_with_path_includes_path_in_message() {
    let err = SlocGuardError::io_with_path(
        std::io::Error::new(std::io::ErrorKind::NotFound, "not found"),
        PathBuf::from("config.toml"),
    );
    let message = err.message();
    assert!(message.contains("config.toml"));
    assert!(message.contains("not found"));
}

#[test]
fn io_with_context_includes_path_and_operation() {
    let err = SlocGuardError::io_with_context(
        std::io::Error::new(std::io::ErrorKind::PermissionDenied, "denied"),
        PathBuf::from("secret.key"),
        "reading",
    );
    let message = err.message();
    assert!(message.contains("secret.key"));
    assert!(message.contains("reading"));
    assert!(message.contains("denied"));
}

#[test]
fn io_without_context_shows_error_only() {
    let err = SlocGuardError::from(std::io::Error::other("generic error"));
    let message = err.message();
    assert_eq!(message, "generic error");
}

#[test]
fn io_detail_includes_path_operation_and_kind() {
    let err = SlocGuardError::io_with_context(
        std::io::Error::new(std::io::ErrorKind::NotFound, "file missing"),
        PathBuf::from("data.json"),
        "opening",
    );
    let detail = err.detail().unwrap();
    assert!(detail.contains("data.json"));
    assert!(detail.contains("opening"));
    assert!(detail.contains("file missing"));
    // Error kind is included but format varies by platform
    assert!(detail.len() > "opening 'data.json': file missing".len());
}

#[test]
fn io_detail_without_context_shows_kind() {
    let err = SlocGuardError::from(std::io::Error::new(
        std::io::ErrorKind::PermissionDenied,
        "access denied",
    ));
    let detail = err.detail().unwrap();
    assert!(detail.contains("access denied"));
    // Error kind is included but format varies by platform
    assert!(detail.len() > "access denied".len());
}

#[test]
fn io_display_with_full_context() {
    let err = SlocGuardError::io_with_context(
        std::io::Error::new(std::io::ErrorKind::NotFound, "missing"),
        PathBuf::from("cache.json"),
        "writing",
    );
    let display = err.to_string();
    assert!(display.contains("IO error"));
    assert!(display.contains("cache.json"));
    assert!(display.contains("writing"));
}

#[test]
fn io_display_with_path_only() {
    let err = SlocGuardError::io_with_path(
        std::io::Error::new(std::io::ErrorKind::NotFound, "missing"),
        PathBuf::from("data.txt"),
    );
    let display = err.to_string();
    assert!(display.contains("data.txt"));
    assert!(display.contains("missing"));
}

#[test]
fn io_from_conversion_preserves_error() {
    let io_err = std::io::Error::new(std::io::ErrorKind::InvalidInput, "bad input");
    let err: SlocGuardError = io_err.into();
    assert_eq!(err.error_type(), "IO");
    assert!(err.message().contains("bad input"));
}
