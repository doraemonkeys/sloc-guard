//! Edge cases: escapes, unicode, quotes, backslashes, and complex patterns

use super::*;

#[test]
fn comment_in_char_literal_sequence() {
    let syntax = rust_syntax();
    let detector = CommentDetector::new(&syntax);

    // Multiple char literals forming comment-like sequence
    // This is contrived but tests parser edge case
    let line = r"let a = '/'; let b = '*'; /* real comment";
    assert!(detector.find_multi_line_start(line).is_some());
}

#[test]
fn string_with_backslash_at_end() {
    let syntax = rust_syntax();
    let detector = CommentDetector::new(&syntax);

    // String ending with backslash-quote vs backslash-backslash-quote
    // "foo\" - escaped quote, string continues
    // "foo\\" - escaped backslash, string ends

    // String with escaped backslash at end, then comment
    assert!(
        detector
            .find_multi_line_start(r#"let s = "path\\"; /* comment"#)
            .is_some()
    );

    // Multiple escaped backslashes
    assert!(
        detector
            .find_multi_line_start(r#"let s = "path\\\\"; /* comment"#)
            .is_some()
    );

    // Escaped backslash followed by quote still ends string
    assert!(
        detector
            .find_multi_line_start(r#"let s = "path\\" /* comment"#)
            .is_some()
    );
}

#[test]
fn consecutive_backslash_escape_sequences() {
    let syntax = rust_syntax();
    let detector = CommentDetector::new(&syntax);

    // "\\\"" = escaped backslash + escaped quote = two chars in string
    // The /* after should NOT be detected (still inside string if not closed)
    let line = r#"let s = "\\\"/*";"#; // String is \\\"/* then ends at "
    // Parser should see: " starts, \\ is escaped backslash, \" is escaped quote, /* is in string, " ends
    assert!(detector.find_multi_line_start(line).is_none());

    // Same but string properly closes and then comment
    let line2 = r#"let s = "\\\""; /* comment"#;
    assert!(detector.find_multi_line_start(line2).is_some());
}

#[test]
fn deeply_escaped_quote_chain() {
    let syntax = rust_syntax();
    let detector = CommentDetector::new(&syntax);

    // Chain: \\\\" means: \\ (escaped backslash) + \\ (escaped backslash) + " (ends string)
    let line = r#"let s = "\\\\"; /* after"#;
    assert!(detector.find_multi_line_start(line).is_some());

    // \\\\\\" means: \\ + \\ + \" = still in string
    let line2 = r#"let s = "\\\\\"/*still in"; foo"#;
    assert!(detector.find_multi_line_start(line2).is_none());
}

#[test]
fn unicode_in_string_before_comment() {
    let syntax = rust_syntax();
    let detector = CommentDetector::new(&syntax);

    // Multi-byte UTF-8 characters
    assert!(
        detector
            .find_multi_line_start(r#"let s = "日本語"; /* comment"#)
            .is_some()
    );

    // Emoji in string
    assert!(
        detector
            .find_multi_line_start(r#"let s = "🎉/*🎉"; foo"#)
            .is_none()
    );

    // Unicode after comment marker
    assert!(detector.find_multi_line_start("/* 日本語").is_some());
}

// =============================================================================
// Quote variations
// =============================================================================

#[test]
fn alternating_quote_types() {
    let syntax = rust_syntax();
    let detector = CommentDetector::new(&syntax);

    // Single quote char, double quote string, alternating
    let line = r#"let c = '"'; let s = "'"; /* comment"#;
    assert!(detector.find_multi_line_start(line).is_some());
}

#[test]
fn quote_in_char_does_not_start_string() {
    let syntax = rust_syntax();
    let detector = CommentDetector::new(&syntax);

    // '"' is a char literal containing double quote
    // The /* after should be detected
    let line = r#"let c = '"'; /* comment"#;
    assert!(detector.find_multi_line_start(line).is_some());
}

#[test]
fn apostrophe_in_identifier() {
    // Some languages allow apostrophes in identifiers (Haskell, OCaml)
    // Rust doesn't, but testing the parser behavior
    let syntax = rust_syntax();
    let detector = CommentDetector::new(&syntax);

    // In Rust, a' would be a lifetime, but 'a is
    // This tests that isolated quotes don't break parsing
    let line = "fn foo<'a>(x: &'a str) /* comment */";
    assert!(detector.find_multi_line_start(line).is_some());
}
