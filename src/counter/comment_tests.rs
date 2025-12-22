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

// =============================================================================
// Nested comments tests
// =============================================================================

#[test]
fn nested_comment_markers_not_supported() {
    let syntax = rust_syntax();
    let detector = CommentDetector::new(&syntax);

    // In C/Rust, nested comments are NOT supported.
    // /* outer /* inner */ still in outer */ - the first */ closes the comment.
    // So "still in outer */" is code, not a comment.
    // We test that find_multi_line_start finds the FIRST /* marker.
    let result = detector.find_multi_line_start("code /* outer /* inner */ more");
    assert!(result.is_some());

    // The first */ should be detected as ending the comment
    assert!(detector.contains_multi_line_end("/* inner */ more", "*/"));
}

#[test]
fn nested_comment_in_sloc_counting() {
    // Test that nested comment markers are counted correctly in SLOC
    // Since nested comments aren't supported, /* outer /* inner */ ends at first */
    let syntax = rust_syntax();
    let detector = CommentDetector::new(&syntax);

    // When we're inside a comment and see */, it ends regardless of nested /*
    let line = "text /* inner */ not_in_comment";
    assert!(detector.contains_multi_line_end(line, "*/"));
}

#[test]
fn comment_marker_after_nested_pattern() {
    let syntax = rust_syntax();
    let detector = CommentDetector::new(&syntax);

    // Pattern where comment-like sequence appears before real comment
    let line = "x = 1; /* outer */ y = /* second";
    let result = detector.find_multi_line_start(line);
    assert!(result.is_some());
}

// =============================================================================
// Raw strings with unbalanced quotes tests
// =============================================================================

#[test]
fn raw_string_with_unbalanced_double_quote() {
    let syntax = rust_syntax();
    let detector = CommentDetector::new(&syntax);

    // Raw string r#"hello "world"# contains unbalanced quote
    // Current implementation may misparse this - documenting behavior
    // r# is treated as code, then "hello " as string, then world"# as code
    // This is a known limitation.
    let line = r##"let s = r#"hello "world"#; let x = 1;"##;

    // The /* is not present, so should return None regardless
    assert!(detector.find_multi_line_start(line).is_none());
}

#[test]
fn raw_string_with_embedded_quote_and_comment_marker() {
    let syntax = rust_syntax();
    let detector = CommentDetector::new(&syntax);

    // r#"he said "hello/* there""# - inner " and /* might confuse parsing
    // KNOWN LIMITATION: parser does not understand raw string syntax (r#"..."#)
    // It sees: r# (code) + "he said " (string ends at inner quote) + hello/* (code with /*)
    // So the /* IS detected because it's outside the "perceived" string.
    #[allow(clippy::needless_raw_string_hashes)] // Content contains "# which requires ##
    let line = r##"let s = r#"he said "hello/* there""#;"##;

    // Due to limitation, /* is detected (incorrectly for real Rust semantics)
    // This documents the current behavior as a known limitation
    let result = detector.find_multi_line_start(line);
    assert!(result.is_some()); // Limitation: falsely detects /* as comment start
}

#[test]
fn raw_string_with_single_unbalanced_quote() {
    let syntax = rust_syntax();
    let detector = CommentDetector::new(&syntax);

    // r#"it's /* fine"# - single quote inside, /* should not be detected
    let line = r##"let s = r#"it's /* fine"#;"##;

    // The /* appears after what parser sees as string content
    // Due to how raw strings are partially handled, this may work by coincidence
    assert!(detector.find_multi_line_start(line).is_none());
}

#[test]
fn raw_string_followed_by_real_comment() {
    let syntax = rust_syntax();
    let detector = CommentDetector::new(&syntax);

    // Raw string ends, then real comment appears
    let line = r##"let s = r#"raw"#; /* real comment"##;

    // After the raw string ends, /* should be detected
    let result = detector.find_multi_line_start(line);
    assert!(result.is_some());
}

#[test]
fn raw_string_with_escaped_sequences() {
    let syntax = rust_syntax();
    let detector = CommentDetector::new(&syntax);

    // Raw strings don't process escapes: r#"foo\"bar"# has literal \"
    // Parser might be confused by the \"
    let line = r##"let s = r#"foo\"/* bar"#;"##;

    // The /* is inside the raw string content
    // Our simple parser treats \" as escaped quote, might misparse
    let result = detector.find_multi_line_start(line);
    // Document current behavior
    assert!(result.is_none());
}

// =============================================================================
// Python triple-quoted strings with nested quotes tests
// =============================================================================

#[test]
fn python_triple_double_quote_with_single_quotes_inside() {
    let syntax = python_syntax();
    let detector = CommentDetector::new(&syntax);

    // """he said 'hello' there""" - single quotes inside triple double quotes
    let line = r#"s = """he said 'hello' there""""#;

    // The first """ opens the docstring
    // Single quotes inside don't interfere with triple-double-quote matching
    // "there""" ends the docstring, trailing " is outside
    // find_multi_line_start detects the first """ as docstring start
    assert!(detector.find_multi_line_start(line).is_some());
}

#[test]
fn python_triple_single_quote_with_double_quotes_inside() {
    let syntax = python_syntax();
    let detector = CommentDetector::new(&syntax);

    // '''he said "hello" there''' - double quotes inside triple single quotes
    let line = r#"s = '''he said "hello" there'''"#;

    // The first ''' opens the docstring
    // Double quotes inside don't interfere with triple-single-quote matching
    // The closing ''' properly ends the docstring
    // find_multi_line_start detects the first ''' as docstring start
    assert!(detector.find_multi_line_start(line).is_some());
}

#[test]
fn python_triple_quote_with_single_nested_same_quote() {
    let syntax = python_syntax();
    let detector = CommentDetector::new(&syntax);

    // """text with one " inside""" - single double quote inside triple
    let line = r#"s = """text with one " inside""""#;

    // The first """ opens the docstring, but the single " inside doesn't close it
    // However, at "inside""" we have a closing """ - docstring is complete
    // Then the trailing " is outside the docstring
    // find_multi_line_start detects the first """ as docstring start
    assert!(detector.find_multi_line_start(line).is_some());
}

#[test]
fn python_triple_quote_with_two_nested_same_quotes() {
    let syntax = python_syntax();
    let detector = CommentDetector::new(&syntax);

    // """text with "" inside""" - two double quotes inside triple
    let line = r#"s = """text with "" inside""""#;

    // The first """ opens the docstring
    // The "" inside is just two quotes (not closing)
    // Then "inside""" ends at the """ - docstring complete
    // Trailing " is outside
    // find_multi_line_start detects the first """ as docstring start
    assert!(detector.find_multi_line_start(line).is_some());
}

#[test]
fn python_triple_quote_with_mixed_quotes() {
    let syntax = python_syntax();
    let detector = CommentDetector::new(&syntax);

    // """it's a "quote" test""" - both single and double inside
    let line = r#"s = """it's a "quote" test""""#;

    // The first """ opens the docstring
    // Single quotes and double quotes inside are just content
    // "test""" ends the docstring, trailing " is outside
    // find_multi_line_start detects the first """ as docstring start
    assert!(detector.find_multi_line_start(line).is_some());
}

#[test]
fn python_triple_quote_followed_by_comment() {
    let syntax = python_syntax();
    let detector = CommentDetector::new(&syntax);

    // After triple-quoted string ends, # comment should... well, it's single-line
    // Let's test with another docstring
    let line = r#"s = """doc1"""; t = """doc2"#;

    // Second """ starts a new docstring
    let result = detector.find_multi_line_start(line);
    assert!(result.is_some());
}

#[test]
fn python_single_line_docstring_with_nested_quotes() {
    let syntax = python_syntax();
    let detector = CommentDetector::new(&syntax);

    // Docstring on one line with nested quotes
    let line = r#""""This is a "docstring" with quotes""""#;

    // The inner quotes should not terminate early
    // This tests the complete docstring on one line
    let result = detector.find_multi_line_start(line);
    // First """ is found and recognized as docstring start
    assert!(result.is_some());
}

#[test]
fn python_empty_triple_quote_string() {
    let syntax = python_syntax();
    let detector = CommentDetector::new(&syntax);

    // """""" - empty triple-quoted string (6 quotes = open + close)
    let line = r#"s = """""""#;

    // Six quotes = empty docstring
    // First """ starts, second """ ends
    assert!(detector.find_multi_line_start(line).is_some());
}
