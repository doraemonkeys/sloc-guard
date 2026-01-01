use std::io::BufRead;
use std::num::NonZeroUsize;

use crate::language::CommentSyntax;

use super::CommentDetector;

const IGNORE_FILE_DIRECTIVE: &str = "sloc-guard:ignore-file";
const IGNORE_NEXT_PREFIX: &str = "sloc-guard:ignore-next";
const IGNORE_START_DIRECTIVE: &str = "sloc-guard:ignore-start";
const IGNORE_END_DIRECTIVE: &str = "sloc-guard:ignore-end";
const DIRECTIVE_SCAN_LINES: usize = 10;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LineStats {
    pub total: usize,
    pub code: usize,
    pub comment: usize,
    pub blank: usize,
    pub ignored: usize,
}

impl LineStats {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            total: 0,
            code: 0,
            comment: 0,
            blank: 0,
            ignored: 0,
        }
    }

    #[must_use]
    pub const fn sloc(&self) -> usize {
        self.code
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CountResult {
    Stats(LineStats),
    IgnoredFile,
}

/// Tracks multi-line comment state, including nesting depth.
///
/// Uses an enum to make illegal states unrepresentable: when inside a comment,
/// all required fields (markers, nesting info) are guaranteed to exist.
#[derive(Debug, Clone, Default)]
enum MultiLineState {
    /// Not currently inside any multi-line comment
    #[default]
    NotInComment,
    /// Inside a multi-line comment with all required context
    InComment {
        /// Current nesting depth (guaranteed >= 1)
        depth: NonZeroUsize,
        /// Start marker for the current comment style
        start_marker: String,
        /// End marker for the current comment style
        end_marker: String,
        /// Whether current comment style supports nesting
        supports_nesting: bool,
    },
}

impl MultiLineState {
    const fn is_in_comment(&self) -> bool {
        matches!(self, Self::InComment { .. })
    }

    fn enter(&mut self, start: &str, end: &str, supports_nesting: bool) {
        match self {
            Self::NotInComment => {
                *self = Self::InComment {
                    depth: NonZeroUsize::MIN, // 1
                    start_marker: start.to_string(),
                    end_marker: end.to_string(),
                    supports_nesting,
                };
            }
            Self::InComment { depth, .. } => {
                // Increment depth, saturating to avoid overflow
                *depth = depth.saturating_add(1);
            }
        }
    }

    fn exit(&mut self) {
        if let Self::InComment { depth, .. } = self {
            if let Some(new_depth) = NonZeroUsize::new(depth.get() - 1) {
                *depth = new_depth;
            } else {
                // depth was 1, now 0 → exit comment
                *self = Self::NotInComment;
            }
        }
    }

    fn reset(&mut self) {
        *self = Self::NotInComment;
    }

    /// Extract markers and nesting info when inside a comment.
    /// Returns None if not in a comment (caller should handle appropriately).
    ///
    /// Returns owned Strings to avoid borrow conflicts when caller needs to mutate state
    /// after extracting marker info. The clone cost is acceptable since this is only
    /// called once per line (not in an inner loop).
    fn comment_info_owned(&self) -> Option<(String, String, bool)> {
        match self {
            Self::NotInComment => None,
            Self::InComment {
                start_marker,
                end_marker,
                supports_nesting,
                ..
            } => Some((start_marker.clone(), end_marker.clone(), *supports_nesting)),
        }
    }
}

pub struct SlocCounter<'a> {
    detector: CommentDetector<'a>,
}

impl<'a> SlocCounter<'a> {
    #[must_use]
    pub const fn new(syntax: &'a CommentSyntax) -> Self {
        Self {
            detector: CommentDetector::new(syntax),
        }
    }

    #[must_use]
    pub fn count(&self, source: &str) -> CountResult {
        let mut stats = LineStats::new();
        let mut multi_line_state = MultiLineState::default();
        let mut ignore_remaining: usize = 0;
        let mut in_ignore_block = false;

        for line in source.lines() {
            // Check for ignore directive in first N lines
            if stats.total < DIRECTIVE_SCAN_LINES && self.has_ignore_file_directive(line) {
                return CountResult::IgnoredFile;
            }

            self.process_line(
                line,
                &mut stats,
                &mut multi_line_state,
                &mut ignore_remaining,
                &mut in_ignore_block,
            );
        }

        CountResult::Stats(stats)
    }

    /// Count lines from byte content (converts to string with lossy UTF-8).
    #[must_use]
    pub fn count_from_bytes(&self, content: &[u8]) -> CountResult {
        let source = String::from_utf8_lossy(content);
        self.count(&source)
    }

    /// Count lines from a buffered reader (streaming, memory-efficient for large files).
    ///
    /// # Errors
    /// Returns an I/O error if reading from the reader fails.
    pub fn count_reader<R: BufRead>(&self, reader: R) -> std::io::Result<CountResult> {
        let mut stats = LineStats::new();
        let mut multi_line_state = MultiLineState::default();
        let mut ignore_remaining: usize = 0;
        let mut in_ignore_block = false;

        for line_result in reader.lines() {
            let line = line_result?;

            // Check for ignore directive in first N lines
            if stats.total < DIRECTIVE_SCAN_LINES && self.has_ignore_file_directive(&line) {
                return Ok(CountResult::IgnoredFile);
            }

            self.process_line(
                &line,
                &mut stats,
                &mut multi_line_state,
                &mut ignore_remaining,
                &mut in_ignore_block,
            );
        }

        Ok(CountResult::Stats(stats))
    }

    fn has_ignore_file_directive(&self, line: &str) -> bool {
        let trimmed = line.trim();
        if !trimmed.contains(IGNORE_FILE_DIRECTIVE) {
            return false;
        }
        // Directive must be in a comment
        self.detector.is_single_line_comment(trimmed)
    }

    fn parse_ignore_next(&self, trimmed: &str) -> Option<usize> {
        if !trimmed.contains(IGNORE_NEXT_PREFIX) {
            return None;
        }
        if !self.detector.is_single_line_comment(trimmed) {
            return None;
        }
        // Find the directive and parse the number
        let pos = trimmed.find(IGNORE_NEXT_PREFIX)?;
        let after = &trimmed[pos + IGNORE_NEXT_PREFIX.len()..];
        let num_str = after.split_whitespace().next()?;
        num_str.parse().ok()
    }

    fn has_ignore_start(&self, trimmed: &str) -> bool {
        trimmed.contains(IGNORE_START_DIRECTIVE) && self.detector.is_single_line_comment(trimmed)
    }

    fn has_ignore_end(&self, trimmed: &str) -> bool {
        trimmed.contains(IGNORE_END_DIRECTIVE) && self.detector.is_single_line_comment(trimmed)
    }

    fn process_line(
        &self,
        line: &str,
        stats: &mut LineStats,
        multi_line_state: &mut MultiLineState,
        ignore_remaining: &mut usize,
        in_ignore_block: &mut bool,
    ) {
        stats.total += 1;
        let trimmed = line.trim();

        // Check for ignore directives (only in single-line comments)
        if self.detector.is_single_line_comment(trimmed) {
            // Check for ignore-end first (to exit ignore block)
            if self.has_ignore_end(trimmed) {
                *in_ignore_block = false;
                stats.comment += 1;
                return;
            }
            // Check for ignore-start
            if self.has_ignore_start(trimmed) {
                *in_ignore_block = true;
                stats.comment += 1;
                return;
            }
            // Check for ignore-next N
            if let Some(n) = self.parse_ignore_next(trimmed) {
                *ignore_remaining = n;
                stats.comment += 1;
                return;
            }
        }

        // If we're in an ignore block or have remaining ignore lines, mark as ignored
        if *in_ignore_block {
            stats.ignored += 1;
            // Still need to track multi-line comment state for proper parsing after ignore block
            self.track_multi_line_comment_state(line, multi_line_state);
            return;
        }

        if *ignore_remaining > 0 {
            *ignore_remaining -= 1;
            stats.ignored += 1;
            // Still need to track multi-line comment state
            self.track_multi_line_comment_state(line, multi_line_state);
            return;
        }

        // Normal line classification
        if multi_line_state.is_in_comment() {
            stats.comment += 1;
            self.update_multi_line_state_inside_comment(line, multi_line_state);
            return;
        }

        if trimmed.is_empty() {
            stats.blank += 1;
            return;
        }

        // Check multi-line comment BEFORE single-line comment.
        // This is crucial for languages like Lua where `--[[` (multi-line) starts
        // with `--` (single-line prefix). Without this order, `--[[...` would be
        // incorrectly classified as a single-line comment.
        if let Some(matched) = self.detector.find_multi_line_start(line) {
            let comment = matched.comment;
            let start = &comment.start;
            // Use dynamic end marker for patterns like Lua long brackets (--[=[...]=])
            let end = matched.end_marker();

            if comment.supports_nesting {
                // For nested comments, count all starts and ends in the line
                let (open_count, close_count) =
                    self.detector.count_nesting_changes(line, start, end);
                // Apply nesting changes: first starts increase depth, then ends decrease
                for _ in 0..open_count {
                    multi_line_state.enter(start, end, true);
                }
                for _ in 0..close_count {
                    multi_line_state.exit();
                }
            } else {
                // Non-nested: simple check if line ends the comment
                if !self.detector.contains_multi_line_end(line, end) {
                    multi_line_state.enter(start, end, false);
                }
            }
            stats.comment += 1;
            return;
        }

        // Check single-line comment AFTER multi-line to handle overlapping prefixes
        // (e.g., Lua's `--` vs `--[[`)
        if self.detector.is_single_line_comment(trimmed) {
            stats.comment += 1;
            return;
        }

        stats.code += 1;
    }

    /// Update multi-line state when already inside a comment.
    ///
    /// The enum-based `MultiLineState` guarantees that when we're in a comment,
    /// all markers are available—no need for `debug_assert` or error handling.
    fn update_multi_line_state_inside_comment(&self, line: &str, state: &mut MultiLineState) {
        // Extract marker info (owned) before mutating state to avoid borrow conflicts.
        // With the enum-based MultiLineState, this is guaranteed to succeed
        // when is_in_comment() is true.
        let Some((start, end, supports_nesting)) = state.comment_info_owned() else {
            // Caller should only invoke this when is_in_comment() is true.
            // If we reach here, it's a logic error in the caller.
            return;
        };

        if supports_nesting {
            // Count nested starts and ends
            let (starts, ends) = self.detector.count_nesting_changes(line, &start, &end);
            for _ in 0..starts {
                state.enter(&start, &end, true);
            }
            for _ in 0..ends {
                state.exit();
            }
        } else {
            // Simple: check if line contains end marker
            if self.detector.contains_multi_line_end(line, &end) {
                state.reset();
            }
        }
    }

    fn track_multi_line_comment_state(&self, line: &str, state: &mut MultiLineState) {
        if state.is_in_comment() {
            self.update_multi_line_state_inside_comment(line, state);
        } else if let Some(matched) = self.detector.find_multi_line_start(line) {
            let comment = matched.comment;
            let start = &comment.start;
            // Use dynamic end marker for patterns like Lua long brackets (--[=[...]=])
            let end = matched.end_marker();

            if comment.supports_nesting {
                let (starts, ends) = self.detector.count_nesting_changes(line, start, end);
                for _ in 0..starts {
                    state.enter(start, end, true);
                }
                for _ in 0..ends {
                    state.exit();
                }
            } else if !self.detector.contains_multi_line_end(line, end) {
                state.enter(start, end, false);
            }
        }
    }
}

#[cfg(test)]
#[path = "sloc_tests/mod.rs"]
mod sloc_tests;
