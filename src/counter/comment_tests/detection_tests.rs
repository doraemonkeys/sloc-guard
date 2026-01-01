//! Basic single-line and multi-line comment detection tests

use super::*;

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
    let m = result.unwrap();
    assert_eq!(m.comment.start.as_str(), "/*");
    assert_eq!(m.end_marker(), "*/");

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
fn line_is_just_comment_marker() {
    let syntax = rust_syntax();
    let detector = CommentDetector::new(&syntax);

    // Line is exactly the comment marker
    assert!(detector.find_multi_line_start("/*").is_some());
    assert!(detector.contains_multi_line_end("*/", "*/"));
}

#[test]
fn comment_marker_at_various_positions() {
    let syntax = rust_syntax();
    let detector = CommentDetector::new(&syntax);

    // At start
    assert!(detector.find_multi_line_start("/* comment").is_some());

    // At end
    assert!(detector.find_multi_line_start("code /*").is_some());

    // In middle
    assert!(detector.find_multi_line_start("a /* b").is_some());
}

#[test]
fn empty_needle_returns_none() {
    let syntax = rust_syntax();
    let detector = CommentDetector::new(&syntax);

    // Edge case: empty end marker
    assert!(!detector.contains_multi_line_end("any line", ""));
}

#[test]
fn partial_comment_marker_not_matched() {
    let syntax = rust_syntax();
    let detector = CommentDetector::new(&syntax);

    // Just / is not /*
    let line = "let x = a / b;";
    assert!(detector.find_multi_line_start(line).is_none());

    // Just * is not */
    assert!(!detector.contains_multi_line_end("let x = a * b;", "*/"));
}

#[test]
fn comment_marker_split_by_whitespace() {
    let syntax = rust_syntax();
    let detector = CommentDetector::new(&syntax);

    // "/ *" is NOT a comment (space between)
    let line = "let x = 1; / * not a comment";
    assert!(detector.find_multi_line_start(line).is_none());

    // "* /" is NOT a comment end
    let line2 = "still in comment * /";
    assert!(!detector.contains_multi_line_end(line2, "*/"));
}

#[test]
fn multiple_different_comment_styles_first_defined_wins() {
    // Language with multiple multi-line comment styles (like Vue)
    let syntax = CommentSyntax::new(vec!["//"], vec![("/*", "*/"), ("<!--", "-->")]);
    let detector = CommentDetector::new(&syntax);

    // /* appears first in line, and is first in syntax definition -> detected
    let result = detector.find_multi_line_start("code /* js */ <!-- html");
    assert!(result.is_some());
    let m = result.unwrap();
    assert_eq!(m.comment.start.as_str(), "/*");
    assert_eq!(m.end_marker(), "*/");
}

#[test]
fn multiple_comment_styles_match_by_position_not_definition_order() {
    // When multiple comment styles exist, find_multi_line_start
    // returns the one that appears earliest in the line, not by definition order.
    let syntax = CommentSyntax::new(vec!["//"], vec![("/*", "*/"), ("<!--", "-->")]);
    let detector = CommentDetector::new(&syntax);

    // <!-- appears FIRST in line (position 5), /* appears SECOND (position 22)
    // Returns <!-- because it appears first in the line
    let result = detector.find_multi_line_start("code <!-- html --> /* js");
    assert!(result.is_some());
    let m = result.unwrap();
    assert_eq!(m.comment.start.as_str(), "<!--");
    assert_eq!(m.end_marker(), "-->");

    // When /* appears first, it should be returned
    let result2 = detector.find_multi_line_start("code /* js */ <!-- html");
    assert!(result2.is_some());
    let m2 = result2.unwrap();
    assert_eq!(m2.comment.start.as_str(), "/*");
    assert_eq!(m2.end_marker(), "*/");
}

#[test]
fn html_comment_style() {
    // Test HTML/XML style comments
    let syntax = CommentSyntax::new(vec![], vec![("<!--", "-->")]);
    let detector = CommentDetector::new(&syntax);

    assert!(
        detector
            .find_multi_line_start("<!-- html comment")
            .is_some()
    );
    assert!(detector.contains_multi_line_end("end of comment -->", "-->"));

    // In string (if language has strings)
    let syntax2 = CommentSyntax::new(vec![], vec![("<!--", "-->")]);
    let detector2 = CommentDetector::new(&syntax2);
    assert!(
        detector2
            .find_multi_line_start(r#"let s = "<!--"; foo"#)
            .is_none()
    );
}
