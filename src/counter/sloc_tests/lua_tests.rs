//! Lua-specific `SlocCounter` tests, especially for long brackets with dynamic end markers.
//!
//! These tests verify two critical behaviors:
//! 1. Dynamic end markers (--[=[ uses ]=], not ]])
//! 2. Multi-line comment detection takes precedence over single-line (--[[ vs --)

use super::*;
use crate::language::LuaLongBracket;

fn lua_syntax_with_long_brackets() -> CommentSyntax {
    CommentSyntax::with_multi_line(vec!["--"], vec![LuaLongBracket::comment().into()])
}

// =============================================================================
// Level-0 long brackets (--[[ and ]])
// =============================================================================

#[test]
fn sloc_lua_level_0_bracket_single_line() {
    let syntax = lua_syntax_with_long_brackets();
    let counter = SlocCounter::new(&syntax);

    let source = "--[[ comment ]]\ncode";
    let stats = unwrap_stats(counter.count(source));

    assert_eq!(stats.total, 2);
    assert_eq!(stats.comment, 1);
    assert_eq!(stats.code, 1);
}

#[test]
fn sloc_lua_level_0_bracket_multiline() {
    let syntax = lua_syntax_with_long_brackets();
    let counter = SlocCounter::new(&syntax);

    // Level-0 spanning multiple lines
    // Previously failed due to --[[ being treated as single-line -- comment
    let source = "--[[ start\ncontinue\nend ]]\ncode";
    let stats = unwrap_stats(counter.count(source));

    assert_eq!(stats.total, 4);
    assert_eq!(stats.comment, 3);
    assert_eq!(stats.code, 1);
}

// =============================================================================
// Extended long brackets (--[=[ and ]=], --[==[ and ]==], etc.)
// =============================================================================
// These tests verify that SlocCounter correctly uses dynamic end markers.
// The bug was that `comment.end` (static "]]") was used instead of
// `matched.end_marker()` (dynamic "]=]", "]==]", etc.).

#[test]
fn sloc_lua_extended_bracket_single_line() {
    let syntax = lua_syntax_with_long_brackets();
    let counter = SlocCounter::new(&syntax);

    let source = "--[=[ this is a comment ]=]";
    let stats = unwrap_stats(counter.count(source));

    assert_eq!(stats.total, 1);
    assert_eq!(stats.comment, 1);
    assert_eq!(stats.code, 0);
}

#[test]
fn sloc_lua_extended_bracket_multiline() {
    let syntax = lua_syntax_with_long_brackets();
    let counter = SlocCounter::new(&syntax);

    // Level-1 long bracket spanning multiple lines
    let source = "--[=[ start of comment\ncontinuation\nend ]=]\nlocal x = 1";
    let stats = unwrap_stats(counter.count(source));

    assert_eq!(stats.total, 4);
    assert_eq!(stats.comment, 3);
    assert_eq!(stats.code, 1);
}

#[test]
fn sloc_lua_extended_bracket_wrong_end_not_closed() {
    let syntax = lua_syntax_with_long_brackets();
    let counter = SlocCounter::new(&syntax);

    // Level-1 opened with --[=[, but ]] (level-0) should NOT close it
    // This is the critical test for the dynamic end marker bug
    let source = "--[=[ comment with ]] in it\nstill in comment\nend ]=]\ncode";
    let stats = unwrap_stats(counter.count(source));

    // Lines 1-3: all comment (]] on line 1 does NOT close level-1 comment)
    // Line 4: code
    assert_eq!(stats.total, 4);
    assert_eq!(stats.comment, 3);
    assert_eq!(stats.code, 1);
}

#[test]
fn sloc_lua_extended_bracket_level_2() {
    let syntax = lua_syntax_with_long_brackets();
    let counter = SlocCounter::new(&syntax);

    // Level-2 long bracket: --[==[ ... ]==]
    // ]=] on line 2 should NOT close this level-2 comment
    let source = "--[==[ level 2\nstill comment ]=] not closed\nend ]==]\ncode";
    let stats = unwrap_stats(counter.count(source));

    assert_eq!(stats.total, 4);
    assert_eq!(stats.comment, 3);
    assert_eq!(stats.code, 1);
}

// =============================================================================
// Ignore directives with Lua long brackets
// =============================================================================

#[test]
fn sloc_lua_extended_bracket_in_ignore_block() {
    let syntax = lua_syntax_with_long_brackets();
    let counter = SlocCounter::new(&syntax);

    let source =
        "-- sloc-guard:ignore-start\n--[=[ comment\ncontinue ]=]\n-- sloc-guard:ignore-end\ncode";
    let stats = unwrap_stats(counter.count(source));

    // Line 1: comment (ignore-start directive)
    // Lines 2-3: ignored (in ignore block)
    // Line 4: comment (ignore-end directive)
    // Line 5: code
    assert_eq!(stats.total, 5);
    assert_eq!(stats.comment, 2);
    assert_eq!(stats.ignored, 2);
    assert_eq!(stats.code, 1);
}

#[test]
fn sloc_lua_extended_bracket_spanning_ignore_block() {
    let syntax = lua_syntax_with_long_brackets();
    let counter = SlocCounter::new(&syntax);

    // Multi-line comment starts in ignore block, ends after
    // Tests that track_multi_line_comment_state uses correct end marker
    let source = "-- sloc-guard:ignore-next 2\n--[=[ start\nmiddle\nend ]=]\ncode";
    let stats = unwrap_stats(counter.count(source));

    // Line 1: comment (ignore-next directive)
    // Lines 2-3: ignored (next 2 lines, but state tracking continues)
    // Line 4: comment (continuation of multi-line comment, ends with ]=])
    // Line 5: code
    assert_eq!(stats.total, 5);
    assert_eq!(stats.comment, 2);
    assert_eq!(stats.ignored, 2);
    assert_eq!(stats.code, 1);
}

// =============================================================================
// Static vs dynamic syntax comparison
// =============================================================================

#[test]
fn sloc_lua_static_syntax_multiline_works() {
    // Verify that static Lua syntax (without LuaLongBracket) also works
    // after the fix to check multi-line before single-line
    let static_syntax = CommentSyntax::new(vec!["--"], vec![("--[[", "]]")]);
    let counter = SlocCounter::new(&static_syntax);

    let source = "--[[ start\ncontinue\nend ]]\ncode";
    let stats = unwrap_stats(counter.count(source));

    assert_eq!(stats.total, 4);
    assert_eq!(stats.comment, 3);
    assert_eq!(stats.code, 1);
}
