use super::*;

#[test]
fn glob_pattern_in_string_not_treated_as_comment() {
    let syntax = rust_syntax();
    let counter = SlocCounter::new(&syntax);
    // "src/**" contains /* but it's in a string, not a comment
    let source = r#"let pattern = "src/**";"#;
    let stats = unwrap_stats(counter.count(source));

    assert_eq!(stats.total, 1);
    assert_eq!(stats.code, 1);
    assert_eq!(stats.comment, 0);
}

#[test]
fn multiple_glob_patterns_in_strings() {
    let syntax = rust_syntax();
    let counter = SlocCounter::new(&syntax);
    // Multiple patterns with /* inside strings
    let source = r#"pattern: "src/generated/**".to_string(),
pattern: "tests/**/fixtures/**".to_string(),
let x = 1;
pattern: "src/*/utils/**".to_string(),"#;
    let stats = unwrap_stats(counter.count(source));

    assert_eq!(stats.total, 4);
    assert_eq!(stats.code, 4);
    assert_eq!(stats.comment, 0);
}

#[test]
fn glob_pattern_followed_by_real_comment() {
    let syntax = rust_syntax();
    let counter = SlocCounter::new(&syntax);
    let source = r#"let pattern = "src/**"; // This is a real comment
/* This is a real multi-line comment */
let x = 1;"#;
    let stats = unwrap_stats(counter.count(source));

    assert_eq!(stats.total, 3);
    assert_eq!(stats.code, 2); // pattern line has trailing comment but is counted as code
    assert_eq!(stats.comment, 1); // multi-line comment
}

#[test]
fn realistic_test_file_with_glob_patterns() {
    let syntax = rust_syntax();
    let counter = SlocCounter::new(&syntax);
    // Simulates a typical test file structure with glob patterns
    let source = r#"#[test]
fn test_glob_rule() {
    let config = Config {
        rules: vec![Rule {
            pattern: "src/generated/**".to_string(),
            max_files: Some(100),
        }],
    };
    let checker = Checker::new(&config);
    assert!(checker.is_enabled());
}"#;
    let stats = unwrap_stats(counter.count(source));

    assert_eq!(stats.total, 11);
    assert_eq!(stats.code, 11);
    assert_eq!(stats.comment, 0);
}

#[test]
fn comment_end_marker_in_string_not_closing_comment() {
    let syntax = rust_syntax();
    let counter = SlocCounter::new(&syntax);
    // Once in a multi-line comment, "*/" in a string should still close it
    // because we're IN a comment, not looking at code
    let source = "/* comment\nstill comment */\nlet x = 1;";
    let stats = unwrap_stats(counter.count(source));

    assert_eq!(stats.total, 3);
    assert_eq!(stats.code, 1);
    assert_eq!(stats.comment, 2);
}

#[test]
fn sloc_raw_string_simple() {
    let syntax = rust_syntax();
    let counter = SlocCounter::new(&syntax);
    // Simple raw string
    let source = r##"let s = r#"hello world"#;"##;
    let stats = unwrap_stats(counter.count(source));

    assert_eq!(stats.total, 1);
    assert_eq!(stats.code, 1);
    assert_eq!(stats.comment, 0);
}

#[test]
fn sloc_raw_string_with_quotes_inside() {
    let syntax = rust_syntax();
    let counter = SlocCounter::new(&syntax);
    // Raw string with embedded quotes: r#"say "hi""#
    #[allow(clippy::needless_raw_string_hashes)] // Content contains "# which requires ##
    let source = r##"let s = r#"say "hi""#;"##;
    let stats = unwrap_stats(counter.count(source));

    assert_eq!(stats.total, 1);
    assert_eq!(stats.code, 1);
    assert_eq!(stats.comment, 0);
}

#[test]
fn sloc_raw_string_with_comment_like_pattern() {
    let syntax = rust_syntax();
    let counter = SlocCounter::new(&syntax);
    // Raw string containing /* pattern: r#"pattern/*"#
    let source = r##"let s = r#"glob/**/pattern"#;"##;
    let stats = unwrap_stats(counter.count(source));

    assert_eq!(stats.total, 1);
    assert_eq!(stats.code, 1);
    assert_eq!(stats.comment, 0);
}

#[test]
fn sloc_raw_string_multiline_delimiter() {
    let syntax = rust_syntax();
    let counter = SlocCounter::new(&syntax);
    // Raw string with ## delimiter: r##"text"##
    let source = r###"let s = r##"hello"##;"###;
    let stats = unwrap_stats(counter.count(source));

    assert_eq!(stats.total, 1);
    assert_eq!(stats.code, 1);
    assert_eq!(stats.comment, 0);
}
