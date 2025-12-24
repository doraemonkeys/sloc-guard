//! Primitive SVG elements: axes, bars, and lines.

use std::fmt::Write;

use super::format::html_escape;
use super::style::{ChartColor, TextAnchor};

/// Axis orientation for charts.
#[derive(Debug, Clone, Copy)]
pub enum AxisOrientation {
    Horizontal,
    Vertical,
}

/// Base trait for SVG elements.
pub trait SvgElement {
    /// Render the element to an SVG string.
    fn render(&self) -> String;
}

/// Tick position and label for axis rendering.
struct TickInfo {
    start_x: f64,
    start_y: f64,
    end_x: f64,
    end_y: f64,
    label_x: f64,
    label_y: f64,
    anchor: TextAnchor,
}

/// Axis component for charts.
#[derive(Debug, Clone)]
pub struct Axis {
    pub orientation: AxisOrientation,
    pub x: f64,
    pub y: f64,
    pub length: f64,
    pub labels: Vec<(f64, String)>,
    pub color: ChartColor,
    pub tick_length: f64,
    pub font_size: f64,
}

impl Axis {
    #[must_use]
    pub fn horizontal(x: f64, y: f64, length: f64) -> Self {
        Self {
            orientation: AxisOrientation::Horizontal,
            x,
            y,
            length,
            labels: Vec::new(),
            color: ChartColor::css_var("text-muted"),
            tick_length: 5.0,
            font_size: 10.0,
        }
    }

    #[must_use]
    pub fn vertical(x: f64, y: f64, length: f64) -> Self {
        Self {
            orientation: AxisOrientation::Vertical,
            x,
            y,
            length,
            labels: Vec::new(),
            color: ChartColor::css_var("text-muted"),
            tick_length: 5.0,
            font_size: 10.0,
        }
    }

    #[must_use]
    pub fn with_labels(mut self, labels: Vec<(f64, String)>) -> Self {
        self.labels = labels;
        self
    }

    #[must_use]
    pub fn with_color(mut self, color: ChartColor) -> Self {
        self.color = color;
        self
    }

    #[must_use]
    pub const fn with_font_size(mut self, size: f64) -> Self {
        self.font_size = size;
        self
    }

    fn calculate_tick(&self, pos: f64) -> TickInfo {
        match self.orientation {
            AxisOrientation::Horizontal => {
                let tick_x = pos.mul_add(self.length, self.x);
                TickInfo {
                    start_x: tick_x,
                    start_y: self.y,
                    end_x: tick_x,
                    end_y: self.y + self.tick_length,
                    label_x: tick_x,
                    label_y: self.y + self.tick_length + self.font_size + 2.0,
                    anchor: TextAnchor::Middle,
                }
            }
            AxisOrientation::Vertical => {
                let tick_y = pos.mul_add(-self.length, self.y);
                TickInfo {
                    start_x: self.x,
                    start_y: tick_y,
                    end_x: self.x - self.tick_length,
                    end_y: tick_y,
                    label_x: self.x - self.tick_length - 4.0,
                    label_y: tick_y + self.font_size / 3.0,
                    anchor: TextAnchor::End,
                }
            }
        }
    }
}

impl SvgElement for Axis {
    fn render(&self) -> String {
        let mut output = String::new();
        let color = self.color.to_css();

        // Main axis line
        let (end_x, end_y) = match self.orientation {
            AxisOrientation::Horizontal => (self.x + self.length, self.y),
            AxisOrientation::Vertical => (self.x, self.y - self.length),
        };

        let _ = writeln!(
            output,
            r#"<line x1="{}" y1="{}" x2="{end_x}" y2="{end_y}" stroke="{color}" stroke-width="1"/>"#,
            self.x, self.y
        );

        // Ticks and labels
        for (pos, label) in &self.labels {
            let tick = self.calculate_tick(*pos);

            let _ = writeln!(
                output,
                r#"<line x1="{}" y1="{}" x2="{}" y2="{}" stroke="{color}" stroke-width="1"/>"#,
                tick.start_x, tick.start_y, tick.end_x, tick.end_y
            );

            let escaped_label = html_escape(label);
            let _ = writeln!(
                output,
                r#"<text x="{}" y="{}" text-anchor="{}" fill="{color}" font-size="{}">{escaped_label}</text>"#,
                tick.label_x, tick.label_y, tick.anchor, self.font_size
            );
        }

        output
    }
}

/// A single bar in a bar chart.
#[derive(Debug, Clone)]
pub struct Bar {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub color: ChartColor,
    pub label: String,
    pub value: f64,
}

impl SvgElement for Bar {
    fn render(&self) -> String {
        let color = self.color.to_css();
        let escaped_label = html_escape(&self.label);
        // Accessibility: title element for screen readers and hover tooltip
        format!(
            r#"<rect x="{}" y="{}" width="{}" height="{}" fill="{color}" rx="2">
    <title>{escaped_label}: {}</title>
</rect>"#,
            self.x, self.y, self.width, self.height, self.value
        )
    }
}

/// A line segment in a line chart.
#[derive(Debug, Clone)]
pub struct Line {
    pub points: Vec<(f64, f64)>,
    pub color: ChartColor,
    pub stroke_width: f64,
    pub fill: bool,
    pub fill_opacity: f64,
    /// Y-coordinate of the baseline for fill area. Required when `fill=true`.
    /// In SVG coordinates, higher values are lower on screen.
    pub baseline_y: Option<f64>,
}

impl Line {
    #[must_use]
    pub const fn new(points: Vec<(f64, f64)>, color: ChartColor) -> Self {
        Self {
            points,
            color,
            stroke_width: 2.0,
            fill: false,
            fill_opacity: 0.1,
            baseline_y: None,
        }
    }

    /// Enable fill area under the line. Requires `baseline_y` to be set for correct rendering.
    #[must_use]
    pub const fn with_fill(mut self, fill: bool) -> Self {
        self.fill = fill;
        self
    }

    /// Set the baseline Y-coordinate for fill area (bottom of chart in SVG coords).
    #[must_use]
    pub const fn with_baseline_y(mut self, y: f64) -> Self {
        self.baseline_y = Some(y);
        self
    }

    #[must_use]
    pub const fn with_stroke_width(mut self, width: f64) -> Self {
        self.stroke_width = width;
        self
    }
}

impl SvgElement for Line {
    fn render(&self) -> String {
        if self.points.is_empty() {
            return String::new();
        }

        let color = self.color.to_css();

        // Build path
        let mut path = String::new();
        for (i, (x, y)) in self.points.iter().enumerate() {
            if i == 0 {
                let _ = write!(path, "M{x},{y}");
            } else {
                let _ = write!(path, " L{x},{y}");
            }
        }

        let mut output = String::new();

        // Optional fill area (under the line) - requires baseline_y to be set
        // Silently skip fill if baseline_y not set (design decision: fail-safe)
        if self.fill
            && self.points.len() >= 2
            && let Some(baseline_y) = self.baseline_y
        {
            let first_x = self.points[0].0;
            let last_x = self.points[self.points.len() - 1].0;
            let mut fill_path = path.clone();
            let _ = write!(
                fill_path,
                " L{last_x},{baseline_y} L{first_x},{baseline_y} Z"
            );
            let _ = writeln!(
                output,
                r#"<path d="{fill_path}" fill="{color}" fill-opacity="{}" stroke="none"/>"#,
                self.fill_opacity
            );
        }

        // Main line
        let _ = writeln!(
            output,
            r#"<path d="{path}" fill="none" stroke="{color}" stroke-width="{}" stroke-linecap="round" stroke-linejoin="round"/>"#,
            self.stroke_width
        );

        output
    }
}

#[cfg(test)]
#[path = "element_tests.rs"]
mod tests;
