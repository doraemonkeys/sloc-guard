//! File size distribution histogram for HTML reports.

use std::fmt::Write;

use super::data::DataPoint;
use super::element::{Bar, SvgElement};
use super::format::{format_number, html_escape};
use super::style::ChartColor;
use crate::output::ProjectStatistics;

/// Line count range buckets for the histogram.
#[derive(Debug, Clone, Copy)]
pub struct SizeBucket {
    /// Minimum line count (inclusive)
    pub min: usize,
    /// Maximum line count (exclusive, None = unbounded)
    pub max: Option<usize>,
    /// Display label
    pub label: &'static str,
}

impl SizeBucket {
    /// Check if a line count falls within this bucket.
    #[must_use]
    pub const fn contains(&self, lines: usize) -> bool {
        if lines < self.min {
            return false;
        }
        match self.max {
            Some(max) => lines < max,
            None => true,
        }
    }
}

/// Default buckets: 0-50, 51-100, 101-200, 201-500, 501+
pub const DEFAULT_BUCKETS: &[SizeBucket] = &[
    SizeBucket {
        min: 0,
        max: Some(51),
        label: "0-50",
    },
    SizeBucket {
        min: 51,
        max: Some(101),
        label: "51-100",
    },
    SizeBucket {
        min: 101,
        max: Some(201),
        label: "101-200",
    },
    SizeBucket {
        min: 201,
        max: Some(501),
        label: "201-500",
    },
    SizeBucket {
        min: 501,
        max: None,
        label: "501+",
    },
];

/// File size distribution histogram.
#[derive(Debug)]
pub struct FileSizeHistogram {
    pub title: String,
    pub data: Vec<DataPoint>,
    pub width: f64,
    pub height: f64,
    pub padding: f64,
    pub bar_color: ChartColor,
    /// Minimum file count to show histogram (empty state threshold)
    pub min_files: usize,
    /// Total files in the histogram (used for sufficient data check)
    total_files: usize,
}

impl Default for FileSizeHistogram {
    fn default() -> Self {
        Self {
            title: "File Size Distribution".to_string(),
            data: Vec::new(),
            width: 400.0,
            height: 200.0,
            padding: 40.0,
            bar_color: ChartColor::css_var("chart-primary"),
            min_files: 3,
            total_files: 0,
        }
    }
}

impl FileSizeHistogram {
    /// Create histogram from project statistics.
    #[must_use]
    pub fn from_stats(stats: &ProjectStatistics) -> Self {
        let data = Self::compute_buckets(stats);
        let total_files = stats.files.len();
        Self {
            data,
            total_files,
            ..Default::default()
        }
    }

    /// Create histogram with custom title.
    #[must_use]
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }

    /// Set chart dimensions.
    #[must_use]
    pub const fn with_size(mut self, width: f64, height: f64) -> Self {
        self.width = width;
        self.height = height;
        self
    }

    /// Set bar color.
    #[must_use]
    pub fn with_color(mut self, color: ChartColor) -> Self {
        self.bar_color = color;
        self
    }

    /// Check if there are enough files to display the histogram.
    #[must_use]
    pub const fn has_sufficient_data(&self) -> bool {
        self.total_files >= self.min_files
    }

    /// Compute bucket counts from project statistics.
    #[allow(clippy::cast_precision_loss)]
    fn compute_buckets(stats: &ProjectStatistics) -> Vec<DataPoint> {
        let mut counts = vec![0usize; DEFAULT_BUCKETS.len()];

        for file in &stats.files {
            // Use code lines (SLOC) for bucketing
            let lines = file.stats.code;
            for (i, bucket) in DEFAULT_BUCKETS.iter().enumerate() {
                if bucket.contains(lines) {
                    counts[i] += 1;
                    break;
                }
            }
        }

        DEFAULT_BUCKETS
            .iter()
            .zip(counts)
            .map(|(bucket, count)| DataPoint::new(bucket.label, count as f64))
            .collect()
    }
}

impl SvgElement for FileSizeHistogram {
    #[allow(
        clippy::cast_precision_loss,
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss
    )]
    fn render(&self) -> String {
        let mut output = String::new();

        let _ = writeln!(
            output,
            r#"<svg viewBox="0 0 {} {}" xmlns="http://www.w3.org/2000/svg" role="img">"#,
            self.width, self.height
        );

        let escaped_title = html_escape(&self.title);
        let _ = writeln!(output, r"    <title>{escaped_title}</title>");

        // Empty state: show message when insufficient data
        if self.total_files < self.min_files {
            let text_color = ChartColor::css_var("text-muted").to_css();
            let message = if self.total_files == 0 {
                "No files to display"
            } else {
                "Not enough files for histogram"
            };
            let _ = writeln!(
                output,
                r#"    <text x="{}" y="{}" text-anchor="middle" fill="{text_color}" font-size="14">{message}</text>"#,
                self.width / 2.0,
                self.height / 2.0
            );
            output.push_str("</svg>");
            return output;
        }

        let chart_width = self.padding.mul_add(-2.0, self.width);
        let chart_height = self.padding.mul_add(-2.0, self.height);

        // Find max value for scaling
        let max_value = self
            .data
            .iter()
            .map(|d| d.value)
            .fold(0.0_f64, f64::max)
            .max(1.0);

        // Bar dimensions
        let bar_count = self.data.len();
        let gap_ratio = 0.2;
        let total_gap = chart_width * gap_ratio;
        let bar_width = (chart_width - total_gap) / bar_count as f64;
        let gap = total_gap / (bar_count + 1) as f64;
        let base_offset = self.padding + gap;

        // Draw bars
        for (i, point) in self.data.iter().enumerate() {
            let x = (bar_width + gap).mul_add(i as f64, base_offset);
            let bar_height = (point.value / max_value) * chart_height;
            let y = self.padding + chart_height - bar_height;

            let color = point
                .color
                .clone()
                .unwrap_or_else(|| self.bar_color.clone());

            let bar_element = Bar {
                x,
                y,
                width: bar_width,
                height: bar_height,
                color,
                label: point.label.clone(),
                value: point.value,
            };

            let _ = writeln!(output, "    {}", bar_element.render());

            // Value label on top of bar (file count)
            if point.value > 0.0 {
                let text_color = ChartColor::css_var("text").to_css();
                #[allow(clippy::cast_possible_truncation)]
                let count = point.value as i64;
                let formatted = format_number(count);
                let _ = writeln!(
                    output,
                    r#"    <text x="{}" y="{}" text-anchor="middle" fill="{text_color}" font-size="10">{formatted}</text>"#,
                    x + bar_width / 2.0,
                    y - 4.0
                );
            }

            // X-axis label (line range)
            let label_color = ChartColor::css_var("text-muted").to_css();
            let escaped_label = html_escape(&point.label);
            let _ = writeln!(
                output,
                r#"    <text x="{}" y="{}" text-anchor="middle" fill="{label_color}" font-size="10">{escaped_label}</text>"#,
                x + bar_width / 2.0,
                self.height - 8.0
            );
        }

        // Y-axis title
        let title_color = ChartColor::css_var("text").to_css();
        let _ = writeln!(
            output,
            r#"    <text x="{}" y="{}" text-anchor="middle" fill="{title_color}" font-size="11" font-weight="500">Lines of Code</text>"#,
            self.width / 2.0,
            self.height - 2.0
        );

        output.push_str("</svg>");
        output
    }
}

#[cfg(test)]
#[path = "histogram_tests.rs"]
mod tests;
