use std::borrow::Cow;

use crate::language::{CommentSyntax, MultiLineComment, PatternKind};

/// Result of finding a multi-line comment start
#[derive(Debug, Clone)]
pub struct MultiLineMatch<'a> {
    pub comment: &'a MultiLineComment,
    pub position: usize,
    /// Dynamically computed end marker for patterns like Lua long brackets.
    /// If None, use `comment.end` as the static end marker.
    ///
    /// Uses `Cow` to avoid allocation when the dynamic end equals the static end
    /// (e.g., Lua level-0 brackets where end is "]]").
    pub dynamic_end: Option<Cow<'static, str>>,
}

impl MultiLineMatch<'_> {
    /// Get the end marker to use for this match
    #[must_use]
    pub fn end_marker(&self) -> &str {
        self.dynamic_end
            .as_ref()
            .map_or(self.comment.end.as_str(), |c| c.as_ref())
    }
}

pub struct CommentDetector<'a> {
    syntax: &'a CommentSyntax,
}

impl<'a> CommentDetector<'a> {
    #[must_use]
    pub const fn new(syntax: &'a CommentSyntax) -> Self {
        Self { syntax }
    }

    #[must_use]
    pub fn is_single_line_comment(&self, trimmed: &str) -> bool {
        self.syntax
            .single_line
            .iter()
            .any(|prefix| trimmed.starts_with(prefix))
    }

    /// Find multi-line comment start, returning the one that appears earliest in line.
    /// Also returns position for potential future use.
    ///
    /// # Performance Note
    /// This method allocates a `Vec<char>` for Unicode-aware character iteration.
    /// This is acceptable overhead because:
    /// - The allocation is short-lived (freed on return)
    /// - Called once per line with no reuse opportunity across calls
    /// - Required for correct handling of multi-byte UTF-8 characters in string detection
    #[must_use]
    pub fn find_multi_line_start(&self, line: &str) -> Option<MultiLineMatch<'a>> {
        let trimmed = line.trim_start();
        let chars: Vec<char> = line.chars().collect();

        let mut best_match: Option<MultiLineMatch<'a>> = None;

        // Pre-check if syntax includes Rust raw string pattern
        let skip_raw_strings = self
            .syntax
            .multi_line
            .iter()
            .any(|c| c.pattern_kind == PatternKind::RustRawString);

        for comment in &self.syntax.multi_line {
            // Check line-start constraint
            if comment.must_be_at_line_start {
                // For line-start markers, check if trimmed line starts with the marker
                if !trimmed.starts_with(&comment.start) {
                    continue;
                }
                // Position is where the marker actually starts (after whitespace)
                let leading_ws = line.len() - trimmed.len();
                let pos = leading_ws;
                if best_match.as_ref().is_none_or(|m| pos < m.position) {
                    best_match = Some(MultiLineMatch {
                        comment,
                        position: pos,
                        dynamic_end: None,
                    });
                }
                continue;
            }

            // Handle dynamic patterns
            match comment.pattern_kind {
                PatternKind::LuaLongBracket => {
                    // Search for --[=*[ pattern outside strings
                    if let Some((pos, dynamic_end)) =
                        find_lua_long_bracket_outside_string(&chars, true)
                        && best_match.as_ref().is_none_or(|m| pos < m.position)
                    {
                        best_match = Some(MultiLineMatch {
                            comment,
                            position: pos,
                            dynamic_end,
                        });
                    }
                }
                PatternKind::RustRawString => {
                    // Raw strings are NOT comments; they are used to skip content
                    // when looking for other comment markers. Skip them here.
                }
                PatternKind::Static => {
                    if let Some(pos) = find_outside_string(&chars, &comment.start, skip_raw_strings)
                        && best_match.as_ref().is_none_or(|m| pos < m.position)
                    {
                        best_match = Some(MultiLineMatch {
                            comment,
                            position: pos,
                            dynamic_end: None,
                        });
                    }
                }
            }
        }

        best_match
    }

    /// Try to match a Lua long bracket pattern at the given position.
    /// Returns `Some((matched_length, level))` if matched, where level is the number of `=` signs.
    fn match_lua_long_bracket(
        chars: &[char],
        pos: usize,
        require_dash_prefix: bool,
    ) -> Option<(usize, usize)> {
        let mut i = pos;

        // Check for optional `--` prefix (required for comments, not for strings)
        if require_dash_prefix {
            if i + 1 >= chars.len() || chars[i] != '-' || chars[i + 1] != '-' {
                return None;
            }
            i += 2;
        }

        // Must have `[`
        if i >= chars.len() || chars[i] != '[' {
            return None;
        }
        i += 1;

        // Count `=` signs
        let mut level = 0;
        while i < chars.len() && chars[i] == '=' {
            level += 1;
            i += 1;
        }

        // Must have closing `[`
        if i >= chars.len() || chars[i] != '[' {
            return None;
        }
        i += 1;

        Some((i - pos, level))
    }

    /// Try to match a Rust raw string pattern at the given position.
    /// Returns `Some((matched_length, level))` if matched, where level is the number of `#` signs.
    ///
    /// # Limitations
    /// - Byte raw strings (`br#"..."#`) are NOT currently supported. They are treated as
    ///   regular identifiers followed by raw strings. This is acceptable for comment detection
    ///   since byte strings don't contain comments.
    fn match_rust_raw_string(chars: &[char], pos: usize) -> Option<(usize, usize)> {
        let mut i = pos;

        // Must start with `r`
        // Note: We intentionally don't handle `br#"..."#` (byte raw strings) here.
        // This is acceptable because byte string contents can't contain comments.
        if i >= chars.len() || chars[i] != 'r' {
            return None;
        }
        i += 1;

        // Count `#` signs (can be zero)
        let mut level = 0;
        while i < chars.len() && chars[i] == '#' {
            level += 1;
            i += 1;
        }

        // Must have opening `"`
        if i >= chars.len() || chars[i] != '"' {
            return None;
        }
        i += 1;

        Some((i - pos, level))
    }

    /// Build the end marker for a Lua long bracket with the given level.
    fn lua_long_bracket_end(level: usize) -> String {
        let equals = "=".repeat(level);
        format!("]{equals}]")
    }

    /// Build the end marker for a Rust raw string with the given level.
    fn rust_raw_string_end(level: usize) -> String {
        let hashes = "#".repeat(level);
        format!("\"{hashes}")
    }

    /// Check if line contains end marker, respecting nesting if applicable.
    /// For nested comments, caller must track and pass current nesting depth.
    #[must_use]
    pub fn contains_multi_line_end(&self, line: &str, end_marker: &str) -> bool {
        find_outside_string_simple(line, end_marker).is_some()
    }

    /// Count nesting changes in a line for nested block comments.
    /// Returns `(starts_found, ends_found)` outside of strings.
    #[must_use]
    pub fn count_nesting_changes(
        &self,
        line: &str,
        start_marker: &str,
        end_marker: &str,
    ) -> (usize, usize) {
        count_markers_outside_string(line, start_marker, end_marker)
    }
}

/// Represents a string delimiter type for tracking string literal boundaries.
/// Uses a compact enum instead of heap-allocated Vec to avoid allocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StringDelimiter {
    /// Single quote: '
    Single,
    /// Double quote: "
    Double,
    /// Triple single quote: '''
    TripleSingle,
    /// Triple double quote: """
    TripleDouble,
}

impl StringDelimiter {
    /// Create delimiter from a character (single char variant).
    const fn from_char(c: char) -> Option<Self> {
        match c {
            '\'' => Some(Self::Single),
            '"' => Some(Self::Double),
            _ => None,
        }
    }

    /// Create delimiter from a character (triple char variant).
    const fn triple_from_char(c: char) -> Option<Self> {
        match c {
            '\'' => Some(Self::TripleSingle),
            '"' => Some(Self::TripleDouble),
            _ => None,
        }
    }

    /// Check if this single-char delimiter variant matches the given closing character.
    ///
    /// Returns `true` only for non-triple delimiters (`Single` or `Double`) when `c`
    /// matches the corresponding quote character. Triple delimiters always return `false`
    /// here—they require a 3-char sequence match handled separately.
    const fn matches_single(self, c: char) -> bool {
        matches!((self, c), (Self::Single, '\'') | (Self::Double, '"'))
    }

    /// Check if this is a triple-char delimiter.
    const fn is_triple(self) -> bool {
        matches!(self, Self::TripleSingle | Self::TripleDouble)
    }
}

/// State machine for tracking position while skipping string literals.
/// Used for counting comment markers outside of strings (nesting detection).
///
/// Uses `StringDelimiter` enum to avoid heap allocation per delimiter.
struct StringSkipper {
    in_string: bool,
    string_delim: Option<StringDelimiter>,
}

impl StringSkipper {
    const fn new() -> Self {
        Self {
            in_string: false,
            string_delim: None,
        }
    }

    /// Returns whether currently inside a string literal.
    const fn in_string(&self) -> bool {
        self.in_string
    }

    /// Core implementation for processing a character.
    /// Returns the number of chars consumed (always ≥ 1).
    ///
    /// When `track_single_quotes` is false, single-char quote delimiters
    /// are not tracked (used when searching for ''' or """ patterns).
    fn process_impl(&mut self, chars: &[char], i: usize, track_single_quotes: bool) -> usize {
        let c = chars[i];

        // Handle escape sequences inside strings
        if self.in_string && c == '\\' && i + 1 < chars.len() {
            return 2; // Skip escaped character
        }

        // Handle multi-char string delimiters (''' or """) for Python
        if (c == '"' || c == '\'')
            && i + 2 < chars.len()
            && chars[i + 1] == c
            && chars[i + 2] == c
            && let Some(triple_delim) = StringDelimiter::triple_from_char(c)
        {
            if !self.in_string {
                self.in_string = true;
                self.string_delim = Some(triple_delim);
                return 3;
            } else if self.string_delim == Some(triple_delim) {
                self.in_string = false;
                self.string_delim = None;
                return 3;
            }
        }

        // Handle single-char string delimiters (skip if searching for multi-char quote patterns)
        if track_single_quotes && let Some(single_delim) = StringDelimiter::from_char(c) {
            if !self.in_string {
                self.in_string = true;
                self.string_delim = Some(single_delim);
            } else if self
                .string_delim
                .is_some_and(|d| !d.is_triple() && d.matches_single(c))
            {
                self.in_string = false;
                self.string_delim = None;
            }
        }

        1
    }

    /// Process a character, returning the number of chars consumed (always ≥ 1).
    /// Updates internal string tracking state.
    fn process(&mut self, chars: &[char], i: usize) -> usize {
        let consumed = self.process_impl(chars, i, true);
        debug_assert!(consumed >= 1, "process() must consume at least 1 char");
        consumed
    }

    /// Process a character with special handling for multi-char quote needles.
    /// Returns the number of chars consumed (always ≥ 1).
    ///
    /// When `skip_single_quote_tracking` is true, single-char quote delimiters
    /// are not tracked (used when searching for ''' or """ patterns).
    fn process_with_quote_skip(
        &mut self,
        chars: &[char],
        i: usize,
        skip_single_quote_tracking: bool,
    ) -> usize {
        let consumed = self.process_impl(chars, i, !skip_single_quote_tracking);
        debug_assert!(
            consumed >= 1,
            "process_with_quote_skip() must consume at least 1 char"
        );
        consumed
    }
}

/// Find a Lua long bracket comment pattern (--[=*[) outside of string literals.
/// Returns `(byte_position, dynamic_end_marker)` if found.
///
/// The `dynamic_end` uses `Cow` to avoid allocation when the end marker matches
/// the static end (level-0 brackets where end is "]]").
///
/// Uses `StringSkipper` for consistent string handling (including triple quotes).
fn find_lua_long_bracket_outside_string(
    chars: &[char],
    require_dash_prefix: bool,
) -> Option<(usize, Option<Cow<'static, str>>)> {
    let mut skipper = StringSkipper::new();
    let mut i = 0;

    while i < chars.len() {
        // Check for Lua long bracket when NOT in a string
        if !skipper.in_string()
            && let Some((_matched_len, level)) =
                CommentDetector::match_lua_long_bracket(chars, i, require_dash_prefix)
        {
            let byte_pos: usize = chars[..i].iter().map(|ch| ch.len_utf8()).sum();
            // For level-0 (no = signs), the end marker is "]]" which matches the static end.
            // Use None to indicate we should use the static end marker, avoiding allocation.
            let dynamic_end = if level == 0 {
                None
            } else {
                Some(Cow::Owned(CommentDetector::lua_long_bracket_end(level)))
            };
            return Some((byte_pos, dynamic_end));
        }

        // Use StringSkipper for consistent string handling (handles triple quotes, escapes)
        i += skipper.process(chars, i);
    }

    None
}

/// Try to match and skip a Rust raw string at the given position.
/// Returns the number of characters to skip if matched, None otherwise.
fn try_skip_rust_raw_string(chars: &[char], pos: usize) -> Option<usize> {
    // Match r#*"
    let (start_len, level) = CommentDetector::match_rust_raw_string(chars, pos)?;

    // Find the closing "#*
    let end_marker = CommentDetector::rust_raw_string_end(level);
    let end_chars: Vec<char> = end_marker.chars().collect();

    let mut i = pos + start_len;
    while i < chars.len() {
        if chars[i..].starts_with(&end_chars) {
            return Some(i + end_chars.len() - pos);
        }
        i += 1;
    }

    // Raw string extends beyond line (multi-line raw string)
    Some(chars.len() - pos)
}

/// Find `needle` in `chars` only if it appears outside of string literals.
/// Returns the byte position if found, None otherwise.
///
/// Special handling for multi-character string delimiters like `'''` and `"""` (Python docstrings):
/// these are recognized as comment markers even though they use quote characters.
///
/// # Parameters
///
/// - `skip_raw_strings`: When `true`, Rust raw strings (`r#"..."#`) are skipped during search.
///   This is necessary because raw strings can contain sequences like `/*` that would otherwise
///   be falsely detected as comment starts. Pass `true` when the syntax includes `RustRawString`
///   in its multi-line patterns (auto-detected in `find_multi_line_start`), `false`
///   for other languages where `r#"` has no special meaning.
fn find_outside_string(chars: &[char], needle: &str, skip_raw_strings: bool) -> Option<usize> {
    if needle.is_empty() {
        return None;
    }

    let needle_chars: Vec<char> = needle.chars().collect();

    // Check if needle itself is a multi-char quote sequence (like ''' or """)
    let needle_is_multichar_quote = needle.len() >= 2 && {
        let first = needle_chars[0];
        (first == '"' || first == '\'') && needle_chars.iter().all(|&c| c == first)
    };

    let mut skipper = StringSkipper::new();
    let mut i = 0;

    while i < chars.len() {
        // When not in a string, check for Rust raw strings and skip them
        if skip_raw_strings
            && !skipper.in_string()
            && chars[i] == 'r'
            && let Some(skip_len) = try_skip_rust_raw_string(chars, i)
        {
            i += skip_len;
            continue;
        }

        // Check for needle match FIRST when NOT in a string
        if !skipper.in_string() && chars[i..].starts_with(&needle_chars) {
            let byte_pos: usize = chars[..i].iter().map(|ch| ch.len_utf8()).sum();
            return Some(byte_pos);
        }

        i += skipper.process_with_quote_skip(chars, i, needle_is_multichar_quote);
    }

    None
}

/// Wrapper for `find_outside_string` that takes a string slice and disables raw string skipping.
/// Used for end marker detection where we already know we're inside a comment.
fn find_outside_string_simple(line: &str, needle: &str) -> Option<usize> {
    let chars: Vec<char> = line.chars().collect();
    find_outside_string(&chars, needle, false)
}

/// Count occurrences of `start_marker` and `end_marker` outside string literals.
/// Used for tracking nesting depth in languages like Rust and Swift.
fn count_markers_outside_string(
    line: &str,
    start_marker: &str,
    end_marker: &str,
) -> (usize, usize) {
    if start_marker.is_empty() || end_marker.is_empty() {
        return (0, 0);
    }

    let chars: Vec<char> = line.chars().collect();
    let start_chars: Vec<char> = start_marker.chars().collect();
    let end_chars: Vec<char> = end_marker.chars().collect();

    let mut skipper = StringSkipper::new();
    let mut starts = 0;
    let mut ends = 0;
    let mut i = 0;

    while i < chars.len() {
        // Skip Rust raw strings when not in a regular string
        if !skipper.in_string()
            && chars[i] == 'r'
            && let Some(skip_len) = try_skip_rust_raw_string(&chars, i)
        {
            i += skip_len;
            continue;
        }

        // Check for markers FIRST when NOT in a string
        if !skipper.in_string() {
            if chars[i..].starts_with(&start_chars) {
                starts += 1;
                i += start_chars.len();
                continue;
            }
            if chars[i..].starts_with(&end_chars) {
                ends += 1;
                i += end_chars.len();
                continue;
            }
        }

        i += skipper.process(&chars, i);
    }

    (starts, ends)
}

#[cfg(test)]
#[path = "comment_tests/mod.rs"]
mod tests;
