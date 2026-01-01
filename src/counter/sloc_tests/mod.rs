use super::*;
use std::io::Cursor;

mod counting_tests;
mod ignore_block_tests;
mod ignore_file_tests;
mod ignore_next_tests;
mod lua_tests;
mod nested_comment_tests;
mod python_tests;
mod string_literal_tests;

// Re-export shared test fixtures for submodules
pub(super) use crate::counter::test_fixtures::{
    python_syntax, rust_syntax, rust_syntax_with_nesting,
};

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
