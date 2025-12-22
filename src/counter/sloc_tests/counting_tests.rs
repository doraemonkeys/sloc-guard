use super::*;

#[test]
fn line_stats_default() {
    let stats = LineStats::default();
    assert_eq!(stats.total, 0);
    assert_eq!(stats.code, 0);
    assert_eq!(stats.comment, 0);
    assert_eq!(stats.blank, 0);
}

#[test]
fn count_empty_source() {
    let syntax = rust_syntax();
    let counter = SlocCounter::new(&syntax);
    let stats = unwrap_stats(counter.count(""));

    assert_eq!(stats.total, 0);
    assert_eq!(stats.sloc(), 0);
}

#[test]
fn count_code_only() {
    let syntax = rust_syntax();
    let counter = SlocCounter::new(&syntax);
    let source = "fn main() {\n    println!(\"hello\");\n}";
    let stats = unwrap_stats(counter.count(source));

    assert_eq!(stats.total, 3);
    assert_eq!(stats.code, 3);
    assert_eq!(stats.comment, 0);
    assert_eq!(stats.blank, 0);
}

#[test]
fn count_with_blank_lines() {
    let syntax = rust_syntax();
    let counter = SlocCounter::new(&syntax);
    let source = "fn main() {\n\n    println!(\"hello\");\n\n}";
    let stats = unwrap_stats(counter.count(source));

    assert_eq!(stats.total, 5);
    assert_eq!(stats.code, 3);
    assert_eq!(stats.blank, 2);
}

#[test]
fn count_with_single_line_comments() {
    let syntax = rust_syntax();
    let counter = SlocCounter::new(&syntax);
    let source = "// This is a comment\nfn main() {\n    // Another comment\n}";
    let stats = unwrap_stats(counter.count(source));

    assert_eq!(stats.total, 4);
    assert_eq!(stats.code, 2);
    assert_eq!(stats.comment, 2);
}

#[test]
fn count_with_doc_comments() {
    let syntax = rust_syntax();
    let counter = SlocCounter::new(&syntax);
    let source = "/// Documentation\n//! Module docs\nfn main() {}";
    let stats = unwrap_stats(counter.count(source));

    assert_eq!(stats.code, 1);
    assert_eq!(stats.comment, 2);
}

#[test]
fn count_with_multi_line_comment() {
    let syntax = rust_syntax();
    let counter = SlocCounter::new(&syntax);
    let source = "/* Multi\n   line\n   comment */\nfn main() {}";
    let stats = unwrap_stats(counter.count(source));

    assert_eq!(stats.total, 4);
    assert_eq!(stats.code, 1);
    assert_eq!(stats.comment, 3);
}

#[test]
fn count_with_single_line_multi_comment() {
    let syntax = rust_syntax();
    let counter = SlocCounter::new(&syntax);
    let source = "/* single line comment */\nfn main() {}";
    let stats = unwrap_stats(counter.count(source));

    assert_eq!(stats.code, 1);
    assert_eq!(stats.comment, 1);
}

#[test]
fn sloc_returns_code_count() {
    let stats = LineStats {
        total: 100,
        code: 80,
        comment: 15,
        blank: 5,
        ignored: 0,
    };
    assert_eq!(stats.sloc(), 80);
}

#[test]
fn count_mixed_content() {
    let syntax = rust_syntax();
    let counter = SlocCounter::new(&syntax);
    let source = r#"//! Module documentation

use std::io;

/// Main function
fn main() {
    /* inline comment */
    println!("Hello");
}
"#;
    let stats = unwrap_stats(counter.count(source));

    assert_eq!(stats.blank, 2);
    assert!(stats.comment >= 3);
    assert!(stats.code >= 4);
}

#[test]
fn count_reader_produces_same_result_as_count() {
    let syntax = rust_syntax();
    let counter = SlocCounter::new(&syntax);
    let source = r#"//! Module documentation

use std::io;

/// Main function
fn main() {
    /* inline comment */
    println!("Hello");
}
"#;

    let stats_from_str = unwrap_stats(counter.count(source));
    let stats_from_reader = unwrap_stats_reader(counter.count_reader(Cursor::new(source)));

    assert_eq!(stats_from_str.total, stats_from_reader.total);
    assert_eq!(stats_from_str.code, stats_from_reader.code);
    assert_eq!(stats_from_str.comment, stats_from_reader.comment);
    assert_eq!(stats_from_str.blank, stats_from_reader.blank);
}

#[test]
fn count_reader_empty_input() {
    let syntax = rust_syntax();
    let counter = SlocCounter::new(&syntax);
    let stats = unwrap_stats_reader(counter.count_reader(Cursor::new("")));

    assert_eq!(stats.total, 0);
    assert_eq!(stats.sloc(), 0);
}

#[test]
fn count_reader_with_multi_line_comment() {
    let syntax = rust_syntax();
    let counter = SlocCounter::new(&syntax);
    let source = "/* Multi\n   line\n   comment */\nfn main() {}";
    let stats = unwrap_stats_reader(counter.count_reader(Cursor::new(source)));

    assert_eq!(stats.total, 4);
    assert_eq!(stats.code, 1);
    assert_eq!(stats.comment, 3);
}

#[test]
fn line_stats_ignored_default() {
    let stats = LineStats::default();
    assert_eq!(stats.ignored, 0);
}

#[test]
fn line_stats_new_has_zero_ignored() {
    let stats = LineStats::new();
    assert_eq!(stats.ignored, 0);
}
