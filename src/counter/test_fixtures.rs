//! Shared test fixtures for counter tests.
//!
//! Provides common `CommentSyntax` configurations used across both
//! `comment_tests` and `sloc_tests` modules.

use crate::language::{CommentSyntax, MultiLineComment, RustRawString};

/// Rust syntax without nesting support (for backward-compatible tests)
pub fn rust_syntax() -> CommentSyntax {
    CommentSyntax::new(vec!["//", "///", "//!"], vec![("/*", "*/")])
}

/// Rust syntax WITH nesting support (actual Rust behavior)
pub fn rust_syntax_with_nesting() -> CommentSyntax {
    CommentSyntax::with_multi_line(
        vec!["//", "///", "//!"],
        vec![MultiLineComment::new("/*", "*/").with_nesting()],
    )
}

/// Rust syntax WITH `RustRawString` enabled (full production behavior)
pub fn rust_syntax_with_raw_string() -> CommentSyntax {
    CommentSyntax::with_multi_line(
        vec!["//", "///", "//!"],
        vec![
            MultiLineComment::new("/*", "*/").with_nesting(),
            RustRawString::new().into(),
        ],
    )
}

/// Python syntax with triple-quoted strings as multi-line comments
pub fn python_syntax() -> CommentSyntax {
    CommentSyntax::new(vec!["#"], vec![("\"\"\"", "\"\"\""), ("'''", "'''")])
}

/// Lua syntax with long bracket comments
pub fn lua_syntax() -> CommentSyntax {
    CommentSyntax::new(vec!["--"], vec![("--[[", "]]")])
}

/// Ruby syntax with =begin/=end multi-line comments
pub fn ruby_syntax() -> CommentSyntax {
    CommentSyntax::new(vec!["#"], vec![("=begin", "=end")])
}

/// SQL syntax with -- single-line and /* */ multi-line comments
pub fn sql_syntax() -> CommentSyntax {
    CommentSyntax::new(vec!["--"], vec![("/*", "*/")])
}
