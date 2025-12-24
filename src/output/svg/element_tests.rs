//! Tests for primitive SVG elements.

use super::*;

mod axis_tests {
    use super::*;

    #[test]
    fn horizontal_axis_renders() {
        let axis = Axis::horizontal(10.0, 100.0, 200.0).with_labels(vec![
            (0.0, "0".to_string()),
            (0.5, "50".to_string()),
            (1.0, "100".to_string()),
        ]);

        let svg = axis.render();
        assert!(svg.contains("<line"));
        assert!(svg.contains("<text"));
        assert!(svg.contains("text-anchor=\"middle\""));
    }

    #[test]
    fn vertical_axis_renders() {
        let axis = Axis::vertical(50.0, 150.0, 100.0)
            .with_labels(vec![(0.0, "0".to_string()), (1.0, "100".to_string())]);

        let svg = axis.render();
        assert!(svg.contains("<line"));
        assert!(svg.contains("text-anchor=\"end\""));
    }

    #[test]
    fn axis_with_custom_font_size() {
        let axis = Axis::horizontal(0.0, 0.0, 100.0)
            .with_font_size(14.0)
            .with_labels(vec![(0.5, "mid".to_string())]);

        let svg = axis.render();
        assert!(svg.contains("font-size=\"14\""));
    }
}

mod bar_tests {
    use super::*;

    #[test]
    fn bar_renders_with_title() {
        let bar = Bar {
            x: 10.0,
            y: 20.0,
            width: 50.0,
            height: 80.0,
            color: ChartColor::hex("#22c55e"),
            label: "Test Bar".to_string(),
            value: 100.0,
        };

        let svg = bar.render();
        assert!(svg.contains("<rect"));
        assert!(svg.contains("<title>"));
        assert!(svg.contains("Test Bar: 100"));
        assert!(svg.contains("fill=\"#22c55e\""));
    }

    #[test]
    fn bar_escapes_special_characters() {
        let bar = Bar {
            x: 0.0,
            y: 0.0,
            width: 10.0,
            height: 10.0,
            color: ChartColor::hex("#000"),
            label: "Test <script>".to_string(),
            value: 1.0,
        };

        let svg = bar.render();
        assert!(svg.contains("&lt;script&gt;"));
        assert!(!svg.contains("<script>"));
    }
}

mod line_tests {
    use super::*;

    #[test]
    fn line_renders_path() {
        let line = Line::new(
            vec![(0.0, 100.0), (50.0, 50.0), (100.0, 75.0)],
            ChartColor::css_var("chart-primary"),
        );

        let svg = line.render();
        assert!(svg.contains("<path"));
        assert!(svg.contains("M0,100 L50,50 L100,75"));
        assert!(svg.contains("stroke-width=\"2\""));
    }

    #[test]
    fn line_with_custom_stroke_width() {
        let line = Line::new(vec![(0.0, 0.0), (100.0, 100.0)], ChartColor::hex("#000"))
            .with_stroke_width(4.0);

        let svg = line.render();
        assert!(svg.contains("stroke-width=\"4\""));
    }

    #[test]
    fn empty_line_returns_empty_string() {
        let line = Line::new(vec![], ChartColor::hex("#000"));
        assert!(line.render().is_empty());
    }

    #[test]
    fn line_with_fill_and_baseline_renders_area() {
        let line = Line::new(
            vec![(10.0, 50.0), (50.0, 30.0), (90.0, 60.0)],
            ChartColor::hex("#22c55e"),
        )
        .with_fill(true)
        .with_baseline_y(100.0);

        let svg = line.render();
        // Should have two paths: fill area and line
        assert!(svg.matches("<path").count() >= 2);
        // Fill path should close to baseline_y (100)
        assert!(svg.contains("L90,100 L10,100 Z"));
        assert!(svg.contains("fill-opacity=\"0.1\""));
    }

    #[test]
    fn line_with_fill_without_baseline_skips_fill() {
        let line =
            Line::new(vec![(0.0, 50.0), (100.0, 50.0)], ChartColor::hex("#000")).with_fill(true);
        // baseline_y not set

        let svg = line.render();
        // Should only have the line path, no fill
        assert_eq!(svg.matches("<path").count(), 1);
        assert!(!svg.contains("fill-opacity"));
    }

    #[test]
    fn line_with_fill_single_point_skips_fill() {
        let line = Line::new(vec![(50.0, 50.0)], ChartColor::hex("#000"))
            .with_fill(true)
            .with_baseline_y(100.0);

        let svg = line.render();
        // Single point cannot form area, should only have line
        assert_eq!(svg.matches("<path").count(), 1);
    }
}
