//! Tests for composite chart types.

use super::*;
use crate::output::svg::element::SvgElement;

mod bar_chart_tests {
    use super::*;

    #[test]
    fn bar_chart_renders_svg() {
        let data = vec![
            DataPoint::new("A", 10.0),
            DataPoint::new("B", 20.0),
            DataPoint::new("C", 15.0),
        ];

        let chart = BarChart::new("Test Chart", data);
        let svg = chart.render();

        assert!(svg.contains("<svg"));
        assert!(svg.contains("</svg>"));
        assert!(svg.contains("viewBox=\"0 0 400 200\""));
        assert!(svg.contains("<title"));
        assert!(svg.contains("Test Chart"));
    }

    #[test]
    fn bar_chart_with_custom_size() {
        let chart = BarChart::new("Custom", vec![DataPoint::new("X", 5.0)]).with_size(600.0, 300.0);

        let svg = chart.render();
        assert!(svg.contains("viewBox=\"0 0 600 300\""));
    }

    #[test]
    fn bar_chart_empty_shows_message() {
        let chart = BarChart::new("Empty Chart", vec![]);
        let svg = chart.render();

        assert!(svg.contains("No data available"));
    }

    #[test]
    fn bar_chart_has_accessibility_attributes() {
        let chart = BarChart::new("Accessible Chart", vec![DataPoint::new("A", 1.0)]);
        let svg = chart.render();

        assert!(svg.contains("role=\"img\""));
        // <title> as first child provides accessible name without needing id/aria-labelledby
        assert!(svg.contains("<title>Accessible Chart</title>"));
    }

    #[test]
    fn bar_chart_default_creates_empty_chart() {
        let chart = BarChart::default();
        assert!(chart.title.is_empty());
        assert!(chart.data.is_empty());
        assert!((chart.width - 400.0).abs() < f64::EPSILON);
        assert!((chart.height - 200.0).abs() < f64::EPSILON);
    }

    #[test]
    fn bar_chart_escapes_title() {
        let chart = BarChart::new("<script>alert('xss')</script>", vec![]);
        let svg = chart.render();

        assert!(svg.contains("&lt;script&gt;"));
        assert!(!svg.contains("<script>alert"));
    }
}

mod horizontal_bar_chart_tests {
    use super::*;

    #[test]
    fn horizontal_bar_chart_renders() {
        let data = vec![
            DataPoint::new("Rust", 1000.0),
            DataPoint::new("Python", 500.0),
            DataPoint::new("Go", 300.0),
        ];

        let chart = HorizontalBarChart::new("Language Breakdown", data);
        let svg = chart.render();

        assert!(svg.contains("<svg"));
        assert!(svg.contains("Language Breakdown"));
        assert!(svg.contains("Rust"));
        assert!(svg.contains("Python"));
        assert!(svg.contains("Go"));
    }

    #[test]
    fn horizontal_bar_chart_empty_shows_message() {
        let chart = HorizontalBarChart::new("Empty", vec![]);
        let svg = chart.render();

        assert!(svg.contains("No language data"));
    }

    #[test]
    fn horizontal_bar_chart_formats_large_numbers() {
        let data = vec![DataPoint::new("Large", 150_000.0)];
        let chart = HorizontalBarChart::new("Test", data);
        let svg = chart.render();

        // Should show "150.0K" instead of "150000"
        assert!(svg.contains("150.0K"));
    }

    #[test]
    fn horizontal_bar_chart_custom_width() {
        let chart =
            HorizontalBarChart::new("Wide", vec![DataPoint::new("A", 1.0)]).with_width(800.0);
        let svg = chart.render();

        assert!(svg.contains("viewBox=\"0 0 800"));
    }

    #[test]
    fn horizontal_bar_chart_default_creates_empty_chart() {
        let chart = HorizontalBarChart::default();
        assert!(chart.title.is_empty());
        assert!(chart.data.is_empty());
        assert!((chart.width - 400.0).abs() < f64::EPSILON);
    }

    #[test]
    fn horizontal_bar_chart_no_id_collision() {
        let chart = HorizontalBarChart::new("Test", vec![DataPoint::new("A", 1.0)]);
        let svg = chart.render();
        // No static IDs that could collide with other charts
        assert!(!svg.contains("id=\""));
    }
}

mod line_chart_tests {
    use super::*;

    #[test]
    fn line_chart_renders() {
        let data = vec![
            DataPoint::new("Jan", 100.0),
            DataPoint::new("Feb", 120.0),
            DataPoint::new("Mar", 90.0),
            DataPoint::new("Apr", 150.0),
        ];

        let chart = LineChart::new("Trend", data);
        let svg = chart.render();

        assert!(svg.contains("<svg"));
        assert!(svg.contains("<path"));
        assert!(svg.contains("<circle"));
        assert!(svg.contains("Trend"));
    }

    #[test]
    fn line_chart_empty_shows_message() {
        let chart = LineChart::new("No Data", vec![]);
        let svg = chart.render();

        assert!(svg.contains("No trend data"));
    }

    #[test]
    fn line_chart_without_points() {
        let data = vec![DataPoint::new("A", 10.0), DataPoint::new("B", 20.0)];
        let chart = LineChart::new("Test", data).with_points(false);
        let svg = chart.render();

        assert!(!svg.contains("<circle"));
    }

    #[test]
    fn line_chart_without_area() {
        let data = vec![DataPoint::new("A", 10.0), DataPoint::new("B", 20.0)];
        let chart = LineChart::new("Test", data).with_area(false);
        let svg = chart.render();

        // Area fill has fill-opacity="0.1", line has fill="none"
        assert!(!svg.contains("fill-opacity=\"0.1\""));
    }

    #[test]
    fn line_chart_custom_size() {
        let chart =
            LineChart::new("Custom", vec![DataPoint::new("X", 1.0)]).with_size(800.0, 400.0);
        let svg = chart.render();

        assert!(svg.contains("viewBox=\"0 0 800 400\""));
    }

    #[test]
    fn line_chart_shows_grid_lines() {
        let data = vec![DataPoint::new("A", 100.0)];
        let chart = LineChart::new("Grid", data);
        let svg = chart.render();

        // Grid lines have stroke-dasharray
        assert!(svg.contains("stroke-dasharray=\"4,4\""));
    }

    #[test]
    fn line_chart_has_hover_titles() {
        let data = vec![DataPoint::new("Point A", 42.0)];
        let chart = LineChart::new("Test", data);
        let svg = chart.render();

        assert!(svg.contains("<title>Point A: 42</title>"));
    }

    #[test]
    fn line_chart_default_creates_empty_chart() {
        let chart = LineChart::default();
        assert!(chart.title.is_empty());
        assert!(chart.data.is_empty());
        assert!((chart.width - 500.0).abs() < f64::EPSILON);
        assert!((chart.height - 200.0).abs() < f64::EPSILON);
        assert!(chart.show_points);
        assert!(chart.show_area);
    }

    #[test]
    fn line_chart_no_id_collision() {
        let chart = LineChart::new("Test", vec![DataPoint::new("A", 1.0)]);
        let svg = chart.render();
        // No static IDs that could collide with other charts
        assert!(!svg.contains("id=\""));
    }

    #[test]
    fn line_chart_single_point_handles_x_step() {
        // Regression test: single point should not panic on x_step calculation
        let data = vec![DataPoint::new("Single", 100.0)];
        let chart = LineChart::new("Single Point", data);
        let svg = chart.render();

        assert!(svg.contains("<svg"));
        assert!(svg.contains("<circle"));
    }
}
