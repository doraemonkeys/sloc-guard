use super::*;

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
