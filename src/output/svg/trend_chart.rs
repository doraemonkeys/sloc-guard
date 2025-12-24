//! Trend line chart for HTML reports showing SLOC history over time.

use std::fmt::Write;

use super::data::DataPoint;
use super::element::SvgElement;
use super::format::{format_number, html_escape};
use super::style::ChartColor;
use crate::stats::TrendHistory;

/// Maximum number of data points to display (downsample if exceeded).
const MAX_POINTS: usize = 30;

/// Time range thresholds for smart label formatting (in seconds).
const ONE_DAY: u64 = 86_400;
const ONE_MONTH: u64 = 30 * ONE_DAY;

/// Vertical offset for delta indicator arrows above the midpoint between data points.
const DELTA_INDICATOR_Y_OFFSET: f64 = 12.0;

/// Vertical offset for X-axis labels below the chart baseline.
const X_LABEL_Y_OFFSET: f64 = 14.0;

/// Trend line chart showing code lines over time.
///
/// Visualizes historical SLOC data from `TrendHistory`. If the history contains
/// more than 30 entries, it is downsampled to ensure readability while preserving
/// the first and last data points.
///
/// Features:
/// - Delta indicators: ↓green (decrease=good), ↑red (increase)
/// - Smart X-axis labels: days/weeks based on time range
/// - Hover tooltips with value and delta info
#[derive(Debug)]
pub struct TrendLineChart {
    /// Data points for rendering
    data: Vec<DataPoint>,
    /// Chart dimensions
    width: f64,
    height: f64,
    padding: f64,
    /// Line color
    line_color: ChartColor,
    /// Show delta indicators between points
    show_deltas: bool,
}

impl TrendLineChart {
    /// Create chart from trend history.
    ///
    /// - Downsamples to max 30 points if history is longer
    /// - X labels: formatted date with `git_ref`/branch context
    /// - Y values: code lines
    /// - Delta indicators: colored arrows showing change direction
    #[must_use]
    pub fn from_history(history: &TrendHistory) -> Self {
        let entries = history.entries();

        // Calculate time range for smart label formatting
        let time_range_secs = if entries.len() >= 2 {
            entries.last().map_or(0, |e| e.timestamp) - entries.first().map_or(0, |e| e.timestamp)
        } else {
            0
        };

        let processed_entries = if entries.len() <= MAX_POINTS {
            entries.to_vec()
        } else {
            Self::downsample(entries, MAX_POINTS)
        };

        let data = Self::entries_to_data_points(&processed_entries, time_range_secs);

        Self {
            data,
            width: 500.0,
            height: 220.0,
            padding: 50.0,
            line_color: ChartColor::css_var("chart-primary"),
            show_deltas: true,
        }
    }

    /// Check if there's trend data to display.
    #[must_use]
    pub const fn has_data(&self) -> bool {
        !self.data.is_empty()
    }

    /// Set chart dimensions.
    #[must_use]
    pub const fn with_size(mut self, width: f64, height: f64) -> Self {
        self.width = width;
        self.height = height;
        self
    }

    /// Set line color.
    #[must_use]
    pub fn with_color(mut self, color: ChartColor) -> Self {
        self.line_color = color;
        self
    }

    /// Enable or disable delta indicators.
    #[must_use]
    pub const fn with_deltas(mut self, show: bool) -> Self {
        self.show_deltas = show;
        self
    }

    /// Convert trend entries to chart data points with smart labels.
    #[allow(clippy::cast_precision_loss)]
    fn entries_to_data_points(
        entries: &[crate::stats::TrendEntry],
        time_range_secs: u64,
    ) -> Vec<DataPoint> {
        entries
            .iter()
            .map(|entry| {
                let label = Self::format_entry_label(entry, time_range_secs);
                DataPoint::new(label, entry.code as f64)
            })
            .collect()
    }

    /// Format an entry's label for display on X-axis and tooltips.
    ///
    /// Uses smart formatting based on time range:
    /// - < 1 week: "MM/DD" (e.g., "12/24")
    /// - 1 week - 1 month: "MM/DD" (e.g., "12/24")
    /// - > 1 month: "MM/DD" with week grouping consideration
    ///
    /// Optional git context is appended for tooltips.
    fn format_entry_label(entry: &crate::stats::TrendEntry, time_range_secs: u64) -> String {
        let date = Self::format_timestamp_smart(entry.timestamp, time_range_secs);

        // Add git context if available (prefer ref over branch for brevity)
        match (&entry.git_ref, &entry.git_branch) {
            (Some(git_ref), _) => format!("{date} ({git_ref})"),
            (None, Some(branch)) => format!("{date} ({branch})"),
            (None, None) => date,
        }
    }

    /// Format a Unix timestamp with smart label based on time range.
    ///
    /// - Short range (<= 1 month): "MM/DD" format
    /// - Long range (> 1 month): "MM/DD" format but labels may show week numbers
    fn format_timestamp_smart(timestamp: u64, time_range_secs: u64) -> String {
        let (year, month, day) = Self::timestamp_to_date(timestamp);

        if time_range_secs > ONE_MONTH {
            // For ranges > 1 month, show week indicator for context
            let week = Self::week_of_year(year, month, day);
            format!("W{week:02}")
        } else {
            format!("{month:02}/{day:02}")
        }
    }

    /// Calculate week of year (ISO week number approximation).
    const fn week_of_year(year: u32, month: u32, day: u32) -> u32 {
        // Calculate day of year
        let days_before_month = if Self::is_leap_year(year) {
            [0, 31, 60, 91, 121, 152, 182, 213, 244, 274, 305, 335]
        } else {
            [0, 31, 59, 90, 120, 151, 181, 212, 243, 273, 304, 334]
        };
        let day_of_year = days_before_month[(month - 1) as usize] + day;

        // Week number (1-indexed, starting Monday)
        day_of_year.div_ceil(7)
    }

    /// Convert Unix timestamp to (year, month, day).
    ///
    /// Simple implementation without external date libraries.
    /// Handles leap years for reasonable accuracy.
    #[allow(clippy::cast_possible_truncation)]
    fn timestamp_to_date(timestamp: u64) -> (u32, u32, u32) {
        // Days since Unix epoch (1970-01-01)
        let mut days = (timestamp / ONE_DAY) as u32;

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

        // Days in each month
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
        let middle_count = max_points - 2;
        let source_middle = entries.len() - 2;

        for i in 1..=middle_count {
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

    /// Get delta arrow and color class for a value change.
    ///
    /// For SLOC: decrease is good (green ↓), increase is bad (red ↑).
    #[allow(clippy::cast_possible_truncation)]
    fn delta_indicator(prev_value: f64, curr_value: f64) -> (&'static str, &'static str) {
        use std::cmp::Ordering;
        let delta = (curr_value - prev_value) as i64;
        match delta.cmp(&0) {
            Ordering::Less => ("↓", "delta-good"), // Decrease: good for SLOC
            Ordering::Greater => ("↑", "delta-bad"), // Increase: potentially concerning
            Ordering::Equal => ("", "delta-neutral"),
        }
    }

    /// Draw grid lines and Y-axis labels.
    #[allow(clippy::cast_precision_loss)]
    fn draw_grid(
        output: &mut String,
        padding: f64,
        width: f64,
        chart_height: f64,
        max_value: f64,
        value_range: f64,
    ) {
        let grid_color = ChartColor::css_var("border").to_css();
        let label_color = ChartColor::css_var("text-muted").to_css();

        for i in 0..=4 {
            let y = (chart_height / 4.0).mul_add(f64::from(i), padding);
            let _ = writeln!(
                output,
                r#"    <line x1="{}" y1="{y}" x2="{}" y2="{y}" stroke="{grid_color}" stroke-width="1" stroke-dasharray="4,4" opacity="0.5"/>"#,
                padding,
                width - padding
            );

            let label_value = max_value - (value_range * f64::from(i) / 4.0);
            #[allow(clippy::cast_possible_truncation)]
            let formatted = format_number(label_value as i64);
            let _ = writeln!(
                output,
                r#"    <text x="{}" y="{y}" text-anchor="end" fill="{label_color}" font-size="10" dominant-baseline="middle">{formatted}</text>"#,
                padding - 8.0
            );
        }
    }

    /// Draw line path and optional area fill.
    fn draw_line_and_area(
        output: &mut String,
        points: &[(f64, f64)],
        baseline_y: f64,
        color: &str,
    ) {
        // Draw area under line
        if points.len() >= 2 {
            let mut area_path = String::new();
            let _ = write!(area_path, "M{},{baseline_y}", points[0].0);
            for (x, y) in points {
                let _ = write!(area_path, " L{x},{y}");
            }
            let _ = write!(area_path, " L{},{baseline_y} Z", points[points.len() - 1].0);

            let _ = writeln!(
                output,
                r#"    <path d="{area_path}" fill="{color}" fill-opacity="0.1" stroke="none"/>"#
            );
        }

        // Draw line
        if !points.is_empty() {
            let mut line_path = String::new();
            for (i, (x, y)) in points.iter().enumerate() {
                if i == 0 {
                    let _ = write!(line_path, "M{x},{y}");
                } else {
                    let _ = write!(line_path, " L{x},{y}");
                }
            }
            let _ = writeln!(
                output,
                r#"    <path d="{line_path}" fill="none" stroke="{color}" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>"#
            );
        }
    }

    /// Draw a delta indicator arrow between two points.
    #[allow(clippy::cast_precision_loss)]
    fn draw_delta_indicator(
        output: &mut String,
        prev_point: (f64, f64),
        curr_point: (f64, f64),
        prev_value: f64,
        curr_value: f64,
    ) {
        let (arrow, color_class) = Self::delta_indicator(prev_value, curr_value);
        if arrow.is_empty() {
            return;
        }

        let mid_x = f64::midpoint(prev_point.0, curr_point.0);
        let mid_y = f64::midpoint(prev_point.1, curr_point.1) - DELTA_INDICATOR_Y_OFFSET;

        let color = match color_class {
            "delta-good" => ChartColor::css_var("delta-good").to_css(),
            "delta-bad" => ChartColor::css_var("delta-bad").to_css(),
            _ => ChartColor::css_var("delta-neutral").to_css(),
        };

        #[allow(clippy::cast_possible_truncation)]
        let delta = (curr_value - prev_value) as i64;
        let delta_abs = delta.unsigned_abs();

        // Only show arrow for significant changes (> 1% or > 10 lines)
        let threshold = (prev_value * 0.01).max(10.0);
        if delta_abs as f64 > threshold {
            let _ = writeln!(
                output,
                r#"    <text x="{mid_x}" y="{mid_y}" text-anchor="middle" fill="{color}" font-size="12" class="delta-indicator" data-delta="{delta:+}">{arrow}</text>"#
            );
        }
    }

    /// Render the chart as SVG.
    #[allow(clippy::cast_precision_loss)]
    fn render_svg(&self) -> String {
        let mut output = String::new();

        let _ = writeln!(
            output,
            r#"<svg viewBox="0 0 {} {}" xmlns="http://www.w3.org/2000/svg" role="img">"#,
            self.width, self.height
        );
        let _ = writeln!(output, r"    <title>Code Lines Over Time</title>");

        if self.data.is_empty() {
            let text_color = ChartColor::css_var("text-muted").to_css();
            let _ = writeln!(
                output,
                r#"    <text x="{}" y="{}" text-anchor="middle" fill="{text_color}" font-size="14">No trend data</text>"#,
                self.width / 2.0,
                self.height / 2.0
            );
            output.push_str("</svg>");
            return output;
        }

        let chart_width = self.padding.mul_add(-2.0, self.width);
        let chart_height = self.padding.mul_add(-2.0, self.height);
        let baseline_y = self.padding + chart_height;

        // Find min/max for Y scaling
        let max_value = self
            .data
            .iter()
            .map(|d| d.value)
            .fold(f64::NEG_INFINITY, f64::max);
        let min_value = self
            .data
            .iter()
            .map(|d| d.value)
            .fold(f64::INFINITY, f64::min);
        let value_range = (max_value - min_value).max(1.0);
        let padded_min = value_range.mul_add(-0.1, min_value);
        let padded_range = value_range * 1.2;

        // Calculate points
        let point_count = self.data.len();
        let x_step = if point_count > 1 {
            chart_width / (point_count - 1) as f64
        } else {
            0.0
        };

        let points: Vec<(f64, f64)> = self
            .data
            .iter()
            .enumerate()
            .map(|(i, d)| {
                let x = x_step.mul_add(i as f64, self.padding);
                let normalized = (d.value - padded_min) / padded_range;
                let y = baseline_y - normalized * chart_height;
                (x, y)
            })
            .collect();

        // Draw grid, line, and area
        Self::draw_grid(
            &mut output,
            self.padding,
            self.width,
            chart_height,
            max_value,
            value_range,
        );
        Self::draw_line_and_area(&mut output, &points, baseline_y, &self.line_color.to_css());

        // Draw data points with tooltips and delta indicators
        self.draw_data_points(&mut output, &points, baseline_y);

        output.push_str("</svg>");
        output
    }

    /// Draw data points with tooltips and optional delta indicators.
    #[allow(clippy::cast_precision_loss)]
    fn draw_data_points(&self, output: &mut String, points: &[(f64, f64)], baseline_y: f64) {
        let point_color = self.line_color.to_css();
        let point_count = points.len();

        for (i, ((x, y), data)) in points.iter().zip(self.data.iter()).enumerate() {
            let escaped_label = html_escape(&data.label);
            #[allow(clippy::cast_possible_truncation)]
            let value_display = data.value as i64;

            // Build tooltip with delta info
            let tooltip = if i > 0 && self.show_deltas {
                let prev_value = self.data[i - 1].value;
                #[allow(clippy::cast_possible_truncation)]
                let delta = (data.value - prev_value) as i64;
                let delta_str = if delta >= 0 {
                    format!("+{delta}")
                } else {
                    format!("{delta}")
                };
                format!("{escaped_label}: {value_display} ({delta_str})")
            } else {
                format!("{escaped_label}: {value_display}")
            };

            let _ = writeln!(
                output,
                r#"    <circle cx="{x}" cy="{y}" r="4" fill="{point_color}" stroke="var(--color-card, white)" stroke-width="2">
        <title>{tooltip}</title>
    </circle>"#
            );

            // Draw delta indicator between points
            if i > 0 && self.show_deltas {
                Self::draw_delta_indicator(
                    output,
                    points[i - 1],
                    (*x, *y),
                    self.data[i - 1].value,
                    data.value,
                );
            }

            // X-axis labels (show first, last, and some intermediate)
            if i == 0
                || i == point_count - 1
                || (point_count > 5 && i % (point_count / 5).max(1) == 0)
            {
                let label_color = ChartColor::css_var("text-muted").to_css();
                let display_label = if data.label.len() > 10 {
                    format!("{}…", &data.label[..9])
                } else {
                    data.label.clone()
                };
                let escaped = html_escape(&display_label);
                let _ = writeln!(
                    output,
                    r#"    <text x="{x}" y="{}" text-anchor="middle" fill="{label_color}" font-size="9">{escaped}</text>"#,
                    baseline_y + X_LABEL_Y_OFFSET
                );
            }
        }
    }
}

impl SvgElement for TrendLineChart {
    fn render(&self) -> String {
        self.render_svg()
    }
}

#[cfg(test)]
#[path = "trend_chart_tests.rs"]
mod tests;
