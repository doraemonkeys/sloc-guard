//! Trend delta formatting utilities for stats output.
//!
//! Provides relative time formatting, percentage calculation, and colored trend arrows.

use super::ColorMode;
use super::ansi;
use crate::stats::TrendDelta;

// Time thresholds in seconds for relative time formatting
const MINUTE: u64 = 60;
const HOUR: u64 = 60 * MINUTE;
const DAY: u64 = 24 * HOUR;
const WEEK: u64 = 7 * DAY;
const MONTH: u64 = 30 * DAY;

/// Convert a Unix timestamp to relative time string ("2 hours ago").
///
/// Returns `None` if the timestamp is in the future or current time cannot be determined.
pub fn format_relative_time(timestamp: u64) -> Option<String> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .ok()?
        .as_secs();

    if timestamp > now {
        return None;
    }

    let elapsed = now - timestamp;

    let result = if elapsed < MINUTE {
        "just now".to_string()
    } else if elapsed < HOUR {
        let mins = elapsed / MINUTE;
        if mins == 1 {
            "1 minute ago".to_string()
        } else {
            format!("{mins} minutes ago")
        }
    } else if elapsed < DAY {
        let hours = elapsed / HOUR;
        if hours == 1 {
            "1 hour ago".to_string()
        } else {
            format!("{hours} hours ago")
        }
    } else if elapsed < WEEK {
        let days = elapsed / DAY;
        if days == 1 {
            "1 day ago".to_string()
        } else {
            format!("{days} days ago")
        }
    } else if elapsed < MONTH {
        let weeks = elapsed / WEEK;
        if weeks == 1 {
            "1 week ago".to_string()
        } else {
            format!("{weeks} weeks ago")
        }
    } else {
        let months = elapsed / MONTH;
        if months == 1 {
            "1 month ago".to_string()
        } else {
            format!("{months} months ago")
        }
    };

    Some(result)
}

/// Calculate percentage change from previous to current value.
///
/// Returns `None` if previous value was zero (cannot calculate percentage from zero).
#[allow(clippy::cast_precision_loss, clippy::cast_possible_wrap)]
pub fn calculate_percentage(delta: i64, current: usize) -> Option<f64> {
    let previous = current as i64 - delta;
    if previous == 0 {
        return None;
    }
    Some(delta as f64 / previous as f64 * 100.0)
}

/// Get trend arrow based on delta direction.
pub const fn trend_arrow(delta: i64) -> &'static str {
    if delta > 0 {
        "↑"
    } else if delta < 0 {
        "↓"
    } else {
        "~"
    }
}

/// Get ANSI color code for trend direction.
pub const fn trend_color(delta: i64) -> &'static str {
    if delta > 0 {
        ansi::GREEN
    } else if delta < 0 {
        ansi::RED
    } else {
        ansi::DIM
    }
}

/// Format a delta value with +/- sign.
pub fn format_delta(value: i64) -> String {
    use std::cmp::Ordering;
    match value.cmp(&0) {
        Ordering::Greater => format!("+{value}"),
        Ordering::Less => format!("{value}"),
        Ordering::Equal => "0".to_string(),
    }
}

/// Formatter for trend delta output with optional colors.
pub struct TrendLineFormatter {
    use_colors: bool,
}

impl TrendLineFormatter {
    #[must_use]
    pub fn new(mode: ColorMode) -> Self {
        Self {
            use_colors: Self::should_use_colors(mode),
        }
    }

    fn should_use_colors(mode: ColorMode) -> bool {
        mode.should_enable_for_stdout()
    }

    /// Format a trend line with arrow, delta, and percentage.
    ///
    /// Example: "  Code: ↑ +50 (+12.5%)"
    #[must_use]
    pub fn format_line(&self, name: &str, delta: i64, current: usize) -> String {
        let arrow = trend_arrow(delta);
        let delta_str = format_delta(delta);

        // Calculate percentage if possible
        let pct_str = calculate_percentage(delta, current)
            .map(|pct| format!(" ({pct:+.1}%)"))
            .unwrap_or_default();

        if self.use_colors {
            let color = trend_color(delta);
            format!(
                "  {name}: {color}{arrow} {delta_str}{pct_str}{}",
                ansi::RESET
            )
        } else {
            format!("  {name}: {arrow} {delta_str}{pct_str}")
        }
    }
}

/// Format the trend section header with git context and/or relative time.
///
/// Examples:
/// - With git: "Changes since commit a1b2c3d (2 hours ago):"
/// - With git + branch: "Changes since commit a1b2c3d on main (2 hours ago):"
/// - Without git: "Changes since previous run (2 hours ago):"
/// - No timestamp: "Changes from previous run:"
#[must_use]
pub fn format_trend_header(trend: &TrendDelta) -> String {
    let relative_time = trend
        .previous_timestamp
        .and_then(format_relative_time)
        .map(|rel| format!(" ({rel})"))
        .unwrap_or_default();

    match (&trend.previous_git_ref, &trend.previous_git_branch) {
        (Some(commit), Some(branch)) => {
            format!("Changes since commit {commit} on {branch}{relative_time}:")
        }
        (Some(commit), None) => {
            format!("Changes since commit {commit}{relative_time}:")
        }
        (None, _) if !relative_time.is_empty() => {
            format!("Changes since previous run{relative_time}:")
        }
        (None, _) => "Changes from previous run:".to_string(),
    }
}

/// Format the trend section header for markdown (no trailing colon).
///
/// Same as `format_trend_header` but without trailing colon for use in headings.
#[must_use]
pub fn format_trend_header_markdown(trend: &TrendDelta) -> String {
    let header = format_trend_header(trend);
    // Remove trailing colon if present
    header.strip_suffix(':').unwrap_or(&header).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_delta_positive() {
        assert_eq!(format_delta(5), "+5");
    }

    #[test]
    fn format_delta_negative() {
        assert_eq!(format_delta(-5), "-5");
    }

    #[test]
    fn format_delta_zero() {
        assert_eq!(format_delta(0), "0");
    }

    #[test]
    fn trend_arrow_positive() {
        assert_eq!(trend_arrow(10), "↑");
    }

    #[test]
    fn trend_arrow_negative() {
        assert_eq!(trend_arrow(-10), "↓");
    }

    #[test]
    fn trend_arrow_zero() {
        assert_eq!(trend_arrow(0), "~");
    }

    #[test]
    fn percentage_from_positive_delta() {
        // current=110, delta=10 -> previous=100 -> +10%
        let pct = calculate_percentage(10, 110).unwrap();
        assert!((pct - 10.0).abs() < 0.01);
    }

    #[test]
    fn percentage_from_negative_delta() {
        // current=90, delta=-10 -> previous=100 -> -10%
        let pct = calculate_percentage(-10, 90).unwrap();
        assert!((pct - (-10.0)).abs() < 0.01);
    }

    #[test]
    fn percentage_from_zero_base_returns_none() {
        // current=10, delta=10 -> previous=0 -> None
        assert!(calculate_percentage(10, 10).is_none());
    }

    #[test]
    fn relative_time_just_now() {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        assert_eq!(format_relative_time(now), Some("just now".to_string()));
    }

    #[test]
    fn relative_time_one_minute() {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        assert_eq!(
            format_relative_time(now - MINUTE),
            Some("1 minute ago".to_string())
        );
    }

    #[test]
    fn relative_time_minutes() {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        assert_eq!(
            format_relative_time(now - 5 * MINUTE),
            Some("5 minutes ago".to_string())
        );
    }

    #[test]
    fn relative_time_one_hour() {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        assert_eq!(
            format_relative_time(now - HOUR),
            Some("1 hour ago".to_string())
        );
    }

    #[test]
    fn relative_time_hours() {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        assert_eq!(
            format_relative_time(now - 3 * HOUR),
            Some("3 hours ago".to_string())
        );
    }

    #[test]
    fn relative_time_one_day() {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        assert_eq!(
            format_relative_time(now - DAY),
            Some("1 day ago".to_string())
        );
    }

    #[test]
    fn relative_time_days() {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        assert_eq!(
            format_relative_time(now - 2 * DAY),
            Some("2 days ago".to_string())
        );
    }

    #[test]
    fn relative_time_one_week() {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        assert_eq!(
            format_relative_time(now - WEEK),
            Some("1 week ago".to_string())
        );
    }

    #[test]
    fn relative_time_weeks() {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        assert_eq!(
            format_relative_time(now - 2 * WEEK),
            Some("2 weeks ago".to_string())
        );
    }

    #[test]
    fn relative_time_one_month() {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        assert_eq!(
            format_relative_time(now - MONTH),
            Some("1 month ago".to_string())
        );
    }

    #[test]
    fn relative_time_months() {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        assert_eq!(
            format_relative_time(now - 3 * MONTH),
            Some("3 months ago".to_string())
        );
    }

    #[test]
    fn relative_time_future_returns_none() {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        assert!(format_relative_time(now + 1000).is_none());
    }

    #[test]
    fn trend_line_formatter_no_colors() {
        let formatter = TrendLineFormatter::new(ColorMode::Never);
        let line = formatter.format_line("Code", 50, 550);
        assert!(line.contains("Code:"));
        assert!(line.contains("↑"));
        assert!(line.contains("+50"));
        assert!(line.contains('%'));
    }

    #[test]
    fn trend_line_formatter_with_colors() {
        let formatter = TrendLineFormatter::new(ColorMode::Always);
        let line = formatter.format_line("Code", -10, 90);
        assert!(line.contains('\x1b')); // ANSI escape
        assert!(line.contains("↓"));
    }

    #[test]
    fn trend_header_with_git_and_branch() {
        let trend = TrendDelta {
            files_delta: 0,
            lines_delta: 0,
            code_delta: 0,
            comment_delta: 0,
            blank_delta: 0,
            previous_timestamp: None,
            previous_git_ref: Some("a1b2c3d".to_string()),
            previous_git_branch: Some("main".to_string()),
        };
        let header = format_trend_header(&trend);
        assert_eq!(header, "Changes since commit a1b2c3d on main:");
    }

    #[test]
    fn trend_header_with_git_no_branch() {
        let trend = TrendDelta {
            files_delta: 0,
            lines_delta: 0,
            code_delta: 0,
            comment_delta: 0,
            blank_delta: 0,
            previous_timestamp: None,
            previous_git_ref: Some("a1b2c3d".to_string()),
            previous_git_branch: None,
        };
        let header = format_trend_header(&trend);
        assert_eq!(header, "Changes since commit a1b2c3d:");
    }

    #[test]
    fn trend_header_no_git_no_timestamp() {
        let trend = TrendDelta::default();
        let header = format_trend_header(&trend);
        assert_eq!(header, "Changes from previous run:");
    }

    #[test]
    fn trend_header_no_git_with_timestamp() {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let trend = TrendDelta {
            previous_timestamp: Some(now - 2 * HOUR),
            ..Default::default()
        };
        let header = format_trend_header(&trend);
        assert_eq!(header, "Changes since previous run (2 hours ago):");
    }

    #[test]
    fn trend_header_with_git_and_timestamp() {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let trend = TrendDelta {
            previous_timestamp: Some(now - 2 * HOUR),
            previous_git_ref: Some("a1b2c3d".to_string()),
            previous_git_branch: None,
            ..Default::default()
        };
        let header = format_trend_header(&trend);
        assert_eq!(header, "Changes since commit a1b2c3d (2 hours ago):");
    }

    #[test]
    fn trend_header_markdown_removes_colon() {
        let trend = TrendDelta {
            previous_git_ref: Some("a1b2c3d".to_string()),
            previous_git_branch: Some("main".to_string()),
            ..Default::default()
        };
        let header = format_trend_header_markdown(&trend);
        assert_eq!(header, "Changes since commit a1b2c3d on main");
        assert!(!header.ends_with(':'));
    }
}
