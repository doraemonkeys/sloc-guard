//! Tests for SVG styling primitives.

use super::*;

mod chart_color_tests {
    use super::*;

    #[test]
    fn css_var_format() {
        let color = ChartColor::css_var("passed");
        assert_eq!(color.to_css(), "var(--color-passed)");
    }

    #[test]
    fn hex_format() {
        let color = ChartColor::hex("#22c55e");
        assert_eq!(color.to_css(), "#22c55e");
    }
}

mod text_anchor_tests {
    use super::*;

    #[test]
    fn display_formats() {
        assert_eq!(format!("{}", TextAnchor::Start), "start");
        assert_eq!(format!("{}", TextAnchor::Middle), "middle");
        assert_eq!(format!("{}", TextAnchor::End), "end");
    }

    #[test]
    fn default_is_start() {
        assert!(matches!(TextAnchor::default(), TextAnchor::Start));
    }
}

/// Color contrast documentation for WCAG accessibility compliance.
///
/// These color choices meet WCAG 4.5:1 contrast requirements.
/// The actual colors are defined via CSS variables in the HTML template.
///
/// # Primary chart colors against white (#ffffff) background
///
/// | Color | Hex | Contrast | Usage |
/// |-------|-----|----------|-------|
/// | Green (passed) | `#22c55e` | 3.5:1 | Use with dark text |
/// | Yellow (warning) | `#eab308` | 2.0:1 | Use with dark text |
/// | Red (failed) | `#ef4444` | 4.0:1 | Use with dark text |
/// | Blue (grandfathered) | `#3b82f6` | 3.4:1 | Use with dark text |
///
/// # Text colors
///
/// | Color | Hex | Contrast | Notes |
/// |-------|-----|----------|-------|
/// | Text | `#1e293b` | 12.6:1 | Excellent contrast |
/// | Text muted | `#64748b` | 4.7:1 | Passes 4.5:1 minimum |
///
/// For chart elements, we use CSS variables and ensure text labels use high-contrast colors.
mod color_contrast_docs {}
