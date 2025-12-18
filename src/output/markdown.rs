use std::fmt::Write;

use crate::checker::{CheckResult, CheckStatus};
use crate::error::Result;

use super::OutputFormatter;

pub struct MarkdownFormatter;

impl MarkdownFormatter {
    const fn status_icon(status: &CheckStatus) -> &'static str {
        match status {
            CheckStatus::Passed => "✅",
            CheckStatus::Warning => "⚠️",
            CheckStatus::Failed => "❌",
            CheckStatus::Grandfathered => "🔵",
        }
    }

    const fn status_text(status: &CheckStatus) -> &'static str {
        match status {
            CheckStatus::Passed => "Passed",
            CheckStatus::Warning => "Warning",
            CheckStatus::Failed => "Failed",
            CheckStatus::Grandfathered => "Grandfathered",
        }
    }
}

impl OutputFormatter for MarkdownFormatter {
    fn format(&self, results: &[CheckResult]) -> Result<String> {
        let mut output = String::new();

        // Count by status
        let (passed, warnings, failed, grandfathered) =
            results
                .iter()
                .fold((0, 0, 0, 0), |(p, w, f, g), r| match r.status {
                    CheckStatus::Passed => (p + 1, w, f, g),
                    CheckStatus::Warning => (p, w + 1, f, g),
                    CheckStatus::Failed => (p, w, f + 1, g),
                    CheckStatus::Grandfathered => (p, w, f, g + 1),
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
        let non_passed: Vec<_> = results
            .iter()
            .filter(|r| !matches!(r.status, CheckStatus::Passed))
            .collect();

        if !non_passed.is_empty() {
            writeln!(output, "### Details\n").ok();
            writeln!(
                output,
                "| Status | File | Lines | Limit | Code | Comment | Blank | Reason |"
            )
            .ok();
            writeln!(output, "|:------:|------|------:|------:|-----:|--------:|------:|--------|").ok();

            for result in &non_passed {
                let icon = Self::status_icon(&result.status);
                let status = Self::status_text(&result.status);
                let path = result.path.display();
                let sloc = result.stats.sloc();
                let limit = result.limit;
                let code = result.stats.code;
                let comment = result.stats.comment;
                let blank = result.stats.blank;
                let reason = result.override_reason.as_deref().unwrap_or("-");

                writeln!(
                    output,
                    "| {icon} {status} | `{path}` | {sloc} | {limit} | {code} | {comment} | {blank} | {reason} |"
                )
                .ok();
            }
        }

        Ok(output)
    }
}

#[cfg(test)]
#[path = "markdown_tests.rs"]
mod tests;
