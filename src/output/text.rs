use std::fmt::Write;
use std::io::Write as IoWrite;

use crate::checker::CheckResult;
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
    show_suggestions: bool,
}

impl TextFormatter {
    #[must_use]
    pub fn new(mode: ColorMode) -> Self {
        Self::with_verbose(mode, 0)
    }

    #[must_use]
    pub fn with_verbose(mode: ColorMode, verbose: u8) -> Self {
        let use_colors = Self::should_use_colors(mode);
        Self {
            use_colors,
            verbose,
            show_suggestions: false,
        }
    }

    #[must_use]
    pub const fn with_suggestions(mut self, show: bool) -> Self {
        self.show_suggestions = show;
        self
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

    const fn status_icon(result: &CheckResult) -> &'static str {
        match result {
            CheckResult::Passed { .. } => "✓",
            CheckResult::Warning { .. } => "⚠",
            CheckResult::Failed { .. } => "✗",
            CheckResult::Grandfathered { .. } => "◉",
        }
    }

    fn colorize(&self, text: &str, result: &CheckResult) -> String {
        if !self.use_colors {
            return text.to_string();
        }

        let color = match result {
            CheckResult::Passed { .. } => ansi::GREEN,
            CheckResult::Warning { .. } => ansi::YELLOW,
            CheckResult::Failed { .. } => ansi::RED,
            CheckResult::Grandfathered { .. } => ansi::CYAN,
        };

        format!("{color}{text}{}", ansi::RESET)
    }

    fn format_result(&self, result: &CheckResult, output: &mut Vec<u8>) {
        let icon = Self::status_icon(result);
        let status_str = match result {
            CheckResult::Passed { .. } => "PASSED",
            CheckResult::Warning { .. } => "WARNING",
            CheckResult::Failed { .. } => "FAILED",
            CheckResult::Grandfathered { .. } => "GRANDFATHERED",
        };
        let colored_status = self.colorize(status_str, result);

        writeln!(
            output,
            "{icon} {colored_status}: {}",
            result.path().display()
        )
        .ok();

        let reason = result.override_reason();
        let is_structure_files = reason.is_some_and(|r| r.contains("structure: files"));
        let is_structure_dirs = reason.is_some_and(|r| r.contains("structure: subdirs"));

        if is_structure_files {
            writeln!(
                output,
                "   Files: {} (limit: {})",
                result.stats().sloc(),
                result.limit()
            )
            .ok();
        } else if is_structure_dirs {
            writeln!(
                output,
                "   Directories: {} (limit: {})",
                result.stats().sloc(),
                result.limit()
            )
            .ok();
        } else {
            writeln!(
                output,
                "   Lines: {} (limit: {})",
                result.stats().sloc(),
                result.limit()
            )
            .ok();

            writeln!(
                output,
                "   Breakdown: code={}, comment={}, blank={}",
                result.stats().code,
                result.stats().comment,
                result.stats().blank
            )
            .ok();
        }

        // Show override reason if present (in verbose mode or for any status)
        if let Some(r) = reason {
            writeln!(output, "   Reason: {r}").ok();
        }

        // Show split suggestions if enabled and available
        if self.show_suggestions
            && let Some(suggestion) = result.suggestions()
            && suggestion.has_suggestions()
        {
            Self::format_suggestions(suggestion, output);
        }
    }

    fn format_suggestions(suggestion: &crate::analyzer::SplitSuggestion, output: &mut Vec<u8>) {
        writeln!(output, "   Split suggestions:").ok();
        for chunk in &suggestion.chunks {
            writeln!(
                output,
                "     → {}.* (lines {}-{}, ~{} lines)",
                chunk.suggested_name, chunk.start_line, chunk.end_line, chunk.line_count
            )
            .ok();
            if !chunk.functions.is_empty() {
                let funcs = chunk.functions.join(", ");
                writeln!(output, "       Functions: {funcs}").ok();
            }
        }
    }

    fn format_summary(
        &self,
        total: usize,
        passed: usize,
        warnings: usize,
        failed: usize,
        grandfathered: usize,
    ) -> String {
        let passed_str = self.colorize_with_color(&passed.to_string(), ansi::GREEN);
        let warnings_str = self.colorize_with_color(&warnings.to_string(), ansi::YELLOW);
        let failed_str = self.colorize_with_color(&failed.to_string(), ansi::RED);

        let mut summary = format!(
            "Summary: {total} files checked, {passed_str} passed, {warnings_str} warnings, {failed_str} failed"
        );

        if grandfathered > 0 {
            let grandfathered_str =
                self.colorize_with_color(&grandfathered.to_string(), ansi::CYAN);
            let _ = write!(summary, " (baseline: {grandfathered_str} grandfathered)");
        }

        summary
    }

    fn colorize_with_color(&self, text: &str, color: &str) -> String {
        if !self.use_colors {
            return text.to_string();
        }
        format!("{color}{text}{}", ansi::RESET)
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
                    match r {
                        CheckResult::Passed { .. } => p.push(r),
                        CheckResult::Warning { .. } => w.push(r),
                        CheckResult::Failed { .. } => f.push(r),
                        CheckResult::Grandfathered { .. } => g.push(r),
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
