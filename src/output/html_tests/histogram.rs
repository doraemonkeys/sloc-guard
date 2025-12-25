//! Tests for file size distribution histogram chart in HTML output.

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

#[test]
fn histogram_not_shown_without_stats() {
    let results = vec![make_passed_result("src/test.rs", 100, 500)];
    let formatter = HtmlFormatter::new();
    let output = formatter.format(&results).unwrap();

    // No histogram chart without stats (CSS is always present, but no chart HTML)
    assert!(!output.contains("File Size Distribution (by SLOC)"));
    assert!(!output.contains("<h2>Visualizations</h2>"));
}

#[test]
fn histogram_not_shown_with_insufficient_files() {
    let results = vec![make_passed_result("src/test.rs", 100, 500)];
    let stats = make_project_stats(&[25, 50]); // Only 2 files

    let formatter = HtmlFormatter::new().with_stats(stats);
    let output = formatter.format(&results).unwrap();

    // Should not show charts with < 3 files
    assert!(!output.contains("File Size Distribution (by SLOC)"));
    assert!(!output.contains("<h2>Visualizations</h2>"));
}

#[test]
fn histogram_shown_with_sufficient_files() {
    let results = vec![make_passed_result("src/test.rs", 100, 500)];
    let stats = make_project_stats(&[25, 75, 150]); // 3 files

    let formatter = HtmlFormatter::new().with_stats(stats);
    let output = formatter.format(&results).unwrap();

    // Charts section should be present
    assert!(output.contains("charts-section"));
    assert!(output.contains("Visualizations"));
    assert!(output.contains("File Size Distribution"));
    assert!(output.contains("<svg"));
}

#[test]
fn histogram_has_chart_css_variable() {
    let results = vec![make_passed_result("src/test.rs", 100, 500)];

    let formatter = HtmlFormatter::new();
    let output = formatter.format(&results).unwrap();

    // Chart CSS variable should be defined
    assert!(output.contains("--color-chart-primary"));
}

#[test]
fn histogram_has_chart_container_styles() {
    let results = vec![make_passed_result("src/test.rs", 100, 500)];

    let formatter = HtmlFormatter::new();
    let output = formatter.format(&results).unwrap();

    // Chart container styles should be defined
    assert!(output.contains(".chart-container"));
}
