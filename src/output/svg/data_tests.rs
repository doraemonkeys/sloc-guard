//! Tests for chart data model.

use super::*;
use crate::output::svg::style::ChartColor;

#[test]
fn new_creates_point() {
    let point = DataPoint::new("test", 100.0);
    assert_eq!(point.label, "test");
    // Use epsilon comparison for floats
    assert!((point.value - 100.0).abs() < f64::EPSILON);
    assert!(point.color.is_none());
}

#[test]
fn with_color_sets_color() {
    let point = DataPoint::new("test", 50.0).with_color(ChartColor::hex("#fff"));
    assert!(point.color.is_some());
}
