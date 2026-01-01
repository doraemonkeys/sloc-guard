//! Tests for comment markers inside strings - should not be detected

use super::*;

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

#[test]
fn comment_marker_immediately_after_quote() {
    let syntax = rust_syntax();
    let detector = CommentDetector::new(&syntax);

    // String immediately followed by comment (no space)
    assert!(
        detector
            .find_multi_line_start(r#""string"/*comment"#)
            .is_some()
    );

    // String immediately followed by end marker
    assert!(detector.contains_multi_line_end(r#""string"*/"#, "*/"));
}

#[test]
fn glob_pattern_star_star_slash() {
    let syntax = rust_syntax();
    let detector = CommentDetector::new(&syntax);

    // Common glob pattern: **/ which contains */ but is NOT comment end
    // However, */ is a substring, so needs string context to be safe
    // The */ inside "*/" string should not be detected
    // But our pattern is **/ which includes */ as last two chars
    // Actually "**/" is a 4-char string, the */ is at end but inside quotes
    assert!(
        detector
            .find_multi_line_start(r#"let pattern = "**/";"#)
            .is_none()
    );

    // The */ after the string should be detected
    assert!(
        detector
            .find_multi_line_start(r#"let pattern = "**/"; /* real"#)
            .is_some()
    );
}

#[test]
fn regex_with_comment_markers() {
    let syntax = rust_syntax();
    let detector = CommentDetector::new(&syntax);

    // Regex pattern in string containing /*
    let line = r#"let re = Regex::new(r"foo/*bar"); bar"#;
    // The /* is inside the string, should not be detected
    // But wait - r"..." is a raw string in Rust
    // Our parser sees r as code, then "foo/*bar" as string
    // The /* is inside the perceived string, so not detected - correct!
    assert!(detector.find_multi_line_start(line).is_none());
}

#[test]
fn url_in_code() {
    let syntax = rust_syntax();
    let detector = CommentDetector::new(&syntax);

    // URL containing // should not start single-line comment when in string
    let line = r#"let url = "https://example.com";"#;
    // The // is in string, not a comment
    assert!(!detector.is_single_line_comment(line));

    // URL outside string - only matters if line starts with //
    let line2 = "https://example.com";
    assert!(!detector.is_single_line_comment(line2));
}

#[test]
fn division_operator_not_comment() {
    let syntax = rust_syntax();
    let detector = CommentDetector::new(&syntax);

    // a / b is division, not // comment
    assert!(!detector.is_single_line_comment("let x = a / b;"));

    // a/* is weird but / followed by * without slash before isn't //
    // However a/* could start /* */ comment
    let line = "let x = a/* comment */ b";
    assert!(detector.find_multi_line_start(line).is_some());
}

#[test]
fn template_literal_style_string_edge_case() {
    // Some languages have template literals with `${}` or similar
    // Test that these don't interfere (we don't handle them specially)
    let syntax = CommentSyntax::new(vec!["//"], vec![("/*", "*/")]);
    let detector = CommentDetector::new(&syntax);

    // JavaScript-style template literal (but we treat ` as regular char)
    let line = r"let s = `template ${var} string`; /* comment";
    // Backticks aren't string delimiters in our parser
    // So ${ and } and such are just regular characters
    // The /* should be detected (not inside a string we recognize)
    assert!(detector.find_multi_line_start(line).is_some());
}
