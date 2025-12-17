use crate::language::CommentSyntax;

pub struct CommentDetector<'a> {
    syntax: &'a CommentSyntax,
}

impl<'a> CommentDetector<'a> {
    #[must_use]
    pub const fn new(syntax: &'a CommentSyntax) -> Self {
        Self { syntax }
    }

    #[must_use]
    pub fn is_single_line_comment(&self, line: &str) -> bool {
        let trimmed = line.trim();
        self.syntax
            .single_line
            .iter()
            .any(|prefix| trimmed.starts_with(prefix))
    }

    #[must_use]
    pub fn find_multi_line_start(&self, line: &str) -> Option<(&'static str, &'static str)> {
        for &(start, end) in &self.syntax.multi_line {
            if line.contains(start) {
                return Some((start, end));
            }
        }
        None
    }

    #[must_use]
    pub fn contains_multi_line_end(&self, line: &str, end_marker: &str) -> bool {
        line.contains(end_marker)
    }
}

#[cfg(test)]
#[path = "comment_tests.rs"]
mod tests;
