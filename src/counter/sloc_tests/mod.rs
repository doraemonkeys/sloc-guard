use super::*;
use std::io::Cursor;

mod counting_tests;
mod ignore_block_tests;
mod ignore_file_tests;
mod ignore_next_tests;
mod nested_comment_tests;
mod python_tests;
mod string_literal_tests;

pub(super) fn rust_syntax() -> CommentSyntax {
    CommentSyntax::new(vec!["//", "///", "//!"], vec![("/*", "*/")])
}

pub(super) fn python_syntax() -> CommentSyntax {
    CommentSyntax::new(vec!["#"], vec![("\"\"\"", "\"\"\""), ("'''", "'''")])
}

pub(super) fn unwrap_stats(result: CountResult) -> LineStats {
    match result {
        CountResult::Stats(stats) => stats,
        CountResult::IgnoredFile => panic!("Expected Stats, got IgnoredFile"),
    }
}

pub(super) fn unwrap_stats_reader(result: std::io::Result<CountResult>) -> LineStats {
    match result.unwrap() {
        CountResult::Stats(stats) => stats,
        CountResult::IgnoredFile => panic!("Expected Stats, got IgnoredFile"),
    }
}
