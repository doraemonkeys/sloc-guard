//! Ruby-specific comment tests

use super::*;

#[test]
fn ruby_multiline_comment_detection() {
    let syntax = ruby_syntax();
    let detector = CommentDetector::new(&syntax);

    // Ruby multiline comment markers
    assert!(detector.find_multi_line_start("=begin comment").is_some());
    assert!(detector.contains_multi_line_end("=end", "=end"));
}

#[test]
fn ruby_comment_marker_in_string() {
    let syntax = ruby_syntax();
    let detector = CommentDetector::new(&syntax);

    // =begin in string should not start comment
    assert!(
        detector
            .find_multi_line_start(r#"s = "=begin not a comment""#)
            .is_none()
    );
}

#[test]
fn ruby_comment_marker_requires_line_start() {
    // Ruby =begin/=end must be at line start (column 0)
    let syntax = CommentSyntax::with_multi_line(
        vec!["#"],
        vec![MultiLineComment::new("=begin", "=end").at_line_start()],
    );
    let detector = CommentDetector::new(&syntax);

    // =begin NOT at start of line - should NOT be detected
    let line = "x = 1; =begin looks like comment";
    let result = detector.find_multi_line_start(line);
    assert!(result.is_none());

    // =begin at start of line (with leading whitespace trimmed) - should be detected
    let line2 = "  =begin real comment";
    let result2 = detector.find_multi_line_start(line2);
    assert!(result2.is_some());

    // =begin exactly at column 0 - should be detected
    let line3 = "=begin comment";
    let result3 = detector.find_multi_line_start(line3);
    assert!(result3.is_some());
}
