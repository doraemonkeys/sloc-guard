use super::*;

fn rust_syntax() -> CommentSyntax {
    CommentSyntax::new(vec!["//", "///", "//!"], vec![("/*", "*/")])
}

fn python_syntax() -> CommentSyntax {
    CommentSyntax::new(vec!["#"], vec![("'''", "'''"), ("\"\"\"", "\"\"\"")])
}

#[test]
fn detect_rust_single_line_comment() {
    let syntax = rust_syntax();
    let detector = CommentDetector::new(&syntax);

    // Note: is_single_line_comment expects pre-trimmed input
    assert!(detector.is_single_line_comment("// comment"));
    assert!(detector.is_single_line_comment("// indented comment"));
    assert!(detector.is_single_line_comment("/// doc comment"));
    assert!(detector.is_single_line_comment("//! module doc"));
    assert!(!detector.is_single_line_comment("let x = 1; // trailing"));
}

#[test]
fn detect_python_single_line_comment() {
    let syntax = python_syntax();
    let detector = CommentDetector::new(&syntax);

    // Note: is_single_line_comment expects pre-trimmed input
    assert!(detector.is_single_line_comment("# comment"));
    assert!(detector.is_single_line_comment("# indented"));
    assert!(!detector.is_single_line_comment("x = 1  # trailing"));
}

#[test]
fn detect_multi_line_comment_start() {
    let syntax = rust_syntax();
    let detector = CommentDetector::new(&syntax);

    let result = detector.find_multi_line_start("/* start of comment");
    assert!(result.is_some());
    assert_eq!(result.unwrap(), ("/*", "*/"));

    assert!(detector.find_multi_line_start("no comment here").is_none());
}

#[test]
fn detect_python_docstring_start() {
    let syntax = python_syntax();
    let detector = CommentDetector::new(&syntax);

    let result = detector.find_multi_line_start("'''docstring");
    assert!(result.is_some());

    let result = detector.find_multi_line_start("\"\"\"docstring");
    assert!(result.is_some());
}

#[test]
fn detect_multi_line_comment_end() {
    let syntax = rust_syntax();
    let detector = CommentDetector::new(&syntax);

    assert!(detector.contains_multi_line_end("end of comment */", "*/"));
    assert!(!detector.contains_multi_line_end("still in comment", "*/"));
}

#[test]
fn comment_marker_inside_string_is_not_detected() {
    let syntax = rust_syntax();
    let detector = CommentDetector::new(&syntax);

    // Glob pattern in string: "src/generated/**" contains /* but should NOT be detected
    assert!(
        detector
            .find_multi_line_start(r#"pattern: "src/generated/**".to_string(),"#)
            .is_none()
    );

    // Another example with path containing /*
    assert!(
        detector
            .find_multi_line_start(r#"let path = "foo/*bar";"#)
            .is_none()
    );

    // Comment marker NOT in string should still be detected
    assert!(detector.find_multi_line_start("/* real comment").is_some());
    assert!(
        detector
            .find_multi_line_start("let x = 1; /* comment")
            .is_some()
    );

    // Comment end inside string should NOT be detected
    assert!(!detector.contains_multi_line_end(r#"let s = "*/";"#, "*/"));

    // Real comment end should be detected
    assert!(detector.contains_multi_line_end("end of comment */", "*/"));
}

#[test]
fn escaped_quote_inside_string_handled() {
    let syntax = rust_syntax();
    let detector = CommentDetector::new(&syntax);

    // String with escaped quote: "foo\"/*bar" - the /* is still inside the string
    assert!(
        detector
            .find_multi_line_start(r#"let s = "foo\"/*bar";"#)
            .is_none()
    );

    // After the string ends, /* should be detected
    assert!(
        detector
            .find_multi_line_start(r#"let s = "foo"; /* comment"#)
            .is_some()
    );
}

#[test]
fn char_literal_with_comment_marker() {
    let syntax = rust_syntax();
    let detector = CommentDetector::new(&syntax);

    // Very unlikely but handles edge case: '/' followed by '*'
    // This is not a real scenario but tests char literal handling
    assert!(
        detector
            .find_multi_line_start(r"let c = '/'; /* comment")
            .is_some()
    );
}

// =============================================================================
// Additional edge case tests
// =============================================================================

#[test]
fn escaped_backslash_before_quote() {
    let syntax = rust_syntax();
    let detector = CommentDetector::new(&syntax);

    // "foo\\" - backslash is escaped, so the quote ends the string
    // The /* after should be detected
    assert!(
        detector
            .find_multi_line_start(r#"let s = "foo\\"; /* comment"#)
            .is_some()
    );

    // "foo\" - escaped quote, string continues
    // The /* is still inside the string
    assert!(
        detector
            .find_multi_line_start(r#"let s = "foo\"/*bar";"#)
            .is_none()
    );
}

#[test]
fn multiple_strings_on_one_line() {
    let syntax = rust_syntax();
    let detector = CommentDetector::new(&syntax);

    // Comment between two strings
    assert!(
        detector
            .find_multi_line_start(r#""first" /* comment */ "second""#)
            .is_some()
    );

    // Comment marker in first string, real comment after second string
    assert!(
        detector
            .find_multi_line_start(r#""/*" "bar" /* real"#)
            .is_some()
    );

    // Both strings contain /*, no real comment
    assert!(detector.find_multi_line_start(r#""/*" "/*""#).is_none());
}

#[test]
fn string_immediately_followed_by_comment() {
    let syntax = rust_syntax();
    let detector = CommentDetector::new(&syntax);

    // No space between string and comment
    assert!(
        detector
            .find_multi_line_start(r#""foo"/* comment */"#)
            .is_some()
    );

    // Empty string followed by comment
    assert!(detector.find_multi_line_start(r#"""/* comment"#).is_some());
}

#[test]
fn comment_end_marker_in_string() {
    let syntax = rust_syntax();
    let detector = CommentDetector::new(&syntax);

    // */ inside string should NOT be detected as comment end
    assert!(!detector.contains_multi_line_end(r#"let s = "*/";"#, "*/"));

    // */ inside string, then real */ outside
    assert!(detector.contains_multi_line_end(r#"let s = "*/"; */"#, "*/"));

    // Glob pattern with */ inside: "**/file.*" does NOT contain */ as pattern
    // Actually "**/file.*" doesn't have */ - let's use a clearer example
    assert!(!detector.contains_multi_line_end(r#"let s = "end*/";"#, "*/"));
}

#[test]
fn first_comment_marker_in_string_second_outside() {
    let syntax = rust_syntax();
    let detector = CommentDetector::new(&syntax);

    // First /* in string, second /* outside - should detect the second one
    let result = detector.find_multi_line_start(r#"let s = "/*"; /* real"#);
    assert!(result.is_some());
}

#[test]
fn empty_string_edge_cases() {
    let syntax = rust_syntax();
    let detector = CommentDetector::new(&syntax);

    // Empty string, then comment
    assert!(detector.find_multi_line_start(r#""" /* comment"#).is_some());

    // Multiple empty strings
    assert!(
        detector
            .find_multi_line_start(r#""" "" "" /* comment"#)
            .is_some()
    );
}

#[test]
fn single_quote_in_double_quoted_string() {
    let syntax = rust_syntax();
    let detector = CommentDetector::new(&syntax);

    // Single quote inside double-quoted string doesn't end the string
    assert!(
        detector
            .find_multi_line_start(r#"let s = "it's a /*"; foo"#)
            .is_none()
    );

    // After the double-quoted string ends
    assert!(
        detector
            .find_multi_line_start(r#"let s = "it's"; /* comment"#)
            .is_some()
    );
}

#[test]
fn mixed_quotes_char_and_string() {
    let syntax = rust_syntax();
    let detector = CommentDetector::new(&syntax);

    // Char literal followed by string
    assert!(
        detector
            .find_multi_line_start(r#"let c = 'x'; let s = "/*";"#)
            .is_none()
    );

    // Char literal, string, then comment
    assert!(
        detector
            .find_multi_line_start(r#"let c = 'x'; let s = "y"; /* comment"#)
            .is_some()
    );
}

#[test]
fn unicode_before_comment_marker() {
    let syntax = rust_syntax();
    let detector = CommentDetector::new(&syntax);

    // Unicode characters in string before /*
    assert!(
        detector
            .find_multi_line_start(r#"let s = "你好/*"; bar"#)
            .is_none()
    );

    // Unicode in string, comment after
    assert!(
        detector
            .find_multi_line_start(r#"let s = "你好"; /* comment"#)
            .is_some()
    );
}

// =============================================================================
// Known limitations - these tests document current behavior
// =============================================================================

#[test]
fn raw_string_limitation() {
    let syntax = rust_syntax();
    let detector = CommentDetector::new(&syntax);

    // LIMITATION: Raw strings r#"..."# are not specially handled.
    // The current implementation treats r# as code and "..." as a normal string.
    // This means r#"/*"# is seen as: r# (code) + "/*" (string with /*) + # (code)
    // The /* is inside the perceived string, so it's correctly NOT detected.
    //
    // However, this works by coincidence for simple cases:
    assert!(
        detector
            .find_multi_line_start(r##"let s = r#"foo/*bar"#;"##)
            .is_none()
    );

    // For raw strings that contain unbalanced quotes, the detection may be wrong.
    // Example: r#"he said "hello/* there""# - the inner " might confuse parsing
    // This is a known limitation that would require full lexer support to fix.
}
