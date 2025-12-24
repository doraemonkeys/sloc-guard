//! Chart data model.

use super::style::ChartColor;

/// A single data point for charts.
#[derive(Debug, Clone)]
pub struct DataPoint {
    /// Label for this data point (shown on axis or tooltip)
    pub label: String,
    /// Numeric value
    pub value: f64,
    /// Optional color override
    pub color: Option<ChartColor>,
}

impl DataPoint {
    #[must_use]
    pub fn new(label: impl Into<String>, value: f64) -> Self {
        Self {
            label: label.into(),
            value,
            color: None,
        }
    }

    #[must_use]
    pub fn with_color(mut self, color: ChartColor) -> Self {
        self.color = Some(color);
        self
    }
}

#[cfg(test)]
#[path = "data_tests.rs"]
mod tests;
