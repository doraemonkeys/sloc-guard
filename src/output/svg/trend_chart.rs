//! Trend line chart for HTML reports showing SLOC history over time.

use super::chart::LineChart;
use super::data::DataPoint;
use super::element::SvgElement;
use super::style::ChartColor;
use crate::stats::TrendHistory;

/// Maximum number of data points to display (downsample if exceeded).
const MAX_POINTS: usize = 30;

/// Trend line chart showing code lines over time.
///
/// Visualizes historical SLOC data from `TrendHistory`. If the history contains
/// more than 30 entries, it is downsampled to ensure readability while preserving
/// the first and last data points.
#[derive(Debug)]
pub struct TrendLineChart {
    /// Inner line chart for rendering
    chart: LineChart,
}

impl TrendLineChart {
    /// Create chart from trend history.
    ///
    /// - Downsamples to max 30 points if history is longer
    /// - X labels: formatted date with `git_ref`/branch context
    /// - Y values: code lines
    #[must_use]
    pub fn from_history(history: &TrendHistory) -> Self {
        let entries = history.entries();

        let data = if entries.len() <= MAX_POINTS {
            // Use all entries
            Self::entries_to_data_points(entries)
        } else {
            // Downsample: always keep first and last, evenly sample middle
            let sampled = Self::downsample(entries, MAX_POINTS);
            Self::entries_to_data_points(&sampled)
        };

        let chart = LineChart::new("Code Lines Over Time", data)
            .with_color(ChartColor::css_var("chart-primary"))
            .with_size(500.0, 220.0)
            .with_points(true)
            .with_area(true);

        Self { chart }
    }

    /// Check if there's trend data to display.
    #[must_use]
    pub const fn has_data(&self) -> bool {
        !self.chart.data.is_empty()
    }

    /// Set chart dimensions.
    #[must_use]
    pub fn with_size(mut self, width: f64, height: f64) -> Self {
        self.chart = self.chart.with_size(width, height);
        self
    }

    /// Set line color.
    #[must_use]
    pub fn with_color(mut self, color: ChartColor) -> Self {
        self.chart = self.chart.with_color(color);
        self
    }

    /// Convert trend entries to chart data points.
    #[allow(clippy::cast_precision_loss)]
    fn entries_to_data_points(entries: &[crate::stats::TrendEntry]) -> Vec<DataPoint> {
        entries
            .iter()
            .map(|entry| {
                let label = Self::format_entry_label(entry);
                DataPoint::new(label, entry.code as f64)
            })
            .collect()
    }

    /// Format an entry's label for display on X-axis and tooltips.
    ///
    /// Format: "MM/DD" with optional git context in tooltip.
    /// Examples:
    /// - "12/24"
    /// - "12/24 (abc123)"
    /// - "12/24 (main)"
    fn format_entry_label(entry: &crate::stats::TrendEntry) -> String {
        let date = Self::format_timestamp(entry.timestamp);

        // Add git context if available (prefer ref over branch for brevity)
        match (&entry.git_ref, &entry.git_branch) {
            (Some(git_ref), _) => format!("{date} ({git_ref})"),
            (None, Some(branch)) => format!("{date} ({branch})"),
            (None, None) => date,
        }
    }

    /// Format a Unix timestamp as "MM/DD".
    ///
    /// Uses simple arithmetic to extract month and day from Unix timestamp.
    fn format_timestamp(timestamp: u64) -> String {
        // Calculate days since epoch, then decompose into year/month/day
        // This is a simplified calculation suitable for display purposes
        let (_, month, day) = Self::timestamp_to_date(timestamp);
        format!("{month:02}/{day:02}")
    }

    /// Convert Unix timestamp to (year, month, day).
    ///
    /// Simple implementation without external date libraries.
    /// Handles leap years for reasonable accuracy.
    #[allow(clippy::cast_possible_truncation)]
    fn timestamp_to_date(timestamp: u64) -> (u32, u32, u32) {
        const SECS_PER_DAY: u64 = 86400;

        // Days since Unix epoch (1970-01-01)
        let mut days = (timestamp / SECS_PER_DAY) as u32;

        // Calculate year
        let mut year = 1970u32;
        loop {
            let days_in_year = if Self::is_leap_year(year) { 366 } else { 365 };
            if days < days_in_year {
                break;
            }
            days -= days_in_year;
            year += 1;
        }

        // Days in each month (non-leap year)
        let days_in_month = if Self::is_leap_year(year) {
            [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
        } else {
            [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
        };

        // Calculate month and day
        let mut month = 1u32;
        for &dim in &days_in_month {
            if days < dim {
                break;
            }
            days -= dim;
            month += 1;
        }

        let day = days + 1; // Days are 1-indexed

        (year, month, day)
    }

    /// Check if a year is a leap year.
    const fn is_leap_year(year: u32) -> bool {
        year.is_multiple_of(4) && (!year.is_multiple_of(100) || year.is_multiple_of(400))
    }

    /// Downsample entries to at most `max_points` while keeping first and last.
    fn downsample(
        entries: &[crate::stats::TrendEntry],
        max_points: usize,
    ) -> Vec<crate::stats::TrendEntry> {
        if entries.len() <= max_points || max_points < 2 {
            return entries.to_vec();
        }

        let mut result = Vec::with_capacity(max_points);

        // Always include first entry
        result.push(entries[0].clone());

        // Calculate step size for middle entries
        // We need (max_points - 2) middle entries from (entries.len() - 2) candidates
        let middle_count = max_points - 2;
        let source_middle = entries.len() - 2;

        for i in 1..=middle_count {
            // Map i to source index using linear interpolation
            #[allow(
                clippy::cast_precision_loss,
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss
            )]
            let source_idx =
                ((i as f64 / (middle_count + 1) as f64) * source_middle as f64) as usize + 1;
            result.push(entries[source_idx].clone());
        }

        // Always include last entry
        result.push(entries[entries.len() - 1].clone());

        result
    }
}

impl SvgElement for TrendLineChart {
    fn render(&self) -> String {
        self.chart.render()
    }
}

#[cfg(test)]
#[path = "trend_chart_tests.rs"]
mod tests;
