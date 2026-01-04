use crate::error::{ConfigSource, SlocGuardError, span_to_line_col};

// =============================================================================
// Syntax Error Tests
// =============================================================================

#[test]
fn syntax_display_with_origin() {
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
fn syntax_display_without_origin() {
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
fn syntax_display_with_remote_origin() {
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
fn syntax_message() {
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
fn syntax_detail_with_origin() {
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
fn syntax_detail_without_origin() {
    let err = SlocGuardError::Syntax {
        origin: None,
        line: 1,
        column: 1,
        message: "error".to_string(),
    };
    assert!(err.detail().is_none());
}

#[test]
fn syntax_suggestion() {
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
fn span_first_line() {
    let content = "line1 = 1";
    let (line, col) = span_to_line_col(content, 0);
    assert_eq!(line, 1);
    assert_eq!(col, 1);
}

#[test]
fn span_middle_of_first_line() {
    let content = "line1 = 1";
    let (line, col) = span_to_line_col(content, 5);
    assert_eq!(line, 1);
    assert_eq!(col, 6); // 0-indexed position 5 = column 6
}

#[test]
fn span_second_line() {
    let content = "line1 = 1\nline2 = 2";
    let (line, col) = span_to_line_col(content, 10);
    assert_eq!(line, 2);
    assert_eq!(col, 1); // First character of second line
}

#[test]
fn span_third_line_middle() {
    let content = "a\nb\ncdef";
    // Positions: a(0) \n(1) b(2) \n(3) c(4) d(5) e(6) f(7)
    let (line, col) = span_to_line_col(content, 6);
    assert_eq!(line, 3);
    assert_eq!(col, 3); // 'e' is at column 3 of line 3
}

#[test]
fn span_at_newline() {
    let content = "ab\ncd";
    // Position 2 is the newline character
    let (line, col) = span_to_line_col(content, 2);
    assert_eq!(line, 1);
    assert_eq!(col, 3);
}

#[test]
fn span_beyond_content() {
    let content = "abc";
    // Position beyond content length is clamped to content.len()
    let (line, col) = span_to_line_col(content, 100);
    assert_eq!(line, 1);
    // Column = clamped_pos - last_newline + 1 = 3 - 0 + 1 = 4
    assert_eq!(col, 4);
}

#[test]
fn span_empty_content() {
    let content = "";
    let (line, col) = span_to_line_col(content, 0);
    assert_eq!(line, 1);
    assert_eq!(col, 1);
}

#[test]
fn span_multiline_config() {
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

// =============================================================================
// TomlParse Tests
// =============================================================================

#[test]
fn toml_parse_suggestion() {
    let toml_err: std::result::Result<toml::Value, _> = toml::from_str("invalid = [");
    let err = SlocGuardError::TomlParse(toml_err.unwrap_err());
    let suggestion = err.suggestion().unwrap();
    assert!(suggestion.contains("TOML syntax"));
}
