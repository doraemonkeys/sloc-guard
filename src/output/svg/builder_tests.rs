//! Tests for SVG composition builder.

use super::*;
use crate::output::svg::element::Bar;
use crate::output::svg::style::ChartColor;

#[test]
fn builder_creates_svg() {
    let svg = SvgBuilder::new(200.0, 100.0)
        .with_title("Custom SVG")
        .build();

    assert!(svg.contains("<svg"));
    assert!(svg.contains("viewBox=\"0 0 200 100\""));
    assert!(svg.contains("<title>Custom SVG</title>"));
    assert!(svg.contains("</svg>"));
    // No static IDs
    assert!(!svg.contains("id=\""));
}

#[test]
fn builder_adds_elements() {
    let bar = Bar {
        x: 10.0,
        y: 10.0,
        width: 30.0,
        height: 50.0,
        color: ChartColor::hex("#000"),
        label: "Bar".to_string(),
        value: 50.0,
    };

    let svg = SvgBuilder::new(100.0, 100.0).push_element(&bar).build();

    assert!(svg.contains("<rect"));
}

#[test]
fn builder_adds_raw_svg() {
    let svg = SvgBuilder::new(100.0, 100.0)
        .push_raw("<circle cx=\"50\" cy=\"50\" r=\"25\"/>")
        .build();

    assert!(svg.contains("<circle cx=\"50\""));
}
