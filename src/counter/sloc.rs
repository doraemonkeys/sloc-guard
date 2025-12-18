use std::io::BufRead;

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
        let mut in_multi_line_comment = false;
        let mut multi_line_end_marker: Option<&'a str> = None;
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
                &mut in_multi_line_comment,
                &mut multi_line_end_marker,
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
        let mut in_multi_line_comment = false;
        let mut multi_line_end_marker: Option<&'a str> = None;
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
                &mut in_multi_line_comment,
                &mut multi_line_end_marker,
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
        in_multi_line_comment: &mut bool,
        multi_line_end_marker: &mut Option<&'a str>,
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
            self.track_multi_line_comment_state(line, in_multi_line_comment, multi_line_end_marker);
            return;
        }

        if *ignore_remaining > 0 {
            *ignore_remaining -= 1;
            stats.ignored += 1;
            // Still need to track multi-line comment state
            self.track_multi_line_comment_state(line, in_multi_line_comment, multi_line_end_marker);
            return;
        }

        // Normal line classification
        if *in_multi_line_comment {
            stats.comment += 1;
            if let Some(end_marker) = *multi_line_end_marker
                && self.detector.contains_multi_line_end(line, end_marker)
            {
                *in_multi_line_comment = false;
                *multi_line_end_marker = None;
            }
            return;
        }

        if trimmed.is_empty() {
            stats.blank += 1;
            return;
        }

        if self.detector.is_single_line_comment(trimmed) {
            stats.comment += 1;
            return;
        }

        if let Some((_, end)) = self.detector.find_multi_line_start(line) {
            if !self.detector.contains_multi_line_end(line, end) {
                *in_multi_line_comment = true;
                *multi_line_end_marker = Some(end);
            }
            stats.comment += 1;
            return;
        }

        stats.code += 1;
    }

    fn track_multi_line_comment_state(
        &self,
        line: &str,
        in_multi_line_comment: &mut bool,
        multi_line_end_marker: &mut Option<&'a str>,
    ) {
        if *in_multi_line_comment {
            if let Some(end_marker) = *multi_line_end_marker
                && self.detector.contains_multi_line_end(line, end_marker)
            {
                *in_multi_line_comment = false;
                *multi_line_end_marker = None;
            }
        } else if let Some((_, end)) = self.detector.find_multi_line_start(line)
            && !self.detector.contains_multi_line_end(line, end)
        {
            *in_multi_line_comment = true;
            *multi_line_end_marker = Some(end);
        }
    }
}

#[cfg(test)]
#[path = "sloc_tests.rs"]
mod tests;
