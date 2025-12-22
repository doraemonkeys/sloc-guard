use super::*;

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
