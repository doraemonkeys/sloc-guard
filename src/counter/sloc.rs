use crate::language::CommentSyntax;

use super::CommentDetector;

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
    pub fn count(&self, source: &str) -> LineStats {
        let mut stats = LineStats::new();
        let mut in_multi_line_comment = false;
        let mut multi_line_end_marker: Option<&str> = None;

        for line in source.lines() {
            stats.total += 1;

            if in_multi_line_comment {
                stats.comment += 1;
                if let Some(end_marker) = multi_line_end_marker
                    && self.detector.contains_multi_line_end(line, end_marker)
                {
                    in_multi_line_comment = false;
                    multi_line_end_marker = None;
                }
                continue;
            }

            let trimmed = line.trim();

            if trimmed.is_empty() {
                stats.blank += 1;
                continue;
            }

            if self.detector.is_single_line_comment(line) {
                stats.comment += 1;
                continue;
            }

            if let Some((_, end)) = self.detector.find_multi_line_start(line) {
                if !self.detector.contains_multi_line_end(line, end) {
                    in_multi_line_comment = true;
                    multi_line_end_marker = Some(end);
                }
                stats.comment += 1;
                continue;
            }

            stats.code += 1;
        }

        stats
    }
}

#[cfg(test)]
#[path = "sloc_tests.rs"]
mod tests;
