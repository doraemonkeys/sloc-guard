//! Rust raw string handling tests - both limitations and proper support

use super::*;

// =============================================================================
// Known limitations - these tests document current behavior without RustRawString
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

#[test]
fn raw_string_works_by_coincidence_balanced_quotes() {
    let syntax = rust_syntax();
    let detector = CommentDetector::new(&syntax);

    // Case: r#"simple content"#
    // Parser sees: r (code) + # (code) + "simple content" (string) + # (code)
    // Since "simple content" has balanced quotes, this accidentally works.
    let line = r##"let s = r#"simple"#; /* comment"##;
    // The /* after the raw string should be detected
    let result = detector.find_multi_line_start(line);
    assert!(result.is_some()); // Works by coincidence!
}

#[test]
fn raw_string_works_by_coincidence_comment_inside() {
    let syntax = rust_syntax();
    let detector = CommentDetector::new(&syntax);

    // Case: r#"contains /* marker"#
    // Parser sees: r# (code) + "contains /* marker" (string) + # (code)
    // The /* is inside the perceived string, so not detected.
    let line = r##"let s = r#"contains /* marker"#;"##;
    assert!(detector.find_multi_line_start(line).is_none()); // Correct by coincidence!
}

#[test]
fn raw_string_misparse_unbalanced_quote_false_positive() {
    let syntax = rust_syntax();
    let detector = CommentDetector::new(&syntax);

    // Case: r#"he said "hello/* there""#
    // In real Rust: this is a valid raw string containing: he said "hello/* there"
    // The content has one unbalanced " (or you could say two ""s with /* between)
    //
    // Parser sees:
    //   - r (code)
    //   - # (code)
    //   - "he said " (string ends at first unescaped ")
    //   - hello/* (code with /*!)
    //   - there (code)
    //   - "" (empty string)
    //   - # (code)
    //
    // The /* is detected as a comment start (WRONG for real Rust semantics!)
    #[allow(clippy::needless_raw_string_hashes)]
    let line = r##"let s = r#"he said "hello/* there""#;"##;
    let result = detector.find_multi_line_start(line);

    // LIMITATION: Falsely detects /* as comment start
    assert!(result.is_some());
}

#[test]
fn raw_string_misparse_unbalanced_quote_false_negative() {
    let syntax = rust_syntax();
    let detector = CommentDetector::new(&syntax);

    // Case: r#"quote: " /* outside"# /* real comment
    // In real Rust: raw string contains: quote: " /* outside
    //               Then there's a real /* comment after the raw string
    //
    // Parser sees:
    //   - r# (code)
    //   - "quote: " (string ends at second ")
    //   - /* (in code - DETECTED as comment... wait, this is BEFORE the raw string ends)
    //   - outside"# (treated as code after comment start)
    //
    // Actually this example is tricky. Let me trace through more carefully:
    //   r#"quote: " /* outside"# /* real comment
    //   - r (code)
    //   - # (code)
    //   - "quote: " (string: "quote: ")
    //   - <space>/* outside (code - /* detected!)
    //   - "" (empty string from the ""#)
    //   - # /* real comment (code)
    //
    // So the first /* (which is inside the raw string in real Rust) is detected.
    // The second /* (the real comment) might not even be reached.
    #[allow(clippy::needless_raw_string_hashes)]
    let line = r##"let s = r#"quote: " /* inside"#; /* real comment"##;
    let result = detector.find_multi_line_start(line);

    // LIMITATION: Detects the wrong /* (the one inside the raw string)
    assert!(result.is_some());
    // The detected position is at the /* inside "quote: " /* inside"#
    // not at the real comment after the semicolon
}

#[test]
fn raw_string_misparse_odd_quotes_shifts_string_boundaries() {
    let syntax = rust_syntax();
    let detector = CommentDetector::new(&syntax);

    // Case: r#"a"b"c"#
    // In real Rust: raw string contains exactly: a"b"c
    //
    // Parser sees:
    //   - r# (code)
    //   - "a" (string)
    //   - b (code)
    //   - "c" (string)
    //   - # (code)
    //
    // Now if we add /* after c:
    let line = r##"let s = r#"a"b"c/* marker"#;"##;
    // Parser sees:
    //   - r# (code)
    //   - "a" (string)
    //   - b (code)
    //   - "c/* marker" (string - the /* is inside!)
    //   - # (code)
    //
    // So /* is NOT detected because it's inside the perceived "c/* marker" string
    assert!(detector.find_multi_line_start(line).is_none()); // Works by coincidence again!
}

#[test]
fn raw_string_misparse_triple_quote_inside() {
    let syntax = rust_syntax();
    let detector = CommentDetector::new(&syntax);

    // Case: r#"contains """ triple quotes"#
    // In real Rust: raw string contains: contains """ triple quotes
    //
    // Parser sees triple quotes as content (no Python-style handling for non-Python syntax)
    // But the string parsing may get confused:
    //   - r# (code)
    //   - "contains " (string ends at first ")
    //   - "" triple quotes (empty string + code)
    //   - "# (another string starting)
    //
    // If we add /* after:
    let line = r##"let s = r#"contains """/* marker"#;"##;
    // Parser sees:
    //   - "contains " (string)
    //   - "" (empty string from the """)
    //   - /* (code - DETECTED!)
    //   - marker"# (code)
    assert!(detector.find_multi_line_start(line).is_some()); // Wrong!
}

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
// Tests with RustRawString properly enabled
// =============================================================================

#[test]
fn raw_string_proper_support_when_enabled() {
    let syntax = rust_syntax_with_raw_string();
    let detector = CommentDetector::new(&syntax);

    // Now the same problematic case should work correctly:
    // r#"he said "hello/* there""# - the /* is inside the raw string
    #[allow(clippy::needless_raw_string_hashes)]
    let line = r##"let s = r#"he said "hello/* there""#;"##;
    let result = detector.find_multi_line_start(line);

    // With proper raw string support, /* inside raw string is NOT detected
    assert!(result.is_none());
}

#[test]
fn raw_string_proper_support_with_real_comment() {
    let syntax = rust_syntax_with_raw_string();
    let detector = CommentDetector::new(&syntax);

    // Raw string with unbalanced quotes, then a real comment
    #[allow(clippy::needless_raw_string_hashes)]
    let line = r##"let s = r#"quote: ""#; /* real comment"##;
    let result = detector.find_multi_line_start(line);

    // Should detect the real comment, not the content inside raw string
    assert!(result.is_some());
}

#[test]
fn raw_string_level_0_no_hashes() {
    let syntax = rust_syntax_with_raw_string();
    let detector = CommentDetector::new(&syntax);

    // r"..." (level 0, no hashes)
    let line = r#"let s = r"contains /* marker"; foo"#;

    // With proper support, /* inside r"..." should not be detected
    assert!(detector.find_multi_line_start(line).is_none());
}

#[test]
fn raw_string_level_2_two_hashes() {
    let syntax = rust_syntax_with_raw_string();
    let detector = CommentDetector::new(&syntax);

    // r##"..."## (level 2)
    // Note: we need 3 # in the outer raw string to contain 2 # inside
    let line = r###"let s = r##"he said "hello"/* there"##; bar"###;

    // The /* is inside r##"..."##, should not be detected
    assert!(detector.find_multi_line_start(line).is_none());

    // Real comment after should be detected
    let line_with_comment = r###"let s = r##"content"##; /* real"###;
    assert!(detector.find_multi_line_start(line_with_comment).is_some());
}

// =============================================================================
// Failure case comparisons - explicitly compare WITH and WITHOUT RustRawString
// =============================================================================

#[test]
fn failure_case_unbalanced_quote_comparison() {
    // Test case: r#"he said "hello/* there""#
    // In real Rust: entire thing is ONE raw string, /* is INSIDE the string
    // Correct behavior: /* should NOT be detected as comment start

    #[allow(clippy::needless_raw_string_hashes)]
    let line = r##"let s = r#"he said "hello/* there""#;"##;

    // WITHOUT RustRawString (current default): WRONG behavior
    let syntax_without = rust_syntax();
    let detector_without = CommentDetector::new(&syntax_without);
    let result_without = detector_without.find_multi_line_start(line);
    assert!(
        result_without.is_some(),
        "BUG: Without RustRawString, falsely detects /* inside raw string as comment"
    );

    // WITH RustRawString: CORRECT behavior
    let syntax_with = rust_syntax_with_raw_string();
    let detector_with = CommentDetector::new(&syntax_with);
    let result_with = detector_with.find_multi_line_start(line);
    assert!(
        result_with.is_none(),
        "With RustRawString, correctly ignores /* inside raw string"
    );
}

#[test]
fn failure_case_triple_quote_inside_raw_string() {
    // Test case: r#"contains """/* marker"#
    // In real Rust: raw string contains: contains """/* marker
    // Correct behavior: /* should NOT be detected

    let line = r##"let s = r#"contains """/* marker"#;"##;

    // WITHOUT RustRawString: WRONG behavior
    let syntax_without = rust_syntax();
    let detector_without = CommentDetector::new(&syntax_without);
    let result_without = detector_without.find_multi_line_start(line);
    assert!(
        result_without.is_some(),
        "BUG: Without RustRawString, falsely detects /* after \"\"\" sequence"
    );

    // WITH RustRawString: CORRECT behavior
    let syntax_with = rust_syntax_with_raw_string();
    let detector_with = CommentDetector::new(&syntax_with);
    let result_with = detector_with.find_multi_line_start(line);
    assert!(
        result_with.is_none(),
        "With RustRawString, correctly ignores /* inside raw string"
    );
}

#[test]
fn failure_case_comment_inside_detected_wrongly() {
    // Test case: r#"/* not a comment */"#
    // In real Rust: raw string contains: /* not a comment */
    // Correct behavior: neither /* nor */ should be detected as comment markers

    // WITHOUT RustRawString: The /* and */ are inside "...", so by coincidence this works
    // But let's test a case where the internal quotes cause issues:
    let line_with_quote = r##"let s = r#"quote: " /* inside"#;"##;

    // WITHOUT RustRawString: WRONG behavior
    let syntax_without = rust_syntax();
    let detector_without = CommentDetector::new(&syntax_without);
    let result_without = detector_without.find_multi_line_start(line_with_quote);
    // Parser sees: "quote: " (string ends) + /* inside (code with /*!)
    assert!(
        result_without.is_some(),
        "BUG: Without RustRawString, falsely detects /* when quote shifts string boundary"
    );

    // WITH RustRawString: CORRECT behavior
    let syntax_with = rust_syntax_with_raw_string();
    let detector_with = CommentDetector::new(&syntax_with);
    let result_with = detector_with.find_multi_line_start(line_with_quote);
    assert!(
        result_with.is_none(),
        "With RustRawString, correctly handles the entire raw string as one unit"
    );
}

#[test]
fn failure_case_real_comment_after_raw_string() {
    // Test case: r#"quote: ""#; /* real comment
    // In real Rust: raw string ends at "#, then there's a real comment
    // Correct behavior: should detect the /* after the raw string

    #[allow(clippy::needless_raw_string_hashes)]
    let line = r##"let s = r#"quote: ""#; /* real comment"##;

    // WITHOUT RustRawString: Parser misinterprets the structure
    // Parser sees: r# + "quote: " (string ends) + "#; /* real comment" (another string starting!)
    // The /* is inside this second "perceived" string, so NOT detected
    // This demonstrates how raw string misparsing can hide real comments!
    let syntax_without = rust_syntax();
    let detector_without = CommentDetector::new(&syntax_without);
    let result_without = detector_without.find_multi_line_start(line);
    assert!(
        result_without.is_none(),
        "BUG: Without RustRawString, real comment after raw string is missed (hidden in fake string)"
    );

    // WITH RustRawString: CORRECT behavior - detects the real comment
    let syntax_with = rust_syntax_with_raw_string();
    let detector_with = CommentDetector::new(&syntax_with);
    let result_with = detector_with.find_multi_line_start(line);
    assert!(
        result_with.is_some(),
        "With RustRawString, correctly detects real comment after raw string"
    );
}

#[test]
fn failure_case_raw_string_with_backslash_quote() {
    // Test case: r#"path: \"/* marker"#
    // In real Rust: raw string contains: path: \"/* marker (backslash is literal)
    // Correct behavior: /* should NOT be detected

    let line = r##"let s = r#"path: \"/* marker"#;"##;

    // WITHOUT RustRawString: Parser treats \" as escape, works by coincidence
    // Parser sees: r# + "path: \" (with escaped quote) + /* marker" (string continues) + #;
    // Wait, let me trace more carefully:
    // - r (code)
    // - # (code)
    // - "path: \" <- escaped quote, string continues
    // - /* marker"#;" <- string ends at the final "
    // So /* is inside the string, not detected
    let syntax_without = rust_syntax();
    let detector_without = CommentDetector::new(&syntax_without);
    let result_without = detector_without.find_multi_line_start(line);
    assert!(
        result_without.is_none(),
        "Without RustRawString, this case works by coincidence (backslash escapes quote)"
    );

    // WITH RustRawString: Should also not detect
    let syntax_with = rust_syntax_with_raw_string();
    let detector_with = CommentDetector::new(&syntax_with);
    let result_with = detector_with.find_multi_line_start(line);
    assert!(
        result_with.is_none(),
        "With RustRawString, correctly handles raw string with backslash"
    );

    // Test a case that DOES break without RustRawString:
    // r#"a"b/* marker"#
    // In real Rust: raw string contains: a"b/* marker
    // Parser sees: r# + "a" (string ends) + b/* (code with /*!)
    let line_broken = r##"let s = r#"a"b/* marker"#;"##;

    // WITHOUT RustRawString: WRONG behavior
    let result_without_broken = detector_without.find_multi_line_start(line_broken);
    assert!(
        result_without_broken.is_some(),
        "BUG: Without RustRawString, unbalanced quote exposes /* to detection"
    );

    // WITH RustRawString: CORRECT behavior
    let result_with_broken = detector_with.find_multi_line_start(line_broken);
    assert!(
        result_with_broken.is_none(),
        "With RustRawString, handles raw string correctly"
    );
}

#[test]
fn failure_case_multiple_raw_strings_on_line() {
    // Test case: r#"a"b"# r#"c/* d"#
    // In real Rust: two raw strings, the /* is inside the second one
    // Correct behavior: /* should NOT be detected

    #[allow(clippy::needless_raw_string_hashes)]
    let line = r##"let a = r#"x"y"#; let b = r#"c/* d"#;"##;

    // WITHOUT RustRawString: Parser gets confused by the quotes
    let syntax_without = rust_syntax();
    let detector_without = CommentDetector::new(&syntax_without);
    let result_without = detector_without.find_multi_line_start(line);
    // Due to unbalanced quotes from first raw string, parsing shifts and /* may be detected
    assert!(
        result_without.is_some(),
        "BUG: Without RustRawString, multiple raw strings cause parsing confusion"
    );

    // WITH RustRawString: CORRECT behavior
    let syntax_with = rust_syntax_with_raw_string();
    let detector_with = CommentDetector::new(&syntax_with);
    let result_with = detector_with.find_multi_line_start(line);
    assert!(
        result_with.is_none(),
        "With RustRawString, correctly handles multiple raw strings"
    );
}

#[test]
fn failure_case_byte_raw_string() {
    // Test case: br#"bytes/* here"#
    // In real Rust: br#"..."# is a byte raw string
    // Current implementation does NOT handle br#"..."# at all!

    let line = r##"let b = br#"bytes/* here"#;"##;

    // WITHOUT RustRawString: Parser sees b (code) + r#"bytes/* here"# (raw string pattern)
    // Actually, r starts raw string matching, so this might partially work
    let syntax_without = rust_syntax();
    let detector_without = CommentDetector::new(&syntax_without);
    let result_without = detector_without.find_multi_line_start(line);

    // WITH RustRawString: Still doesn't handle br#"..."# because it only looks for r#"
    // This is a KNOWN LIMITATION: byte raw strings (br#"..."#) not supported
    let syntax_with = rust_syntax_with_raw_string();
    let detector_with = CommentDetector::new(&syntax_with);
    let result_with = detector_with.find_multi_line_start(line);

    // Document current behavior - both should not detect /* (by coincidence in this case)
    // because the /* is inside "bytes/* here" which is parsed as a regular string
    assert!(result_without.is_none());
    assert!(result_with.is_none());
    // Note: If the byte raw string contained unbalanced quotes, both would fail!
}

#[test]
fn failure_case_raw_string_spanning_affects_subsequent_code() {
    // When raw string is misparsed, the quote state can be wrong for following code
    // Test case: r#"x"y"# let z = "/*"; /* real
    // First raw string has unbalanced quote, so parser thinks we're in/out of string wrongly

    #[allow(clippy::needless_raw_string_hashes)]
    let line = r##"let a = r#"x"y"#; let z = "/*"; /* real"##;

    // WITHOUT RustRawString: Quote state is corrupted after first raw string
    let syntax_without = rust_syntax();
    let detector_without = CommentDetector::new(&syntax_without);
    let result_without = detector_without.find_multi_line_start(line);
    // Parser sees after r#: "x" (string) + y (code) + "# (broken) + ...
    // The subsequent "/*" may or may not be parsed correctly
    // And the real /* at end may or may not be detected
    // This demonstrates cascading failure from raw string misparsing
    assert!(
        result_without.is_some(),
        "Without RustRawString, some /* is detected (may be wrong one)"
    );

    // WITH RustRawString: CORRECT behavior
    let syntax_with = rust_syntax_with_raw_string();
    let detector_with = CommentDetector::new(&syntax_with);
    let result_with = detector_with.find_multi_line_start(line);
    // Should correctly skip raw string, then see "/*" in string, then detect real /*
    assert!(
        result_with.is_some(),
        "With RustRawString, correctly detects real comment"
    );
}
