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
