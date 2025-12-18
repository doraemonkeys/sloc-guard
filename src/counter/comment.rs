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
    pub fn is_single_line_comment(&self, trimmed: &str) -> bool {
        self.syntax
            .single_line
            .iter()
            .any(|prefix| trimmed.starts_with(prefix))
    }

    #[must_use]
    pub fn find_multi_line_start(&self, line: &str) -> Option<(&'a str, &'a str)> {
        for (start, end) in &self.syntax.multi_line {
            if line.contains(start.as_str()) {
                return Some((start.as_str(), end.as_str()));
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
