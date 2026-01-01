//! SQL-specific comment tests

use super::*;

#[test]
fn sql_dash_dash_vs_decrement() {
    let syntax = sql_syntax();
    let detector = CommentDetector::new(&syntax);

    // In SQL, -- starts a comment
    // But what about x-- (decrement in some languages)?
    // Our parser checks starts_with, so "x--" doesn't match
    assert!(!detector.is_single_line_comment("x-- not a comment"));
    assert!(detector.is_single_line_comment("-- this is a comment"));
}

#[test]
fn sql_comment_in_string() {
    let syntax = sql_syntax();
    let detector = CommentDetector::new(&syntax);

    // SELECT '/* not a comment */' FROM t
    assert!(
        detector
            .find_multi_line_start(r"SELECT '/* not a comment */' FROM t")
            .is_none()
    );
}
