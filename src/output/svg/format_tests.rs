//! Tests for SVG text formatting.

use super::*;

mod html_escape_tests {
    use super::*;

    #[test]
    fn escapes_ampersand() {
        assert_eq!(html_escape("A & B"), "A &amp; B");
    }

    #[test]
    fn escapes_angle_brackets() {
        assert_eq!(html_escape("<tag>"), "&lt;tag&gt;");
    }

    #[test]
    fn escapes_quotes() {
        assert_eq!(html_escape("\"quoted\""), "&quot;quoted&quot;");
        assert_eq!(html_escape("'single'"), "&#39;single&#39;");
    }

    #[test]
    fn escapes_multiple() {
        assert_eq!(
            html_escape("<a href=\"test\">&</a>"),
            "&lt;a href=&quot;test&quot;&gt;&amp;&lt;/a&gt;"
        );
    }
}

mod format_number_tests {
    use super::*;

    #[test]
    fn small_numbers_unchanged() {
        assert_eq!(format_number(0), "0");
        assert_eq!(format_number(999), "999");
        assert_eq!(format_number(9999), "9999");
    }

    #[test]
    fn thousands_show_k() {
        assert_eq!(format_number(10_000), "10.0K");
        assert_eq!(format_number(15_500), "15.5K");
        assert_eq!(format_number(999_999), "1000.0K");
    }

    #[test]
    fn millions_show_m() {
        assert_eq!(format_number(1_000_000), "1.0M");
        assert_eq!(format_number(2_500_000), "2.5M");
    }
}
