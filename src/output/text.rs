use std::io::Write;

use crate::checker::{CheckResult, CheckStatus};
use crate::error::Result;

use super::OutputFormatter;

pub struct TextFormatter;

impl TextFormatter {
    #[must_use]
    pub const fn new(_use_colors: bool) -> Self {
        Self
    }

    const fn status_icon(status: &CheckStatus) -> &'static str {
        match status {
            CheckStatus::Passed => "✓",
            CheckStatus::Warning => "⚠",
            CheckStatus::Failed => "✗",
        }
    }

    fn format_result(result: &CheckResult, output: &mut Vec<u8>) {
        let icon = Self::status_icon(&result.status);
        let status_str = match result.status {
            CheckStatus::Passed => "PASSED",
            CheckStatus::Warning => "WARNING",
            CheckStatus::Failed => "FAILED",
        };

        writeln!(
            output,
            "{icon} {status_str}: {}",
            result.path.display()
        )
        .ok();

        writeln!(
            output,
            "   Lines: {} (limit: {})",
            result.stats.sloc(),
            result.limit
        )
        .ok();

        writeln!(
            output,
            "   Breakdown: code={}, comment={}, blank={}",
            result.stats.code, result.stats.comment, result.stats.blank
        )
        .ok();
    }
}

impl Default for TextFormatter {
    fn default() -> Self {
        Self::new(true)
    }
}

impl OutputFormatter for TextFormatter {
    fn format(&self, results: &[CheckResult]) -> Result<String> {
        let mut output = Vec::new();

        let (passed, warnings, failed): (Vec<_>, Vec<_>, Vec<_>) = results.iter().fold(
            (Vec::new(), Vec::new(), Vec::new()),
            |(mut p, mut w, mut f), r| {
                match r.status {
                    CheckStatus::Passed => p.push(r),
                    CheckStatus::Warning => w.push(r),
                    CheckStatus::Failed => f.push(r),
                }
                (p, w, f)
            },
        );

        for result in &failed {
            Self::format_result(result, &mut output);
            writeln!(output).ok();
        }

        for result in &warnings {
            Self::format_result(result, &mut output);
            writeln!(output).ok();
        }

        writeln!(
            output,
            "Summary: {} files checked, {} passed, {} warnings, {} failed",
            results.len(),
            passed.len(),
            warnings.len(),
            failed.len()
        )
        .ok();

        Ok(String::from_utf8_lossy(&output).to_string())
    }
}

#[cfg(test)]
#[path = "text_tests.rs"]
mod tests;
