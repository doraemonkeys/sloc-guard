//! Nested block comment tests - Rust and Swift style nested comments

use super::*;

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
// Rust nested block comments - LANGUAGE FEATURE LIMITATION
// =============================================================================
// NOTE: Rust supports nested block comments, unlike C/C++.
// In Rust, `/* /* */ */` is ONE complete comment (outer contains inner).
// The current implementation does NOT track nesting depth, causing incorrect behavior.

#[test]
fn rust_nested_block_comment_limitation_simple() {
    let syntax = rust_syntax();
    let detector = CommentDetector::new(&syntax);

    // In Rust: /* outer /* inner */ outer_continues */ is ONE comment.
    // But our parser sees first */ as end, treating "outer_continues */" as code.
    //
    // LIMITATION: We detect */ even when inside nested comment.
    // This test documents the current (incorrect for Rust) behavior.
    let line = "/* outer /* inner */ still_outer */";

    // Our parser finds the first */ and thinks comment ends there
    // This is INCORRECT for Rust which supports nested comments
    assert!(detector.contains_multi_line_end(line, "*/"));

    // What SHOULD happen: the first */ closes the inner, not the outer
    // So "still_outer */" should still be inside the comment
    // Only the second */ should end the comment
    // But we can't test this without counting nesting levels.
}

#[test]
fn rust_nested_block_comment_limitation_counting() {
    // This test demonstrates the SLOC counting error caused by not supporting nested comments.
    //
    // Consider this Rust code:
    // ```
    // /* outer
    //    /* inner */
    //    still in outer */
    // let x = 1;
    // ```
    //
    // Current behavior (INCORRECT for Rust):
    // - Line 1: comment starts
    // - Line 2: comment ends at first */
    // - Line 3: "still in outer */" is counted as CODE (wrong!)
    // - Line 4: code
    //
    // Correct behavior for Rust:
    // - Lines 1-3: all comment
    // - Line 4: code
    //
    // This is a fundamental limitation requiring nested comment depth tracking.
    let syntax = rust_syntax();
    let detector = CommentDetector::new(&syntax);

    // After "/* inner */", we're back inside outer comment, not outside
    // But our parser incorrectly thinks we exited the comment
    let line2 = "   /* inner */";
    assert!(detector.contains_multi_line_end(line2, "*/")); // Returns true (INCORRECT for nested)
}

#[test]
fn rust_nested_comment_depth_two() {
    let syntax = rust_syntax();
    let detector = CommentDetector::new(&syntax);

    // Depth 2 nested comment: /* a /* b /* c */ b */ a */
    // Closing order: first */ closes c, second */ closes b, third */ closes a
    let line = "/* a /* b /* c */ b */ a */";

    // We find FIRST */ (after c) when we should only exit on THIRD */
    assert!(detector.contains_multi_line_end(line, "*/"));
    // This is the FIRST */ position, not the outermost one
}

#[test]
fn rust_nested_comment_only_opener_in_line() {
    let syntax = rust_syntax();
    let detector = CommentDetector::new(&syntax);

    // Line with only nested opener, no closer
    let line = "   /* nested inside outer";
    assert!(!detector.contains_multi_line_end(line, "*/"));
    // This is correct - no */ in line
}

#[test]
fn swift_nested_comment_nesting_detection() {
    // Swift also supports nested block comments like Rust
    // Testing the count_nesting_changes API for proper nesting detection

    let syntax = CommentSyntax::with_multi_line(
        vec!["//", "///"],
        vec![MultiLineComment::new("/*", "*/").with_nesting()],
    );
    let detector = CommentDetector::new(&syntax);

    // Line with nested comment: /* outer /* inner */ still in outer */
    // Should have 2 starts and 2 ends
    let line = "/* outer /* inner */ still in outer */";
    let (starts, ends) = detector.count_nesting_changes(line, "/*", "*/");
    assert_eq!(starts, 2);
    assert_eq!(ends, 2);
}
