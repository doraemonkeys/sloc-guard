use std::fmt::Write;
use std::io::Write as IoWrite;

use crate::checker::{CheckResult, CheckStatus};
use crate::error::Result;

use super::OutputFormatter;

/// Color output mode for terminal display.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ColorMode {
    /// Auto-detect: use colors if stdout is a TTY and `NO_COLOR` is not set
    #[default]
    Auto,
    /// Always use colors
    Always,
    /// Never use colors
    Never,
}

/// ANSI color codes
mod ansi {
    pub const RED: &str = "\x1b[31m";
    pub const GREEN: &str = "\x1b[32m";
    pub const YELLOW: &str = "\x1b[33m";
    pub const CYAN: &str = "\x1b[36m";
    pub const RESET: &str = "\x1b[0m";
}

pub struct TextFormatter {
    use_colors: bool,
    verbose: u8,
}

impl TextFormatter {
    #[must_use]
    pub fn new(mode: ColorMode) -> Self {
        Self::with_verbose(mode, 0)
    }

    #[must_use]
    pub fn with_verbose(mode: ColorMode, verbose: u8) -> Self {
        let use_colors = Self::should_use_colors(mode);
        Self { use_colors, verbose }
    }

    fn should_use_colors(mode: ColorMode) -> bool {
        match mode {
            ColorMode::Always => true,
            ColorMode::Never => false,
            ColorMode::Auto => {
                // Respect NO_COLOR environment variable
                if std::env::var("NO_COLOR").is_ok() {
                    return false;
                }
                // Check if stdout is a TTY
                std::io::IsTerminal::is_terminal(&std::io::stdout())
            }
        }
    }

    const fn status_icon(status: &CheckStatus) -> &'static str {
        match status {
            CheckStatus::Passed => "✓",
            CheckStatus::Warning => "⚠",
            CheckStatus::Failed => "✗",
            CheckStatus::Grandfathered => "◉",
        }
    }

    fn colorize(&self, text: &str, status: &CheckStatus) -> String {
        if !self.use_colors {
            return text.to_string();
        }

        let color = match status {
            CheckStatus::Passed => ansi::GREEN,
            CheckStatus::Warning => ansi::YELLOW,
            CheckStatus::Failed => ansi::RED,
            CheckStatus::Grandfathered => ansi::CYAN,
        };

        format!("{color}{text}{}", ansi::RESET)
    }

    fn colorize_number(&self, num: usize, status: &CheckStatus) -> String {
        self.colorize(&num.to_string(), status)
    }

    fn format_result(&self, result: &CheckResult, output: &mut Vec<u8>) {
        let icon = Self::status_icon(&result.status);
        let status_str = match result.status {
            CheckStatus::Passed => "PASSED",
            CheckStatus::Warning => "WARNING",
            CheckStatus::Failed => "FAILED",
            CheckStatus::Grandfathered => "GRANDFATHERED",
        };
        let colored_status = self.colorize(status_str, &result.status);

        writeln!(output, "{icon} {colored_status}: {}", result.path.display()).ok();

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

    fn format_summary(
        &self,
        total: usize,
        passed: usize,
        warnings: usize,
        failed: usize,
        grandfathered: usize,
    ) -> String {
        let passed_str = self.colorize_number(passed, &CheckStatus::Passed);
        let warnings_str = self.colorize_number(warnings, &CheckStatus::Warning);
        let failed_str = self.colorize_number(failed, &CheckStatus::Failed);

        let mut summary = format!(
            "Summary: {total} files checked, {passed_str} passed, {warnings_str} warnings, {failed_str} failed"
        );

        if grandfathered > 0 {
            let grandfathered_str = self.colorize_number(grandfathered, &CheckStatus::Grandfathered);
            let _ = write!(summary, " (baseline: {grandfathered_str} grandfathered)");
        }

        summary
    }
}

impl Default for TextFormatter {
    fn default() -> Self {
        Self::new(ColorMode::Auto)
    }
}

impl OutputFormatter for TextFormatter {
    fn format(&self, results: &[CheckResult]) -> Result<String> {
        let mut output = Vec::new();

        let (passed, warnings, failed, grandfathered): (Vec<_>, Vec<_>, Vec<_>, Vec<_>) =
            results.iter().fold(
                (Vec::new(), Vec::new(), Vec::new(), Vec::new()),
                |(mut p, mut w, mut f, mut g), r| {
                    match r.status {
                        CheckStatus::Passed => p.push(r),
                        CheckStatus::Warning => w.push(r),
                        CheckStatus::Failed => f.push(r),
                        CheckStatus::Grandfathered => g.push(r),
                    }
                    (p, w, f, g)
                },
            );

        for result in &failed {
            self.format_result(result, &mut output);
            writeln!(output).ok();
        }

        for result in &warnings {
            self.format_result(result, &mut output);
            writeln!(output).ok();
        }

        // Show grandfathered files in verbose mode
        if self.verbose >= 1 {
            for result in &grandfathered {
                self.format_result(result, &mut output);
                writeln!(output).ok();
            }
        }

        // Show passed files only in verbose mode
        if self.verbose >= 1 {
            for result in &passed {
                self.format_result(result, &mut output);
                writeln!(output).ok();
            }
        }

        let summary = self.format_summary(
            results.len(),
            passed.len(),
            warnings.len(),
            failed.len(),
            grandfathered.len(),
        );
        writeln!(output, "{summary}").ok();

        Ok(String::from_utf8_lossy(&output).to_string())
    }
}

#[cfg(test)]
#[path = "text_tests.rs"]
mod tests;
