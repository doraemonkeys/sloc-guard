//! Language breakdown horizontal bar chart for HTML reports.

use super::chart::HorizontalBarChart;
use super::data::DataPoint;
use super::element::SvgElement;
use super::style::ChartColor;
use crate::output::ProjectStatistics;

/// Horizontal bar chart showing SLOC breakdown by programming language.
///
/// Data is sorted by code lines (descending) to highlight dominant languages.
#[derive(Debug)]
pub struct LanguageBreakdownChart {
    /// Inner horizontal bar chart for rendering
    chart: HorizontalBarChart,
    /// Whether there's language data to display
    has_data: bool,
}

impl LanguageBreakdownChart {
    /// Create chart from project statistics.
    ///
    /// Extracts `by_language` data and converts to horizontal bar format.
    /// If `by_language` is `None` or empty, the chart displays empty state.
    #[must_use]
    #[allow(clippy::cast_precision_loss)]
    pub fn from_stats(stats: &ProjectStatistics) -> Self {
        let data = Self::extract_language_data(stats);
        let has_data = !data.is_empty();

        let chart = HorizontalBarChart::new("Language Breakdown by SLOC", data)
            .with_color(ChartColor::css_var("chart-primary"));

        Self { chart, has_data }
    }

    /// Check if there's language data to display.
    #[must_use]
    pub const fn has_data(&self) -> bool {
        self.has_data
    }

    /// Set chart width.
    #[must_use]
    pub fn with_width(mut self, width: f64) -> Self {
        self.chart = self.chart.with_width(width);
        self
    }

    /// Set bar color.
    #[must_use]
    pub fn with_color(mut self, color: ChartColor) -> Self {
        self.chart = self.chart.with_color(color);
        self
    }

    /// Extract language breakdown data as `DataPoint`s.
    ///
    /// Returns data points sorted by code lines (descending), as
    /// `ProjectStatistics.by_language` is pre-sorted.
    #[allow(clippy::cast_precision_loss)]
    fn extract_language_data(stats: &ProjectStatistics) -> Vec<DataPoint> {
        let Some(ref by_language) = stats.by_language else {
            return Vec::new();
        };

        by_language
            .iter()
            .map(|lang| DataPoint::new(&lang.language, lang.code as f64))
            .collect()
    }
}

impl SvgElement for LanguageBreakdownChart {
    fn render(&self) -> String {
        self.chart.render()
    }
}

#[cfg(test)]
#[path = "language_chart_tests.rs"]
mod tests;
