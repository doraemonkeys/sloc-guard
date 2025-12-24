//! Composite chart types: bar charts and line charts.

use std::fmt::Write;

use super::data::DataPoint;
use super::element::{Bar, SvgElement};
use super::format::{format_number, html_escape};
use super::style::ChartColor;

/// Vertical bar chart with automatic scaling.
#[derive(Debug)]
pub struct BarChart {
    pub title: String,
    pub data: Vec<DataPoint>,
    pub width: f64,
    pub height: f64,
    pub padding: f64,
    pub bar_color: ChartColor,
    pub show_values: bool,
}

impl Default for BarChart {
    fn default() -> Self {
        Self {
            title: String::new(),
            data: Vec::new(),
            width: 400.0,
            height: 200.0,
            padding: 40.0,
            bar_color: ChartColor::css_var("chart-primary"),
            show_values: true,
        }
    }
}

impl BarChart {
    #[must_use]
    pub fn new(title: impl Into<String>, data: Vec<DataPoint>) -> Self {
        Self {
            title: title.into(),
            data,
            ..Default::default()
        }
    }

    #[must_use]
    pub const fn with_size(mut self, width: f64, height: f64) -> Self {
        self.width = width;
        self.height = height;
        self
    }

    #[must_use]
    pub fn with_color(mut self, color: ChartColor) -> Self {
        self.bar_color = color;
        self
    }
}

impl SvgElement for BarChart {
    #[allow(clippy::cast_precision_loss)] // Acceptable for chart rendering
    fn render(&self) -> String {
        let mut output = String::new();

        // viewBox for responsive scaling
        // Note: <title> as first child provides accessible name without needing id/aria-labelledby
        let _ = writeln!(
            output,
            r#"<svg viewBox="0 0 {} {}" xmlns="http://www.w3.org/2000/svg" role="img">"#,
            self.width, self.height
        );

        // Accessible title
        let escaped_title = html_escape(&self.title);
        let _ = writeln!(output, r"    <title>{escaped_title}</title>");

        if self.data.is_empty() {
            // Empty state
            let text_color = ChartColor::css_var("text-muted").to_css();
            let _ = writeln!(
                output,
                r#"    <text x="{}" y="{}" text-anchor="middle" fill="{text_color}" font-size="14">No data available</text>"#,
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

            // Value label on top of bar
            if self.show_values {
                let text_color = ChartColor::css_var("text").to_css();
                #[allow(clippy::cast_possible_truncation)]
                let value_display = point.value as i64;
                let _ = writeln!(
                    output,
                    r#"    <text x="{}" y="{}" text-anchor="middle" fill="{text_color}" font-size="10">{value_display}</text>"#,
                    x + bar_width / 2.0,
                    y - 4.0
                );
            }

            // X-axis label
            let label_color = ChartColor::css_var("text-muted").to_css();
            let escaped_label = html_escape(&point.label);
            let _ = writeln!(
                output,
                r#"    <text x="{}" y="{}" text-anchor="middle" fill="{label_color}" font-size="10">{escaped_label}</text>"#,
                x + bar_width / 2.0,
                self.height - 8.0
            );
        }

        output.push_str("</svg>");
        output
    }
}

/// Horizontal bar chart - suited for language breakdown.
#[derive(Debug)]
pub struct HorizontalBarChart {
    pub title: String,
    pub data: Vec<DataPoint>,
    pub width: f64,
    pub height: f64,
    pub padding_left: f64,
    pub padding_right: f64,
    pub padding_top: f64,
    pub padding_bottom: f64,
    pub bar_height: f64,
    pub bar_gap: f64,
    pub bar_color: ChartColor,
    pub show_values: bool,
}

impl Default for HorizontalBarChart {
    fn default() -> Self {
        Self {
            title: String::new(),
            data: Vec::new(),
            width: 400.0,
            height: 62.0, // min height for 1 bar: padding_top + bar_height + bar_gap + padding_bottom
            padding_left: 100.0,
            padding_right: 60.0,
            padding_top: 20.0,
            padding_bottom: 10.0,
            bar_height: 24.0,
            bar_gap: 8.0,
            bar_color: ChartColor::css_var("chart-primary"),
            show_values: true,
        }
    }
}

impl HorizontalBarChart {
    #[must_use]
    #[allow(clippy::cast_precision_loss)] // Acceptable for chart sizing
    pub fn new(title: impl Into<String>, data: Vec<DataPoint>) -> Self {
        let defaults = Self::default();
        let data_len = data.len().max(1);
        let height = (defaults.bar_height + defaults.bar_gap).mul_add(
            data_len as f64,
            defaults.padding_top + defaults.padding_bottom,
        );

        Self {
            title: title.into(),
            data,
            height,
            ..defaults
        }
    }

    #[must_use]
    pub const fn with_width(mut self, width: f64) -> Self {
        self.width = width;
        self
    }

    #[must_use]
    pub fn with_color(mut self, color: ChartColor) -> Self {
        self.bar_color = color;
        self
    }
}

impl SvgElement for HorizontalBarChart {
    #[allow(clippy::cast_precision_loss)] // Acceptable for chart rendering
    fn render(&self) -> String {
        let mut output = String::new();

        let _ = writeln!(
            output,
            r#"<svg viewBox="0 0 {} {}" xmlns="http://www.w3.org/2000/svg" role="img">"#,
            self.width, self.height
        );

        let escaped_title = html_escape(&self.title);
        let _ = writeln!(output, r"    <title>{escaped_title}</title>");

        if self.data.is_empty() {
            let text_color = ChartColor::css_var("text-muted").to_css();
            let _ = writeln!(
                output,
                r#"    <text x="{}" y="{}" text-anchor="middle" fill="{text_color}" font-size="14">No language data</text>"#,
                self.width / 2.0,
                self.height / 2.0
            );
            output.push_str("</svg>");
            return output;
        }

        let chart_width = self.width - self.padding_left - self.padding_right;

        // Find max value
        let max_value = self
            .data
            .iter()
            .map(|d| d.value)
            .fold(0.0_f64, f64::max)
            .max(1.0);

        // Draw bars
        for (i, point) in self.data.iter().enumerate() {
            let y = (self.bar_height + self.bar_gap).mul_add(i as f64, self.padding_top);
            let bar_width = (point.value / max_value) * chart_width;

            let color = point
                .color
                .clone()
                .unwrap_or_else(|| self.bar_color.clone());

            let bar_element = Bar {
                x: self.padding_left,
                y,
                width: bar_width,
                height: self.bar_height,
                color,
                label: point.label.clone(),
                value: point.value,
            };

            let _ = writeln!(output, "    {}", bar_element.render());

            // Label on left
            let label_color = ChartColor::css_var("text").to_css();
            let escaped_label = html_escape(&point.label);
            let _ = writeln!(
                output,
                r#"    <text x="{}" y="{}" text-anchor="end" fill="{label_color}" font-size="12" dominant-baseline="middle">{escaped_label}</text>"#,
                self.padding_left - 8.0,
                y + self.bar_height / 2.0
            );

            // Value on right
            if self.show_values {
                let value_color = ChartColor::css_var("text-muted").to_css();
                #[allow(clippy::cast_possible_truncation)]
                let formatted = format_number(point.value as i64);
                let _ = writeln!(
                    output,
                    r#"    <text x="{}" y="{}" text-anchor="start" fill="{value_color}" font-size="11" dominant-baseline="middle">{formatted}</text>"#,
                    self.padding_left + bar_width + 6.0,
                    y + self.bar_height / 2.0
                );
            }
        }

        output.push_str("</svg>");
        output
    }
}

/// Line chart with automatic scaling for trend visualization.
#[derive(Debug)]
pub struct LineChart {
    pub title: String,
    pub data: Vec<DataPoint>,
    pub width: f64,
    pub height: f64,
    pub padding: f64,
    pub line_color: ChartColor,
    pub show_points: bool,
    pub show_area: bool,
}

impl Default for LineChart {
    fn default() -> Self {
        Self {
            title: String::new(),
            data: Vec::new(),
            width: 500.0,
            height: 200.0,
            padding: 50.0,
            line_color: ChartColor::css_var("chart-primary"),
            show_points: true,
            show_area: true,
        }
    }
}

impl LineChart {
    #[must_use]
    pub fn new(title: impl Into<String>, data: Vec<DataPoint>) -> Self {
        Self {
            title: title.into(),
            data,
            ..Default::default()
        }
    }

    #[must_use]
    pub const fn with_size(mut self, width: f64, height: f64) -> Self {
        self.width = width;
        self.height = height;
        self
    }

    #[must_use]
    pub fn with_color(mut self, color: ChartColor) -> Self {
        self.line_color = color;
        self
    }

    #[must_use]
    pub const fn with_points(mut self, show: bool) -> Self {
        self.show_points = show;
        self
    }

    #[must_use]
    pub const fn with_area(mut self, show: bool) -> Self {
        self.show_area = show;
        self
    }
}

impl SvgElement for LineChart {
    #[allow(clippy::cast_precision_loss)] // Acceptable for chart rendering
    fn render(&self) -> String {
        let mut output = String::new();

        let _ = writeln!(
            output,
            r#"<svg viewBox="0 0 {} {}" xmlns="http://www.w3.org/2000/svg" role="img">"#,
            self.width, self.height
        );

        let escaped_title = html_escape(&self.title);
        let _ = writeln!(output, r"    <title>{escaped_title}</title>");

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

        // Add 10% padding to Y range
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

        // Draw grid lines
        let grid_color = ChartColor::css_var("border").to_css();
        for i in 0..=4 {
            let y = (chart_height / 4.0).mul_add(f64::from(i), self.padding);
            let _ = writeln!(
                output,
                r#"    <line x1="{}" y1="{y}" x2="{}" y2="{y}" stroke="{grid_color}" stroke-width="1" stroke-dasharray="4,4" opacity="0.5"/>"#,
                self.padding,
                self.width - self.padding
            );

            // Y-axis labels
            let label_value = max_value - (value_range * f64::from(i) / 4.0);
            let label_color = ChartColor::css_var("text-muted").to_css();
            #[allow(clippy::cast_possible_truncation)]
            let formatted = format_number(label_value as i64);
            let _ = writeln!(
                output,
                r#"    <text x="{}" y="{y}" text-anchor="end" fill="{label_color}" font-size="10" dominant-baseline="middle">{formatted}</text>"#,
                self.padding - 8.0
            );
        }

        // Draw area under line
        if self.show_area && points.len() >= 2 {
            let mut area_path = String::new();
            let _ = write!(area_path, "M{},{baseline_y}", points[0].0);
            for (x, y) in &points {
                let _ = write!(area_path, " L{x},{y}");
            }
            let _ = write!(area_path, " L{},{baseline_y} Z", points[points.len() - 1].0);

            let area_color = self.line_color.to_css();
            let _ = writeln!(
                output,
                r#"    <path d="{area_path}" fill="{area_color}" fill-opacity="0.1" stroke="none"/>"#
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

            let line_color = self.line_color.to_css();
            let _ = writeln!(
                output,
                r#"    <path d="{line_path}" fill="none" stroke="{line_color}" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>"#
            );
        }

        // Draw points with hover titles
        if self.show_points {
            let point_color = self.line_color.to_css();
            for (i, ((x, y), data)) in points.iter().zip(self.data.iter()).enumerate() {
                let escaped_label = html_escape(&data.label);
                #[allow(clippy::cast_possible_truncation)]
                let value_display = data.value as i64;
                let _ = writeln!(
                    output,
                    r#"    <circle cx="{x}" cy="{y}" r="4" fill="{point_color}" stroke="var(--color-card, white)" stroke-width="2">
        <title>{escaped_label}: {value_display}</title>
    </circle>"#
                );

                // X-axis labels (show first, last, and some intermediate)
                let show_label = i == 0
                    || i == point_count - 1
                    || (point_count > 5 && i % (point_count / 5).max(1) == 0);

                if show_label {
                    let label_color = ChartColor::css_var("text-muted").to_css();
                    // Truncate long labels
                    let display_label = if data.label.len() > 10 {
                        format!("{}…", &data.label[..9])
                    } else {
                        data.label.clone()
                    };
                    let escaped = html_escape(&display_label);
                    let _ = writeln!(
                        output,
                        r#"    <text x="{x}" y="{}" text-anchor="middle" fill="{label_color}" font-size="9">{escaped}</text>"#,
                        baseline_y + 14.0
                    );
                }
            }
        }

        output.push_str("</svg>");
        output
    }
}

#[cfg(test)]
#[path = "chart_tests.rs"]
mod tests;
