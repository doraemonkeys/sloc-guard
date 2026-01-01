//! Lua-specific comment tests including long brackets

use super::*;
use crate::language::LuaLongBracket;

#[test]
fn lua_multiline_comment_detection() {
    let syntax = lua_syntax();
    let detector = CommentDetector::new(&syntax);

    // Basic Lua multiline comment
    assert!(detector.find_multi_line_start("--[[ comment").is_some());
    assert!(detector.contains_multi_line_end("end of comment ]]", "]]"));
}

#[test]
fn lua_comment_marker_looks_like_array_access() {
    let syntax = lua_syntax();
    let detector = CommentDetector::new(&syntax);

    // ]] at end looks like Lua array access: t[x[y]]
    // When inside a --[[ comment, the ]] ends it
    let line = "t[x[y]]"; // NOT inside a comment, just code
    // But if we're searching for ]] end marker, this would match
    assert!(detector.contains_multi_line_end(line, "]]"));

    // This is a potential false positive when tracking multiline state
    // The SLOC counter needs context to know if we're in a comment
}

#[test]
fn lua_double_dash_not_multiline_start() {
    let syntax = lua_syntax();
    let detector = CommentDetector::new(&syntax);

    // Just -- is a single line comment, not --[[
    let line = "-- single line comment";
    assert!(detector.find_multi_line_start(line).is_none());
    assert!(detector.is_single_line_comment("-- single line comment"));
}

#[test]
fn lua_extended_comment_markers() {
    // Lua actually supports --[==[ and ]==] with matching = counts
    // Our simple parser doesn't handle this - document as limitation
    let syntax = lua_syntax();
    let detector = CommentDetector::new(&syntax);

    // --[=[ wouldn't be recognized as our --[[ marker
    let line = "--[=[ extended comment";
    assert!(detector.find_multi_line_start(line).is_none());
    // This is a LIMITATION: extended Lua comments not supported
}

// =============================================================================
// Extended long brackets (--[=[ and ]=]) with LuaLongBracket support
// =============================================================================
// Lua supports "long brackets" with varying levels of equals signs:
// - Level 0: --[[ ... ]]
// - Level 1: --[=[ ... ]=]
// - Level 2: --[==[ ... ]==]
// The number of = signs must match between open and close.

#[test]
fn lua_extended_comment_level_1_basic() {
    // --[=[ comment ]=] is a valid Lua comment

    let syntax = CommentSyntax::with_multi_line(vec!["--"], vec![LuaLongBracket::comment().into()]);
    let detector = CommentDetector::new(&syntax);

    // Basic level-1 long bracket
    let line = "--[=[ this is a level-1 comment";
    assert!(detector.find_multi_line_start(line).is_some());
}

#[test]
fn lua_extended_comment_level_1_end_detection() {
    let syntax = CommentSyntax::with_multi_line(vec!["--"], vec![LuaLongBracket::comment().into()]);
    let detector = CommentDetector::new(&syntax);

    // ]=] should end a level-1 comment
    // Need to match the dynamic end marker
    let line_end = "end of level-1 comment ]=]";
    assert!(detector.contains_multi_line_end(line_end, "]=]"));
}

#[test]
fn lua_extended_comment_level_2() {
    let syntax = CommentSyntax::with_multi_line(vec!["--"], vec![LuaLongBracket::comment().into()]);
    let detector = CommentDetector::new(&syntax);

    // Level-2 long bracket: --[==[ ... ]==]
    let line = "--[==[ level-2 comment";
    assert!(detector.find_multi_line_start(line).is_some());
}

#[test]
fn lua_level_mismatch_not_closed() {
    let syntax = CommentSyntax::with_multi_line(vec!["--"], vec![LuaLongBracket::comment().into()]);
    let detector = CommentDetector::new(&syntax);

    // --[=[ opened with level 1, ]] (level 0) should NOT close it
    // This test verifies level matching: ]] should not match ]=]
    assert!(!detector.contains_multi_line_end("text ]]", "]=]"));
}

#[test]
fn lua_long_string_not_comment() {
    let syntax = CommentSyntax::with_multi_line(vec!["--"], vec![LuaLongBracket::comment().into()]);
    let detector = CommentDetector::new(&syntax);

    // [=[ without -- prefix is a long string, not a comment
    let line = "local s = [=[ long string ]=]";
    assert!(detector.find_multi_line_start(line).is_none());
}

// =============================================================================
// High-level long brackets (testing arbitrary equals sign counts)
// =============================================================================

#[test]
fn lua_extended_comment_level_10() {
    let syntax = CommentSyntax::with_multi_line(vec!["--"], vec![LuaLongBracket::comment().into()]);
    let detector = CommentDetector::new(&syntax);

    // Level-10 long bracket: --[==========[ ... ]==========]
    let line = "--[==========[ level-10 comment";
    let result = detector.find_multi_line_start(line);
    assert!(result.is_some());

    let matched = result.unwrap();
    // Verify the dynamic end marker has 10 equals signs
    assert_eq!(matched.end_marker(), "]==========]");
}

#[test]
fn lua_extended_comment_level_20() {
    let syntax = CommentSyntax::with_multi_line(vec!["--"], vec![LuaLongBracket::comment().into()]);
    let detector = CommentDetector::new(&syntax);

    // Level-20 long bracket with 20 equals signs
    let equals = "=".repeat(20);
    let line = format!("--[{equals}[ level-20 comment");
    let result = detector.find_multi_line_start(&line);
    assert!(result.is_some());

    let matched = result.unwrap();
    let expected_end = format!("]{equals}]");
    assert_eq!(matched.end_marker(), expected_end);
}

#[test]
fn lua_extended_comment_high_level_end_detection() {
    let syntax = CommentSyntax::with_multi_line(vec!["--"], vec![LuaLongBracket::comment().into()]);
    let detector = CommentDetector::new(&syntax);

    // Verify high-level end markers are correctly detected
    let equals_15 = "=".repeat(15);
    let end_marker = format!("]{equals_15}]");
    let line = format!("end of level-15 comment {end_marker}");

    assert!(detector.contains_multi_line_end(&line, &end_marker));

    // Different level should NOT match
    let equals_14 = "=".repeat(14);
    let wrong_end = format!("]{equals_14}]");
    assert!(!detector.contains_multi_line_end(&line, &wrong_end));
}

#[test]
fn lua_extended_comment_level_mismatch_high_levels() {
    let syntax = CommentSyntax::with_multi_line(vec!["--"], vec![LuaLongBracket::comment().into()]);
    let detector = CommentDetector::new(&syntax);

    // Level 10 opened, level 9 end should NOT close it
    let equals_10 = "=".repeat(10);
    let equals_9 = "=".repeat(9);
    let end_10 = format!("]{equals_10}]");
    let end_9 = format!("]{equals_9}]");

    let line_with_wrong_end = format!("text {end_9}");
    assert!(!detector.contains_multi_line_end(&line_with_wrong_end, &end_10));

    // Correct level should match
    let line_with_correct_end = format!("text {end_10}");
    assert!(detector.contains_multi_line_end(&line_with_correct_end, &end_10));
}
