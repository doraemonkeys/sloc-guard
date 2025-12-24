use std::path::PathBuf;

use crate::counter::LineStats;
use crate::output::svg::SvgElement;
use crate::output::svg::histogram::{DEFAULT_BUCKETS, FileSizeHistogram, SizeBucket};
use crate::output::{FileStatistics, ProjectStatistics};

fn make_file(path: &str, code: usize) -> FileStatistics {
    FileStatistics {
        path: PathBuf::from(path),
        stats: LineStats {
            total: code + 10,
            code,
            comment: 5,
            blank: 5,
            ignored: 0,
        },
        language: "Rust".to_string(),
    }
}

fn make_stats(code_lines: &[usize]) -> ProjectStatistics {
    let files: Vec<FileStatistics> = code_lines
        .iter()
        .enumerate()
        .map(|(i, &lines)| make_file(&format!("file{i}.rs"), lines))
        .collect();
    ProjectStatistics::new(files)
}

// ============================================================================
// SizeBucket tests
// ============================================================================

#[test]
fn bucket_contains_min_boundary() {
    let bucket = SizeBucket {
        min: 51,
        max: Some(101),
        label: "51-100",
    };
    assert!(bucket.contains(51));
}

#[test]
fn bucket_excludes_max_boundary() {
    let bucket = SizeBucket {
        min: 51,
        max: Some(101),
        label: "51-100",
    };
    assert!(!bucket.contains(101));
}

#[test]
fn bucket_contains_middle_value() {
    let bucket = SizeBucket {
        min: 51,
        max: Some(101),
        label: "51-100",
    };
    assert!(bucket.contains(75));
}

#[test]
fn bucket_excludes_below_min() {
    let bucket = SizeBucket {
        min: 51,
        max: Some(101),
        label: "51-100",
    };
    assert!(!bucket.contains(50));
}

#[test]
fn unbounded_bucket_contains_large_values() {
    let bucket = SizeBucket {
        min: 501,
        max: None,
        label: "501+",
    };
    assert!(bucket.contains(501));
    assert!(bucket.contains(1000));
    assert!(bucket.contains(10000));
}

#[test]
fn unbounded_bucket_excludes_below_min() {
    let bucket = SizeBucket {
        min: 501,
        max: None,
        label: "501+",
    };
    assert!(!bucket.contains(500));
}

// ============================================================================
// DEFAULT_BUCKETS tests
// ============================================================================

#[test]
fn default_buckets_cover_all_ranges() {
    // Zero lines
    assert!(DEFAULT_BUCKETS[0].contains(0));
    assert!(DEFAULT_BUCKETS[0].contains(50));

    // 51-100
    assert!(DEFAULT_BUCKETS[1].contains(51));
    assert!(DEFAULT_BUCKETS[1].contains(100));

    // 101-200
    assert!(DEFAULT_BUCKETS[2].contains(101));
    assert!(DEFAULT_BUCKETS[2].contains(200));

    // 201-500
    assert!(DEFAULT_BUCKETS[3].contains(201));
    assert!(DEFAULT_BUCKETS[3].contains(500));

    // 500+
    assert!(DEFAULT_BUCKETS[4].contains(501));
    assert!(DEFAULT_BUCKETS[4].contains(999));
}

#[test]
fn default_buckets_no_overlap() {
    for lines in [50, 51, 100, 101, 200, 201, 500, 501] {
        let matching_count = DEFAULT_BUCKETS.iter().filter(|b| b.contains(lines)).count();
        assert_eq!(matching_count, 1, "{lines} should match exactly one bucket");
    }
}

// ============================================================================
// FileSizeHistogram::from_stats tests
// ============================================================================

#[test]
fn from_stats_empty() {
    let stats = make_stats(&[]);
    let histogram = FileSizeHistogram::from_stats(&stats);

    assert_eq!(histogram.data.len(), 5);
    assert!(histogram.data.iter().all(|d| d.value == 0.0));
}

#[test]
fn from_stats_single_bucket() {
    let stats = make_stats(&[25, 30, 45]); // All in 0-50 bucket
    let histogram = FileSizeHistogram::from_stats(&stats);

    assert!((histogram.data[0].value - 3.0).abs() < f64::EPSILON);
    assert!((histogram.data[1].value - 0.0).abs() < f64::EPSILON);
    assert!((histogram.data[2].value - 0.0).abs() < f64::EPSILON);
    assert!((histogram.data[3].value - 0.0).abs() < f64::EPSILON);
    assert!((histogram.data[4].value - 0.0).abs() < f64::EPSILON);
}

#[test]
fn from_stats_multiple_buckets() {
    let stats = make_stats(&[25, 75, 150, 300, 600]); // One per bucket
    let histogram = FileSizeHistogram::from_stats(&stats);

    assert!((histogram.data[0].value - 1.0).abs() < f64::EPSILON); // 0-50
    assert!((histogram.data[1].value - 1.0).abs() < f64::EPSILON); // 51-100
    assert!((histogram.data[2].value - 1.0).abs() < f64::EPSILON); // 101-200
    assert!((histogram.data[3].value - 1.0).abs() < f64::EPSILON); // 201-500
    assert!((histogram.data[4].value - 1.0).abs() < f64::EPSILON); // 500+
}

#[test]
fn from_stats_boundary_values() {
    let stats = make_stats(&[0, 50, 51, 100, 101, 200, 201, 500, 501]);
    let histogram = FileSizeHistogram::from_stats(&stats);

    assert!((histogram.data[0].value - 2.0).abs() < f64::EPSILON); // 0-50: 0, 50
    assert!((histogram.data[1].value - 2.0).abs() < f64::EPSILON); // 51-100: 51, 100
    assert!((histogram.data[2].value - 2.0).abs() < f64::EPSILON); // 101-200: 101, 200
    assert!((histogram.data[3].value - 2.0).abs() < f64::EPSILON); // 201-500: 201, 500
    assert!((histogram.data[4].value - 1.0).abs() < f64::EPSILON); // 500+: 501
}

// ============================================================================
// FileSizeHistogram::has_sufficient_data tests
// ============================================================================

#[test]
fn has_sufficient_data_empty() {
    let stats = make_stats(&[]);
    let histogram = FileSizeHistogram::from_stats(&stats);
    assert!(!histogram.has_sufficient_data());
}

#[test]
fn has_sufficient_data_two_files() {
    let stats = make_stats(&[25, 50]);
    let histogram = FileSizeHistogram::from_stats(&stats);
    assert!(!histogram.has_sufficient_data());
}

#[test]
fn has_sufficient_data_three_files() {
    let stats = make_stats(&[25, 50, 75]);
    let histogram = FileSizeHistogram::from_stats(&stats);
    assert!(histogram.has_sufficient_data());
}

// ============================================================================
// FileSizeHistogram::render tests
// ============================================================================

#[test]
fn render_empty_state() {
    let stats = make_stats(&[]);
    let histogram = FileSizeHistogram::from_stats(&stats);
    let svg = histogram.render();

    assert!(svg.contains("<svg"));
    assert!(svg.contains("<title>File Size Distribution</title>"));
    assert!(svg.contains("No files to display"));
    assert!(svg.contains("</svg>"));
}

#[test]
fn render_insufficient_files() {
    let stats = make_stats(&[25, 50]);
    let histogram = FileSizeHistogram::from_stats(&stats);
    let svg = histogram.render();

    assert!(svg.contains("Not enough files for histogram"));
}

#[test]
fn render_with_data() {
    let stats = make_stats(&[25, 75, 150]);
    let histogram = FileSizeHistogram::from_stats(&stats);
    let svg = histogram.render();

    assert!(svg.contains("<svg"));
    assert!(svg.contains("role=\"img\""));
    assert!(svg.contains("<title>File Size Distribution</title>"));
    // Should have bars
    assert!(svg.contains("<rect"));
    // Should have labels
    assert!(svg.contains("0-50"));
    assert!(svg.contains("51-100"));
    assert!(svg.contains("101-200"));
    assert!(svg.contains("</svg>"));
}

#[test]
fn render_shows_file_counts() {
    let stats = make_stats(&[25, 30, 35, 75]); // 3 in first bucket, 1 in second
    let histogram = FileSizeHistogram::from_stats(&stats);
    let svg = histogram.render();

    // File counts should appear as text labels
    assert!(svg.contains(">3<")); // 3 files in 0-50
    assert!(svg.contains(">1<")); // 1 file in 51-100
}

#[test]
fn render_has_axis_title() {
    let stats = make_stats(&[25, 50, 75]);
    let histogram = FileSizeHistogram::from_stats(&stats);
    let svg = histogram.render();

    assert!(svg.contains("Lines of Code"));
}

#[test]
fn render_uses_css_variables() {
    let stats = make_stats(&[25, 50, 75]);
    let histogram = FileSizeHistogram::from_stats(&stats);
    let svg = histogram.render();

    assert!(svg.contains("var(--color-"));
}

// ============================================================================
// Builder pattern tests
// ============================================================================

#[test]
fn with_title() {
    let stats = make_stats(&[25, 50, 75]);
    let histogram = FileSizeHistogram::from_stats(&stats).with_title("Custom Title");
    let svg = histogram.render();

    assert!(svg.contains("<title>Custom Title</title>"));
}

#[test]
fn with_size() {
    let stats = make_stats(&[25, 50, 75]);
    let histogram = FileSizeHistogram::from_stats(&stats).with_size(600.0, 300.0);

    assert!((histogram.width - 600.0).abs() < f64::EPSILON);
    assert!((histogram.height - 300.0).abs() < f64::EPSILON);
}
