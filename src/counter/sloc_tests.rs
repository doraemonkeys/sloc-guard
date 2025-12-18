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
        blank: 5, ignored: 0,
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

// =============================================================================
// Tests for ignore-next N directive
// =============================================================================

#[test]
fn ignore_next_skips_specified_lines() {
    let syntax = rust_syntax();
    let counter = SlocCounter::new(&syntax);
    let source = r"fn main() {
    // sloc-guard:ignore-next 2
    let generated1 = 1;
    let generated2 = 2;
    let real_code = 3;
}";
    let stats = unwrap_stats(counter.count(source));

    assert_eq!(stats.total, 6);
    assert_eq!(stats.code, 3); // main, real_code, closing brace
    assert_eq!(stats.comment, 1); // the ignore-next directive
    assert_eq!(stats.ignored, 2); // generated1, generated2
}

#[test]
fn ignore_next_one_line() {
    let syntax = rust_syntax();
    let counter = SlocCounter::new(&syntax);
    let source = r"// sloc-guard:ignore-next 1
let ignored = 1;
let counted = 2;";
    let stats = unwrap_stats(counter.count(source));

    assert_eq!(stats.total, 3);
    assert_eq!(stats.code, 1); // counted
    assert_eq!(stats.comment, 1); // ignore-next directive
    assert_eq!(stats.ignored, 1); // ignored
}

#[test]
fn ignore_next_zero_ignores_nothing() {
    let syntax = rust_syntax();
    let counter = SlocCounter::new(&syntax);
    let source = r"// sloc-guard:ignore-next 0
let code = 1;";
    let stats = unwrap_stats(counter.count(source));

    assert_eq!(stats.total, 2);
    assert_eq!(stats.code, 1);
    assert_eq!(stats.comment, 1);
    assert_eq!(stats.ignored, 0);
}

#[test]
fn ignore_next_more_than_remaining_lines() {
    let syntax = rust_syntax();
    let counter = SlocCounter::new(&syntax);
    let source = r"// sloc-guard:ignore-next 100
let only_two = 1;
let lines_left = 2;";
    let stats = unwrap_stats(counter.count(source));

    assert_eq!(stats.total, 3);
    assert_eq!(stats.code, 0); // all code lines ignored
    assert_eq!(stats.comment, 1); // ignore-next directive
    assert_eq!(stats.ignored, 2);
}

#[test]
fn ignore_next_with_extra_text() {
    let syntax = rust_syntax();
    let counter = SlocCounter::new(&syntax);
    let source = r"// sloc-guard:ignore-next 1 - generated code
let ignored = 1;
let counted = 2;";
    let stats = unwrap_stats(counter.count(source));

    assert_eq!(stats.code, 1);
    assert_eq!(stats.ignored, 1);
}

#[test]
fn ignore_next_python_style() {
    let syntax = python_syntax();
    let counter = SlocCounter::new(&syntax);
    let source = r"# sloc-guard:ignore-next 2
generated_var1 = 1
generated_var2 = 2
real_code = 3";
    let stats = unwrap_stats(counter.count(source));

    assert_eq!(stats.code, 1); // real_code
    assert_eq!(stats.ignored, 2);
}

#[test]
fn ignore_next_without_number_ignored() {
    let syntax = rust_syntax();
    let counter = SlocCounter::new(&syntax);
    // No number after ignore-next should be ignored (treated as regular comment)
    let source = r"// sloc-guard:ignore-next
let code = 1;";
    let stats = unwrap_stats(counter.count(source));

    assert_eq!(stats.code, 1);
    assert_eq!(stats.comment, 1);
    assert_eq!(stats.ignored, 0);
}

#[test]
fn ignore_next_with_invalid_number() {
    let syntax = rust_syntax();
    let counter = SlocCounter::new(&syntax);
    // Invalid number should be ignored (treated as regular comment)
    let source = r"// sloc-guard:ignore-next abc
let code = 1;";
    let stats = unwrap_stats(counter.count(source));

    assert_eq!(stats.code, 1);
    assert_eq!(stats.comment, 1);
    assert_eq!(stats.ignored, 0);
}

#[test]
fn multiple_ignore_next_directives() {
    let syntax = rust_syntax();
    let counter = SlocCounter::new(&syntax);
    let source = r"// sloc-guard:ignore-next 1
let ignored1 = 1;
let counted = 2;
// sloc-guard:ignore-next 1
let ignored2 = 3;
let also_counted = 4;";
    let stats = unwrap_stats(counter.count(source));

    assert_eq!(stats.code, 2); // counted, also_counted
    assert_eq!(stats.comment, 2); // two ignore-next directives
    assert_eq!(stats.ignored, 2); // ignored1, ignored2
}

// =============================================================================
// Tests for ignore-start/ignore-end block directive
// =============================================================================

#[test]
fn ignore_block_skips_lines_between_markers() {
    let syntax = rust_syntax();
    let counter = SlocCounter::new(&syntax);
    let source = r"fn main() {
    // sloc-guard:ignore-start
    let gen1 = 1;
    let gen2 = 2;
    let gen3 = 3;
    // sloc-guard:ignore-end
    let real = 4;
}";
    let stats = unwrap_stats(counter.count(source));

    assert_eq!(stats.total, 8);
    assert_eq!(stats.code, 3); // main, real, closing brace
    assert_eq!(stats.comment, 2); // ignore-start, ignore-end
    assert_eq!(stats.ignored, 3); // gen1, gen2, gen3
}

#[test]
fn ignore_block_empty() {
    let syntax = rust_syntax();
    let counter = SlocCounter::new(&syntax);
    let source = r"// sloc-guard:ignore-start
// sloc-guard:ignore-end
let code = 1;";
    let stats = unwrap_stats(counter.count(source));

    assert_eq!(stats.code, 1);
    assert_eq!(stats.comment, 2);
    assert_eq!(stats.ignored, 0);
}

#[test]
fn ignore_block_no_end_ignores_rest_of_file() {
    let syntax = rust_syntax();
    let counter = SlocCounter::new(&syntax);
    let source = r"let before = 1;
// sloc-guard:ignore-start
let ignored1 = 2;
let ignored2 = 3;";
    let stats = unwrap_stats(counter.count(source));

    assert_eq!(stats.code, 1); // before
    assert_eq!(stats.comment, 1); // ignore-start
    assert_eq!(stats.ignored, 2); // ignored1, ignored2
}

#[test]
fn ignore_block_python_style() {
    let syntax = python_syntax();
    let counter = SlocCounter::new(&syntax);
    let source = r"before = 1
# sloc-guard:ignore-start
generated = 2
more_generated = 3
# sloc-guard:ignore-end
after = 4";
    let stats = unwrap_stats(counter.count(source));

    assert_eq!(stats.code, 2); // before, after
    assert_eq!(stats.ignored, 2); // generated, more_generated
}

#[test]
fn ignore_block_with_extra_text() {
    let syntax = rust_syntax();
    let counter = SlocCounter::new(&syntax);
    let source = r"// sloc-guard:ignore-start - BEGIN GENERATED
let gen = 1;
// sloc-guard:ignore-end - END GENERATED
let code = 2;";
    let stats = unwrap_stats(counter.count(source));

    assert_eq!(stats.code, 1);
    assert_eq!(stats.ignored, 1);
}

#[test]
fn multiple_ignore_blocks() {
    let syntax = rust_syntax();
    let counter = SlocCounter::new(&syntax);
    let source = r"let code1 = 1;
// sloc-guard:ignore-start
let gen1 = 2;
// sloc-guard:ignore-end
let code2 = 3;
// sloc-guard:ignore-start
let gen2 = 4;
// sloc-guard:ignore-end
let code3 = 5;";
    let stats = unwrap_stats(counter.count(source));

    assert_eq!(stats.code, 3); // code1, code2, code3
    assert_eq!(stats.comment, 4); // two pairs of start/end
    assert_eq!(stats.ignored, 2); // gen1, gen2
}

#[test]
fn ignore_block_with_comments_inside() {
    let syntax = rust_syntax();
    let counter = SlocCounter::new(&syntax);
    let source = r"// sloc-guard:ignore-start
// This is a comment inside ignore block
let gen = 1;
/* multi-line
   comment */
// sloc-guard:ignore-end
let code = 2;";
    let stats = unwrap_stats(counter.count(source));

    assert_eq!(stats.code, 1); // code
    assert_eq!(stats.ignored, 4); // comment, gen, multi-line (2 lines)
}

#[test]
fn ignore_block_with_blank_lines() {
    let syntax = rust_syntax();
    let counter = SlocCounter::new(&syntax);
    let source = r"// sloc-guard:ignore-start

let gen = 1;

// sloc-guard:ignore-end
let code = 2;";
    let stats = unwrap_stats(counter.count(source));

    assert_eq!(stats.code, 1);
    assert_eq!(stats.ignored, 3); // blank, gen, blank
}

// =============================================================================
// Tests combining ignore-next and ignore-block
// =============================================================================

#[test]
fn ignore_next_then_ignore_block() {
    let syntax = rust_syntax();
    let counter = SlocCounter::new(&syntax);
    let source = r"// sloc-guard:ignore-next 1
let ignored_by_next = 1;
// sloc-guard:ignore-start
let ignored_by_block = 2;
// sloc-guard:ignore-end
let code = 3;";
    let stats = unwrap_stats(counter.count(source));

    assert_eq!(stats.code, 1);
    assert_eq!(stats.ignored, 2);
}

// =============================================================================
// Tests for count_reader with new directives
// =============================================================================

#[test]
fn count_reader_ignore_next() {
    let syntax = rust_syntax();
    let counter = SlocCounter::new(&syntax);
    let source = r"// sloc-guard:ignore-next 1
let ignored = 1;
let counted = 2;";
    let stats = unwrap_stats_reader(counter.count_reader(Cursor::new(source)));

    assert_eq!(stats.code, 1);
    assert_eq!(stats.ignored, 1);
}

#[test]
fn count_reader_ignore_block() {
    let syntax = rust_syntax();
    let counter = SlocCounter::new(&syntax);
    let source = r"// sloc-guard:ignore-start
let ignored = 1;
// sloc-guard:ignore-end
let code = 2;";
    let stats = unwrap_stats_reader(counter.count_reader(Cursor::new(source)));

    assert_eq!(stats.code, 1);
    assert_eq!(stats.ignored, 1);
}

// =============================================================================
// Tests for ignored field in LineStats
// =============================================================================

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
