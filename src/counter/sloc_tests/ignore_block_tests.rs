use super::*;

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
