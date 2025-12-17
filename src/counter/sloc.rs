use std::io::BufRead;

use crate::language::CommentSyntax;

use super::CommentDetector;

const IGNORE_FILE_DIRECTIVE: &str = "sloc-guard:ignore-file";
const DIRECTIVE_SCAN_LINES: usize = 10;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LineStats {
    pub total: usize,
    pub code: usize,
    pub comment: usize,
    pub blank: usize,
}

impl LineStats {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            total: 0,
            code: 0,
            comment: 0,
            blank: 0,
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
        let mut multi_line_end_marker: Option<&str> = None;

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
            );
        }

        CountResult::Stats(stats)
    }

    /// Count lines from a buffered reader (streaming, memory-efficient for large files).
    ///
    /// # Errors
    /// Returns an I/O error if reading from the reader fails.
    pub fn count_reader<R: BufRead>(&self, reader: R) -> std::io::Result<CountResult> {
        let mut stats = LineStats::new();
        let mut in_multi_line_comment = false;
        let mut multi_line_end_marker: Option<&str> = None;

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

    fn process_line(
        &self,
        line: &str,
        stats: &mut LineStats,
        in_multi_line_comment: &mut bool,
        multi_line_end_marker: &mut Option<&str>,
    ) {
        stats.total += 1;

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

        let trimmed = line.trim();

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
}

#[cfg(test)]
#[path = "sloc_tests.rs"]
mod tests;
