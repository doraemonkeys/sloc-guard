//! SVG chart generation primitives for HTML reports.
//!
//! Provides building blocks for creating accessible, responsive SVG charts:
//! - viewBox-based scaling (renders at any size)
//! - CSS variable integration (dark mode support via `var(--color-*)`)
//! - Accessibility: `<title>` elements for screen readers, ≥4.5:1 color contrast

mod builder;
mod chart;
mod data;
mod element;
mod format;
mod histogram;
mod style;

pub use builder::SvgBuilder;
pub use chart::{BarChart, HorizontalBarChart, LineChart};
pub use data::DataPoint;
pub use element::{Axis, AxisOrientation, Bar, Line, SvgElement};
pub use histogram::FileSizeHistogram;
pub use style::{ChartColor, TextAnchor};

#[cfg(test)]
#[path = "mod_tests.rs"]
mod tests;
