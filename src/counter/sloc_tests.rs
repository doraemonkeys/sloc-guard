use super::*;
use std::io::Cursor;

fn rust_syntax() -> CommentSyntax {
    CommentSyntax::new(vec!["//", "///", "//!"], vec![("/*", "*/")])
}

fn python_syntax() -> CommentSyntax {
    CommentSyntax::new(vec!["#"], vec![("\"\"\"", "\"\"\""), ("'''", "'''")])
}

fn unwrap_stats(result: CountResult) -> LineStats {
    match result {
        CountResult::Stats(stats) => stats,
        CountResult::IgnoredFile => panic!("Expected Stats, got IgnoredFile"),
    }
}

fn unwrap_stats_reader(result: std::io::Result<CountResult>) -> LineStats {
    match result.unwrap() {
        CountResult::Stats(stats) => stats,
        CountResult::IgnoredFile => panic!("Expected Stats, got IgnoredFile"),
    }
}

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

// =============================================================================
// Tests for inline ignore-file directive
// =============================================================================

#[test]
fn ignore_file_directive_first_line_rust() {
    let syntax = rust_syntax();
    let counter = SlocCounter::new(&syntax);
    let source = "// sloc-guard:ignore-file\nfn main() {\n    println!(\"hello\");\n}";
    let result = counter.count(source);

    assert_eq!(result, CountResult::IgnoredFile);
}

#[test]
fn ignore_file_directive_with_doc_comment() {
    let syntax = rust_syntax();
    let counter = SlocCounter::new(&syntax);
    let source = "/// sloc-guard:ignore-file\nfn main() {}";
    let result = counter.count(source);

    assert_eq!(result, CountResult::IgnoredFile);
}

#[test]
fn ignore_file_directive_within_first_10_lines() {
    let syntax = rust_syntax();
    let counter = SlocCounter::new(&syntax);
    let source = r#"//! Module docs
//!
//! This is a test module

use std::io;

// sloc-guard:ignore-file

fn main() {
    println!("hello");
}
"#;
    let result = counter.count(source);

    assert_eq!(result, CountResult::IgnoredFile);
}

#[test]
fn ignore_file_directive_after_line_10_not_ignored() {
    let syntax = rust_syntax();
    let counter = SlocCounter::new(&syntax);
    let source = r"line1
line2
line3
line4
line5
line6
line7
line8
line9
line10
// sloc-guard:ignore-file
line12
";
    let result = counter.count(source);

    // Directive is on line 11, should NOT be honored
    match result {
        CountResult::Stats(stats) => {
            assert_eq!(stats.total, 12);
        }
        CountResult::IgnoredFile => panic!("Should not be ignored, directive is after line 10"),
    }
}

#[test]
fn ignore_file_directive_python_style() {
    let syntax = python_syntax();
    let counter = SlocCounter::new(&syntax);
    let source = "# sloc-guard:ignore-file\ndef main():\n    print('hello')\n";
    let result = counter.count(source);

    assert_eq!(result, CountResult::IgnoredFile);
}

#[test]
fn ignore_file_directive_with_extra_text() {
    let syntax = rust_syntax();
    let counter = SlocCounter::new(&syntax);
    let source = "// sloc-guard:ignore-file - generated code\nfn main() {}";
    let result = counter.count(source);

    assert_eq!(result, CountResult::IgnoredFile);
}

#[test]
fn ignore_file_directive_not_in_comment_ignored() {
    let syntax = rust_syntax();
    let counter = SlocCounter::new(&syntax);
    // The directive is not in a comment, just bare text
    let source = "sloc-guard:ignore-file\nfn main() {}";
    let result = counter.count(source);

    // Should NOT be ignored because it's not in a comment
    match result {
        CountResult::Stats(stats) => {
            assert_eq!(stats.total, 2);
            assert_eq!(stats.code, 2);
        }
        CountResult::IgnoredFile => panic!("Should not be ignored, directive is not in a comment"),
    }
}

#[test]
fn ignore_file_directive_in_multi_line_comment_not_recognized() {
    let syntax = rust_syntax();
    let counter = SlocCounter::new(&syntax);
    // Multi-line comment style should NOT be recognized (only single-line)
    let source = "/* sloc-guard:ignore-file */\nfn main() {}";
    let result = counter.count(source);

    // Should NOT be ignored (multi-line comments not supported for directive)
    match result {
        CountResult::Stats(stats) => {
            assert_eq!(stats.total, 2);
        }
        CountResult::IgnoredFile => {
            panic!("Should not be ignored, directive is in multi-line comment")
        }
    }
}

#[test]
fn ignore_file_directive_reader() {
    let syntax = rust_syntax();
    let counter = SlocCounter::new(&syntax);
    let source = "// sloc-guard:ignore-file\nfn main() {}";
    let result = counter.count_reader(Cursor::new(source)).unwrap();

    assert_eq!(result, CountResult::IgnoredFile);
}

#[test]
fn ignore_file_directive_with_leading_whitespace() {
    let syntax = rust_syntax();
    let counter = SlocCounter::new(&syntax);
    let source = "    // sloc-guard:ignore-file\nfn main() {}";
    let result = counter.count(source);

    assert_eq!(result, CountResult::IgnoredFile);
}

#[test]
fn no_ignore_directive_returns_stats() {
    let syntax = rust_syntax();
    let counter = SlocCounter::new(&syntax);
    let source = "// Regular comment\nfn main() {}";
    let stats = unwrap_stats(counter.count(source));

    assert_eq!(stats.total, 2);
    assert_eq!(stats.code, 1);
    assert_eq!(stats.comment, 1);
}
