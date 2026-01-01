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

// =============================================================================
// Nested comment support tests (Rust/Swift style)
// =============================================================================

#[test]
fn sloc_rust_nested_comment_simple() {
    // With nesting support enabled, /* /* */ */ is ONE complete comment
    let syntax = rust_syntax_with_nesting();
    let counter = SlocCounter::new(&syntax);

    // Single line nested comment - classified as comment
    let source = "/* outer /* inner */ outer */";
    let stats = unwrap_stats(counter.count(source));

    assert_eq!(stats.total, 1);
    assert_eq!(stats.code, 0);
    assert_eq!(stats.comment, 1);
}

#[test]
fn sloc_rust_nested_comment_multiline_properly_tracked() {
    // With nesting support, this entire block is ONE comment
    let syntax = rust_syntax_with_nesting();
    let counter = SlocCounter::new(&syntax);

    let source = r"/* outer
   /* inner */
   still in outer */
let x = 1;";
    let stats = unwrap_stats(counter.count(source));

    // Lines 1-3: all comment (nested comment properly tracked)
    // Line 4: code
    assert_eq!(stats.total, 4);
    assert_eq!(stats.code, 1);
    assert_eq!(stats.comment, 3);
}

#[test]
fn sloc_rust_nested_comment_depth_two() {
    // Depth 2 nested comment: /* a /* b /* c */ b */ a */
    let syntax = rust_syntax_with_nesting();
    let counter = SlocCounter::new(&syntax);

    let source = r"/* level 1
   /* level 2
      /* level 3 */
   back to level 2 */
back to level 1 */
let x = 1;";
    let stats = unwrap_stats(counter.count(source));

    // Lines 1-5: all comment (3 opens, 3 closes)
    // Line 6: code
    assert_eq!(stats.total, 6);
    assert_eq!(stats.code, 1);
    assert_eq!(stats.comment, 5);
}

#[test]
fn sloc_rust_nested_comment_unbalanced_more_opens() {
    // More opens than closes - comment continues
    let syntax = rust_syntax_with_nesting();
    let counter = SlocCounter::new(&syntax);

    let source = r"/* outer /* inner */
still in outer";
    let stats = unwrap_stats(counter.count(source));

    // Both lines are comment (outer still open)
    assert_eq!(stats.total, 2);
    assert_eq!(stats.code, 0);
    assert_eq!(stats.comment, 2);
}

#[test]
fn sloc_rust_nested_comment_closes_correctly() {
    // Verify comment closes properly after matching depth
    let syntax = rust_syntax_with_nesting();
    let counter = SlocCounter::new(&syntax);

    let source = r"/* outer /* inner */ outer */
code_line();";
    let stats = unwrap_stats(counter.count(source));

    // Line 1: comment (starts and ends on same line)
    // Line 2: code
    assert_eq!(stats.total, 2);
    assert_eq!(stats.code, 1);
    assert_eq!(stats.comment, 1);
}

#[test]
fn sloc_nested_vs_non_nested_comparison() {
    // Compare behavior with and without nesting support
    let source = r"/* outer
/* inner */
still in outer */
let x = 1;";

    // WITHOUT nesting support (C-style)
    let syntax_no_nesting = rust_syntax();
    let counter_no = SlocCounter::new(&syntax_no_nesting);
    let stats_no = unwrap_stats(counter_no.count(source));

    // Line 1: comment starts
    // Line 2: comment ends at first */
    // Line 3: "still in outer */" counted as code (limitation)
    // Line 4: code
    assert_eq!(stats_no.comment, 2);
    assert_eq!(stats_no.code, 2);

    // WITH nesting support (Rust-style)
    let syntax_nesting = rust_syntax_with_nesting();
    let counter_yes = SlocCounter::new(&syntax_nesting);
    let stats_yes = unwrap_stats(counter_yes.count(source));

    // Lines 1-3: all comment (properly tracked nesting)
    // Line 4: code
    assert_eq!(stats_yes.comment, 3);
    assert_eq!(stats_yes.code, 1);
}

// =============================================================================
// Stress tests for deeply nested comments
// =============================================================================

#[test]
fn sloc_deeply_nested_comment_depth_10() {
    // Pathological case: 10 levels of nesting
    // Verifies no overflow in NonZeroUsize::saturating_add()
    let syntax = rust_syntax_with_nesting();
    let counter = SlocCounter::new(&syntax);

    let source = "/* /* /* /* /* /* /* /* /* /* deep */ */ */ */ */ */ */ */ */ */";
    let stats = unwrap_stats(counter.count(source));

    // Single line with 10 opens and 10 closes = complete comment
    assert_eq!(stats.total, 1);
    assert_eq!(stats.code, 0);
    assert_eq!(stats.comment, 1);
}

#[test]
fn sloc_deeply_nested_comment_depth_10_multiline() {
    // 10-level nesting spread across multiple lines
    let syntax = rust_syntax_with_nesting();
    let counter = SlocCounter::new(&syntax);

    let source = r"/* level 1
/* level 2
/* level 3
/* level 4
/* level 5
/* level 6
/* level 7
/* level 8
/* level 9
/* level 10 innermost */
back to 9 */
back to 8 */
back to 7 */
back to 6 */
back to 5 */
back to 4 */
back to 3 */
back to 2 */
back to 1 */
let code = 1;";
    let stats = unwrap_stats(counter.count(source));

    // Lines 1-19: all comment (10 opens, 10 closes)
    // Line 20: code
    assert_eq!(stats.total, 20);
    assert_eq!(stats.code, 1);
    assert_eq!(stats.comment, 19);
}

#[test]
fn sloc_deeply_nested_unbalanced_more_opens() {
    // 10 opens, only 5 closes - comment continues to EOF
    let syntax = rust_syntax_with_nesting();
    let counter = SlocCounter::new(&syntax);

    let source = "/* /* /* /* /* /* /* /* /* /* only 5 closes */ */ */ */ */\nstill in comment";
    let stats = unwrap_stats(counter.count(source));

    // Both lines are comment (5 levels still open)
    assert_eq!(stats.total, 2);
    assert_eq!(stats.code, 0);
    assert_eq!(stats.comment, 2);
}

#[test]
fn sloc_rapid_open_close_sequence() {
    // Rapid alternating open/close at high frequency
    let syntax = rust_syntax_with_nesting();
    let counter = SlocCounter::new(&syntax);

    // Each /* */ pair is separate, resulting in alternating comment/code sections
    // But on a single line with comment markers, the line is classified as comment
    let source = "/* */ /* */ /* */ /* */ /* */ /* */ /* */ /* */ /* */ /* */";
    let stats = unwrap_stats(counter.count(source));

    assert_eq!(stats.total, 1);
    assert_eq!(stats.comment, 1);
    assert_eq!(stats.code, 0);
}
