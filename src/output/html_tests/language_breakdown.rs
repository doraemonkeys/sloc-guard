//! Tests for language breakdown chart in HTML output.

use std::path::PathBuf;

use crate::counter::LineStats;
use crate::output::{FileStatistics, HtmlFormatter, OutputFormatter, ProjectStatistics};

use super::make_passed_result;

fn make_file_stats(path: &str, code: usize, language: &str) -> FileStatistics {
    FileStatistics {
        path: PathBuf::from(path),
        stats: LineStats {
            total: code + 10,
            code,
            comment: 5,
            blank: 5,
            ignored: 0,
        },
        language: language.to_string(),
    }
}

fn make_project_stats(code_lines: &[usize]) -> ProjectStatistics {
    let files: Vec<FileStatistics> = code_lines
        .iter()
        .enumerate()
        .map(|(i, &lines)| make_file_stats(&format!("file{i}.rs"), lines, "Rust"))
        .collect();
    ProjectStatistics::new(files)
}

fn make_project_stats_with_languages(files: Vec<FileStatistics>) -> ProjectStatistics {
    ProjectStatistics::new(files).with_language_breakdown()
}

#[test]
fn language_chart_not_shown_without_stats() {
    let results = vec![make_passed_result("src/test.rs", 100, 500)];
    let formatter = HtmlFormatter::new();
    let output = formatter.format(&results).unwrap();

    assert!(!output.contains("Language Breakdown"));
}

#[test]
fn language_chart_not_shown_without_language_breakdown() {
    let results = vec![make_passed_result("src/test.rs", 100, 500)];
    // Stats without with_language_breakdown()
    let stats = make_project_stats(&[25, 50, 75]);

    let formatter = HtmlFormatter::new().with_stats(stats);
    let output = formatter.format(&results).unwrap();

    // Histogram should appear but not language chart
    assert!(output.contains("File Size Distribution"));
    assert!(!output.contains("Language Breakdown"));
}

#[test]
fn language_chart_shown_with_language_breakdown() {
    let results = vec![make_passed_result("src/test.rs", 100, 500)];
    let files = vec![
        make_file_stats("main.rs", 200, "Rust"),
        make_file_stats("lib.rs", 100, "Rust"),
        make_file_stats("app.go", 150, "Go"),
    ];
    let stats = make_project_stats_with_languages(files);

    let formatter = HtmlFormatter::new().with_stats(stats);
    let output = formatter.format(&results).unwrap();

    // Language breakdown section should be present
    assert!(output.contains("Language Breakdown"));
    assert!(output.contains("Rust"));
    assert!(output.contains("Go"));
}

#[test]
fn language_chart_shows_sloc_values() {
    let results = vec![make_passed_result("src/test.rs", 100, 500)];
    let files = vec![
        make_file_stats("main.rs", 200, "Rust"),
        make_file_stats("lib.rs", 100, "Rust"),
    ];
    let stats = make_project_stats_with_languages(files);

    let formatter = HtmlFormatter::new().with_stats(stats);
    let output = formatter.format(&results).unwrap();

    // Total Rust SLOC: 200 + 100 = 300
    assert!(output.contains("300"));
}

#[test]
fn language_chart_has_horizontal_bars() {
    let results = vec![make_passed_result("src/test.rs", 100, 500)];
    let files = vec![
        make_file_stats("main.rs", 200, "Rust"),
        make_file_stats("app.go", 150, "Go"),
    ];
    let stats = make_project_stats_with_languages(files);

    let formatter = HtmlFormatter::new().with_stats(stats);
    let output = formatter.format(&results).unwrap();

    // Should have rect elements for bars
    assert!(output.contains("<rect"));
}

#[test]
fn both_charts_shown_when_data_available() {
    let results = vec![make_passed_result("src/test.rs", 100, 500)];
    let files = vec![
        make_file_stats("a.rs", 25, "Rust"),
        make_file_stats("b.rs", 75, "Rust"),
        make_file_stats("c.go", 150, "Go"),
    ];
    let stats = make_project_stats_with_languages(files);

    let formatter = HtmlFormatter::new().with_stats(stats);
    let output = formatter.format(&results).unwrap();

    // Both charts should be present
    assert!(output.contains("File Size Distribution"));
    assert!(output.contains("Language Breakdown"));
}

#[test]
fn only_language_chart_when_insufficient_histogram_files() {
    let results = vec![make_passed_result("src/test.rs", 100, 500)];
    // Only 2 files (not enough for histogram), but has language data
    let files = vec![
        make_file_stats("main.rs", 200, "Rust"),
        make_file_stats("app.go", 150, "Go"),
    ];
    let stats = make_project_stats_with_languages(files);

    let formatter = HtmlFormatter::new().with_stats(stats);
    let output = formatter.format(&results).unwrap();

    // Visualization section should still appear for language chart
    assert!(output.contains("Visualizations"));
    assert!(output.contains("Language Breakdown"));
    // But histogram should not appear
    assert!(!output.contains("File Size Distribution"));
}

#[test]
fn language_chart_uses_css_variables() {
    let results = vec![make_passed_result("src/test.rs", 100, 500)];
    let files = vec![make_file_stats("main.rs", 200, "Rust")];
    let stats = make_project_stats_with_languages(files);

    let formatter = HtmlFormatter::new().with_stats(stats);
    let output = formatter.format(&results).unwrap();

    // Chart should use CSS variables for theming
    assert!(output.contains("var(--color-"));
}
