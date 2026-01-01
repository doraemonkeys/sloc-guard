//! Comment detection test suite.
//!
//! Tests are organized into submodules by category:
//! - `detection_tests`: Basic single-line and multi-line comment detection
//! - `string_context_tests`: Comment markers inside string literals (should not be detected)
//! - `edge_case_tests`: Escapes, unicode, quotes, backslashes, complex patterns
//! - `raw_string_tests`: Rust raw string handling and known limitations
//! - `lua_tests`: Lua-specific tests including long brackets
//! - `python_docstring_tests`: Python triple-quoted string tests
//! - `ruby_tests`: Ruby-specific =begin/=end tests
//! - `sql_tests`: SQL-specific comment tests
//! - `nested_comment_tests`: Rust/Swift nested block comment tests

use super::*;

mod detection_tests;
mod edge_case_tests;
mod lua_tests;
mod nested_comment_tests;
mod python_docstring_tests;
mod raw_string_tests;
mod ruby_tests;
mod sql_tests;
mod string_context_tests;

// Re-export shared test fixtures for submodules
pub(super) use crate::counter::test_fixtures::{
    lua_syntax, python_syntax, ruby_syntax, rust_syntax, rust_syntax_with_raw_string, sql_syntax,
};
