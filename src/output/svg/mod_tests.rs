//! Tests for SVG module exports.

use super::*;

#[test]
fn exports_are_available() {
    // Verify all public types are exported
    let _: ChartColor = ChartColor::hex("#000");
    let _: TextAnchor = TextAnchor::Middle;
    let _: AxisOrientation = AxisOrientation::Horizontal;
    let _: DataPoint = DataPoint::new("test", 100.0);
}

#[test]
fn data_point_with_color() {
    let point = DataPoint::new("label", 42.0).with_color(ChartColor::hex("#ff0000"));

    assert_eq!(point.label, "label");
    // Use epsilon comparison for floats
    assert!((point.value - 42.0).abs() < f64::EPSILON);
    assert!(point.color.is_some());
}
