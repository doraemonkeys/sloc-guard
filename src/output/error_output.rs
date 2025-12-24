//! Unified error and warning output formatting with color support.
//!
//! Provides consistent, colored error messages with actionable suggestions.
//! Format: ✖ Error Type / × Detail / help: Suggestion

use std::io::{IsTerminal, Write};

use super::ColorMode;
use super::ansi;

/// Error output formatter with color support.
pub struct ErrorOutput {
    use_colors: bool,
}

impl ErrorOutput {
    /// Creates a new error output formatter with the specified color mode.
    #[must_use]
    #[allow(dead_code)] // Public API for explicit mode control
    pub fn new(mode: ColorMode) -> Self {
        Self {
            use_colors: Self::should_use_colors(mode),
        }
    }

    /// Creates an error output formatter that auto-detects color support on stderr.
    #[must_use]
    pub fn stderr() -> Self {
        Self {
            use_colors: Self::stderr_supports_color(),
        }
    }

    #[allow(dead_code)] // Used by new() which is public API
    fn should_use_colors(mode: ColorMode) -> bool {
        match mode {
            ColorMode::Always => true,
            ColorMode::Never => false,
            ColorMode::Auto => Self::stderr_supports_color(),
        }
    }

    fn stderr_supports_color() -> bool {
        // Respect NO_COLOR environment variable (https://no-color.org/)
        if Self::is_no_color_set() {
            return false;
        }
        // Check if stderr is a TTY
        std::io::stderr().is_terminal()
    }

    /// Prints an error message with consistent formatting.
    ///
    /// Format: `✖ {error_type}: {message}`
    ///         `  × {detail}` (optional)
    ///         `  help: {suggestion}` (optional)
    pub fn print_error(&self, error_type: &str, message: &str) {
        self.print_error_with_detail(error_type, message, None, None);
    }

    /// Prints an error message with detail.
    pub fn print_error_with_detail(
        &self,
        error_type: &str,
        message: &str,
        detail: Option<&str>,
        suggestion: Option<&str>,
    ) {
        let mut stderr = std::io::stderr().lock();
        self.write_error(&mut stderr, error_type, message, detail, suggestion);
    }

    /// Prints a warning message with consistent formatting.
    ///
    /// Format: `⚠ Warning: {message}`
    ///         `  × {detail}` (optional)
    ///         `  help: {suggestion}` (optional)
    pub fn print_warning(&self, message: &str) {
        self.print_warning_with_detail(message, None, None);
    }

    /// Prints a warning message with detail.
    pub fn print_warning_with_detail(
        &self,
        message: &str,
        detail: Option<&str>,
        suggestion: Option<&str>,
    ) {
        let mut stderr = std::io::stderr().lock();
        self.write_warning(&mut stderr, message, detail, suggestion);
    }

    /// Writes error to a writer (for testing).
    pub fn write_error<W: Write>(
        &self,
        w: &mut W,
        error_type: &str,
        message: &str,
        detail: Option<&str>,
        suggestion: Option<&str>,
    ) {
        // Discard write errors: when reporting errors to stderr, failing to write
        // shouldn't cause additional failures. The user's terminal may be closed
        // or redirected, but we can't meaningfully recover from write failures here.
        if self.use_colors {
            let _ = writeln!(
                w,
                "{}{}✖ {error_type}:{} {message}",
                ansi::BOLD,
                ansi::RED,
                ansi::RESET
            );
        } else {
            let _ = writeln!(w, "✖ {error_type}: {message}");
        }

        if let Some(d) = detail {
            if self.use_colors {
                let _ = writeln!(w, "  {}× {d}{}", ansi::DIM, ansi::RESET);
            } else {
                let _ = writeln!(w, "  × {d}");
            }
        }

        if let Some(s) = suggestion {
            if self.use_colors {
                let _ = writeln!(w, "  {}help:{} {s}", ansi::CYAN, ansi::RESET);
            } else {
                let _ = writeln!(w, "  help: {s}");
            }
        }
    }

    /// Writes warning to a writer (for testing).
    pub fn write_warning<W: Write>(
        &self,
        w: &mut W,
        message: &str,
        detail: Option<&str>,
        suggestion: Option<&str>,
    ) {
        // Discard write errors: same rationale as write_error - we can't meaningfully
        // recover from stderr write failures during warning output.
        if self.use_colors {
            let _ = writeln!(
                w,
                "{}{}⚠ Warning:{} {message}",
                ansi::BOLD,
                ansi::YELLOW,
                ansi::RESET
            );
        } else {
            let _ = writeln!(w, "⚠ Warning: {message}");
        }

        if let Some(d) = detail {
            if self.use_colors {
                let _ = writeln!(w, "  {}× {d}{}", ansi::DIM, ansi::RESET);
            } else {
                let _ = writeln!(w, "  × {d}");
            }
        }

        if let Some(s) = suggestion {
            if self.use_colors {
                let _ = writeln!(w, "  {}help:{} {s}", ansi::CYAN, ansi::RESET);
            } else {
                let _ = writeln!(w, "  help: {s}");
            }
        }
    }

    /// Creates an error output formatter with explicit color control (for testing).
    #[cfg(test)]
    pub const fn with_colors(use_colors: bool) -> Self {
        Self { use_colors }
    }

    /// Checks if `NO_COLOR` environment variable is set.
    /// Per <https://no-color.org> spec: presence of the variable (any value) disables color.
    fn is_no_color_set() -> bool {
        std::env::var("NO_COLOR").is_ok()
    }
}

impl Default for ErrorOutput {
    fn default() -> Self {
        Self::stderr()
    }
}

/// Convenience function: prints an error using auto-detected color mode.
pub fn print_error(error_type: &str, message: &str) {
    ErrorOutput::stderr().print_error(error_type, message);
}

/// Convenience function: prints an error with detail and suggestion.
pub fn print_error_full(
    error_type: &str,
    message: &str,
    detail: Option<&str>,
    suggestion: Option<&str>,
) {
    ErrorOutput::stderr().print_error_with_detail(error_type, message, detail, suggestion);
}

/// Convenience function: prints a warning using auto-detected color mode.
pub fn print_warning(message: &str) {
    ErrorOutput::stderr().print_warning(message);
}

/// Convenience function: prints a warning with detail and suggestion.
pub fn print_warning_full(message: &str, detail: Option<&str>, suggestion: Option<&str>) {
    ErrorOutput::stderr().print_warning_with_detail(message, detail, suggestion);
}

#[cfg(test)]
#[path = "error_output_tests.rs"]
mod tests;
