use std::fmt::Write;
use std::path::{Path, PathBuf};

use crate::checker::CheckResult;
use crate::error::Result;

use super::OutputFormatter;
use super::path::display_path;

pub struct MarkdownFormatter {
    show_suggestions: bool,
    project_root: Option<PathBuf>,
}

impl MarkdownFormatter {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            show_suggestions: false,
            project_root: None,
        }
    }

    #[must_use]
    pub const fn with_suggestions(mut self, show: bool) -> Self {
        self.show_suggestions = show;
        self
    }

    #[must_use]
    pub fn with_project_root(mut self, root: Option<PathBuf>) -> Self {
        self.project_root = root;
        self
    }

    fn display_path(&self, path: &Path) -> String {
        display_path(path, self.project_root.as_deref())
    }

    const fn status_icon(result: &CheckResult) -> &'static str {
        match result {
            CheckResult::Passed { .. } => "✅",
            CheckResult::Warning { .. } => "⚠️",
            CheckResult::Failed { .. } => "❌",
            CheckResult::Grandfathered { .. } => "🔵",
        }
    }

    const fn status_text(result: &CheckResult) -> &'static str {
        match result {
            CheckResult::Passed { .. } => "Passed",
            CheckResult::Warning { .. } => "Warning",
            CheckResult::Failed { .. } => "Failed",
            CheckResult::Grandfathered { .. } => "Grandfathered",
        }
    }
}

impl Default for MarkdownFormatter {
    fn default() -> Self {
        Self::new()
    }
}

impl OutputFormatter for MarkdownFormatter {
    fn format(&self, results: &[CheckResult]) -> Result<String> {
        let mut output = String::new();

        // Count by status
        let (passed, warnings, failed, grandfathered) =
            results
                .iter()
                .fold((0, 0, 0, 0), |(p, w, f, g), r| match r {
                    CheckResult::Passed { .. } => (p + 1, w, f, g),
                    CheckResult::Warning { .. } => (p, w + 1, f, g),
                    CheckResult::Failed { .. } => (p, w, f + 1, g),
                    CheckResult::Grandfathered { .. } => (p, w, f, g + 1),
                });

        // Summary section
        writeln!(output, "## SLOC Guard Results\n").ok();
        writeln!(output, "| Metric | Count |").ok();
        writeln!(output, "|--------|------:|").ok();
        writeln!(output, "| Total Files | {total} |", total = results.len()).ok();
        writeln!(output, "| ✅ Passed | {passed} |").ok();
        writeln!(output, "| ⚠️ Warnings | {warnings} |").ok();
        writeln!(output, "| ❌ Failed | {failed} |").ok();
        if grandfathered > 0 {
            writeln!(output, "| 🔵 Grandfathered | {grandfathered} |").ok();
        }
        writeln!(output).ok();

        // Only show detailed table if there are non-passed results
        let non_passed: Vec<_> = results.iter().filter(|r| !r.is_passed()).collect();

        if !non_passed.is_empty() {
            writeln!(output, "### Details\n").ok();
            writeln!(
                output,
                "| Status | File | Total | Lines | Limit | Code | Comment | Blank | Reason |"
            )
            .ok();
            writeln!(
                output,
                "|:------:|------|------:|------:|------:|-----:|--------:|------:|--------|"
            )
            .ok();

            for result in &non_passed {
                let icon = Self::status_icon(result);
                let status = Self::status_text(result);
                let path = self.display_path(result.path());
                // Use raw_stats for display (before skip_comments/skip_blank adjustments)
                let raw = result.raw_stats();
                let total = raw.total;
                let sloc = result.stats().sloc();
                let limit = result.limit();
                let code = raw.code;
                let comment = raw.comment;
                let blank = raw.blank;
                let reason = result.override_reason().unwrap_or("-");

                writeln!(
                    output,
                    "| {icon} {status} | `{path}` | {total} | {sloc} | {limit} | {code} | {comment} | {blank} | {reason} |"
                )
                .ok();
            }

            // Show split suggestions if enabled
            if self.show_suggestions {
                let with_suggestions: Vec<_> = non_passed
                    .iter()
                    .filter(|r| {
                        r.suggestions()
                            .is_some_and(crate::analyzer::SplitSuggestion::has_suggestions)
                    })
                    .collect();

                if !with_suggestions.is_empty() {
                    writeln!(output).ok();
                    writeln!(output, "### Split Suggestions\n").ok();

                    for result in with_suggestions {
                        if let Some(suggestion) = result.suggestions() {
                            writeln!(output, "#### `{}`\n", self.display_path(result.path())).ok();
                            writeln!(output, "| Suggested File | Lines | Functions |").ok();
                            writeln!(output, "|----------------|------:|-----------|").ok();

                            for chunk in &suggestion.chunks {
                                let funcs = if chunk.functions.is_empty() {
                                    "-".to_string()
                                } else {
                                    chunk.functions.join(", ")
                                };
                                writeln!(
                                    output,
                                    "| `{}.*` | ~{} | {} |",
                                    chunk.suggested_name, chunk.line_count, funcs
                                )
                                .ok();
                            }
                            writeln!(output).ok();
                        }
                    }
                }
            }
        }

        Ok(output)
    }
}

#[cfg(test)]
#[path = "markdown_tests.rs"]
mod tests;
