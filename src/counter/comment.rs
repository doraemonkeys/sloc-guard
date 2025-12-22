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
            if let Some(_pos) = find_outside_string(line, start) {
                return Some((start.as_str(), end.as_str()));
            }
        }
        None
    }

    #[must_use]
    pub fn contains_multi_line_end(&self, line: &str, end_marker: &str) -> bool {
        find_outside_string(line, end_marker).is_some()
    }
}

/// Find `needle` in `line` only if it appears outside of string literals.
/// Returns the byte position if found, None otherwise.
///
/// Special handling for multi-character string delimiters like `'''` and `"""` (Python docstrings):
/// these are recognized as comment markers even though they use quote characters.
fn find_outside_string(line: &str, needle: &str) -> Option<usize> {
    if needle.is_empty() {
        return None;
    }

    let chars: Vec<char> = line.chars().collect();
    let needle_chars: Vec<char> = needle.chars().collect();
    let mut in_string = false;
    let mut string_delim: Option<Vec<char>> = None;
    let mut i = 0;

    // Check if needle itself is a multi-char quote sequence (like ''' or """)
    let needle_is_multichar_quote = needle.len() >= 2
        && needle_chars.iter().all(|c| *c == '"' || *c == '\'')
        && needle_chars.iter().all(|c| *c == needle_chars[0]);

    while i < chars.len() {
        let c = chars[i];

        // Handle escape sequences inside strings
        if in_string && c == '\\' && i + 1 < chars.len() {
            i += 2; // Skip escaped character
            continue;
        }

        // Check for needle match FIRST when NOT in a string
        if !in_string && chars[i..].starts_with(&needle_chars) {
            let byte_pos: usize = chars[..i].iter().map(|ch| ch.len_utf8()).sum();
            return Some(byte_pos);
        }

        // Handle multi-char string delimiters (''' or """) for Python
        if (c == '"' || c == '\'') && i + 2 < chars.len() && chars[i + 1] == c && chars[i + 2] == c
        {
            let delim = vec![c, c, c];
            if !in_string {
                in_string = true;
                string_delim = Some(delim);
                i += 3;
                continue;
            } else if string_delim.as_ref() == Some(&delim) {
                in_string = false;
                string_delim = None;
                i += 3;
                continue;
            }
        }

        // Handle single-char string delimiters (skip for multi-char quote needles to avoid false positives)
        if (c == '"' || c == '\'') && !needle_is_multichar_quote {
            if !in_string {
                in_string = true;
                string_delim = Some(vec![c]);
            } else if string_delim.as_ref() == Some(&vec![c]) {
                in_string = false;
                string_delim = None;
            }
        }

        i += 1;
    }

    None
}

#[cfg(test)]
#[path = "comment_tests.rs"]
mod tests;
