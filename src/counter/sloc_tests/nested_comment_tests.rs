use super::*;

#[test]
fn sloc_nested_comment_markers_first_close_wins() {
    let syntax = rust_syntax();
    let counter = SlocCounter::new(&syntax);
    // /* outer /* inner */ rest_is_code */ final
    // In C/Rust, nested comments NOT supported: first */ closes the comment
    // The SlocCounter classifies any line containing a block comment as a comment line
    // (it doesn't split lines into partial code/comment)
    let source = "/* outer /* inner */ rest_is_code */ final";
    let stats = unwrap_stats(counter.count(source));

    // The entire line is classified as comment because it starts with /*
    // (SlocCounter counts lines, not partial line segments)
    assert_eq!(stats.total, 1);
    assert_eq!(stats.code, 0);
    assert_eq!(stats.comment, 1);
}

#[test]
fn sloc_nested_comment_multiline() {
    let syntax = rust_syntax();
    let counter = SlocCounter::new(&syntax);
    // Multi-line with nested markers
    let source = "/* start /* nested\nstill comment */\nlet x = 1;";
    let stats = unwrap_stats(counter.count(source));

    // Line 1: "/* start /* nested" - comment start
    // Line 2: "still comment */" - comment end
    // Line 3: "let x = 1;" - code
    assert_eq!(stats.total, 3);
    assert_eq!(stats.code, 1);
    assert_eq!(stats.comment, 2);
}

#[test]
fn sloc_fake_nested_comment_in_string() {
    let syntax = rust_syntax();
    let counter = SlocCounter::new(&syntax);
    // /* inside string should not trigger comment
    let source = r#"let s = "/* not a comment */";"#;
    let stats = unwrap_stats(counter.count(source));

    assert_eq!(stats.total, 1);
    assert_eq!(stats.code, 1);
    assert_eq!(stats.comment, 0);
}
