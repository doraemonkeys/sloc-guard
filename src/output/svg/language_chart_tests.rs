use std::path::PathBuf;

use crate::counter::LineStats;
use crate::output::svg::SvgElement;
use crate::output::svg::language_chart::LanguageBreakdownChart;
use crate::output::{FileStatistics, ProjectStatistics};

fn make_file(path: &str, language: &str, code: usize) -> FileStatistics {
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

fn make_stats_with_languages(files: Vec<FileStatistics>) -> ProjectStatistics {
    ProjectStatistics::new(files).with_language_breakdown()
}

// ============================================================================
// LanguageBreakdownChart::from_stats tests
// ============================================================================

#[test]
fn from_stats_no_language_data() {
    // Stats without calling with_language_breakdown()
    let stats = ProjectStatistics::new(vec![make_file("test.rs", "Rust", 100)]);
    let chart = LanguageBreakdownChart::from_stats(&stats);

    assert!(!chart.has_data());
}

#[test]
fn from_stats_empty_files() {
    let stats = make_stats_with_languages(vec![]);
    let chart = LanguageBreakdownChart::from_stats(&stats);

    assert!(!chart.has_data());
}

#[test]
fn from_stats_single_language() {
    let files = vec![
        make_file("a.rs", "Rust", 100),
        make_file("b.rs", "Rust", 200),
    ];
    let stats = make_stats_with_languages(files);
    let chart = LanguageBreakdownChart::from_stats(&stats);

    assert!(chart.has_data());
}

#[test]
fn from_stats_multiple_languages() {
    let files = vec![
        make_file("main.rs", "Rust", 200),
        make_file("lib.rs", "Rust", 100),
        make_file("app.go", "Go", 150),
        make_file("main.py", "Python", 50),
    ];
    let stats = make_stats_with_languages(files);
    let chart = LanguageBreakdownChart::from_stats(&stats);

    assert!(chart.has_data());
}

// ============================================================================
// LanguageBreakdownChart::render tests
// ============================================================================

#[test]
fn render_empty_state() {
    let stats = ProjectStatistics::new(vec![]);
    let chart = LanguageBreakdownChart::from_stats(&stats);
    let svg = chart.render();

    assert!(svg.contains("<svg"));
    assert!(svg.contains("<title>Language Breakdown by SLOC</title>"));
    assert!(svg.contains("No language data"));
    assert!(svg.contains("</svg>"));
}

#[test]
fn render_single_language() {
    let files = vec![
        make_file("a.rs", "Rust", 100),
        make_file("b.rs", "Rust", 200),
    ];
    let stats = make_stats_with_languages(files);
    let chart = LanguageBreakdownChart::from_stats(&stats);
    let svg = chart.render();

    assert!(svg.contains("<svg"));
    assert!(svg.contains("role=\"img\""));
    assert!(svg.contains("<rect")); // Horizontal bar
    assert!(svg.contains("Rust"));
    // Total code lines: 300
    assert!(svg.contains("300"));
    assert!(svg.contains("</svg>"));
}

#[test]
fn render_multiple_languages_sorted_by_sloc() {
    let files = vec![
        make_file("main.rs", "Rust", 200),
        make_file("lib.rs", "Rust", 100),
        make_file("app.go", "Go", 500), // Most code
        make_file("main.py", "Python", 50),
    ];
    let stats = make_stats_with_languages(files);
    let chart = LanguageBreakdownChart::from_stats(&stats);
    let svg = chart.render();

    // All languages should appear
    assert!(svg.contains("Rust"));
    assert!(svg.contains("Go"));
    assert!(svg.contains("Python"));

    // Go has most lines (500), should have value displayed
    assert!(svg.contains("500"));
    // Rust: 200 + 100 = 300
    assert!(svg.contains("300"));
    // Python: 50
    assert!(svg.contains("50"));
}

#[test]
fn render_uses_css_variables() {
    let files = vec![make_file("a.rs", "Rust", 100)];
    let stats = make_stats_with_languages(files);
    let chart = LanguageBreakdownChart::from_stats(&stats);
    let svg = chart.render();

    assert!(svg.contains("var(--color-"));
}

#[test]
fn render_has_accessible_title() {
    let files = vec![make_file("a.rs", "Rust", 100)];
    let stats = make_stats_with_languages(files);
    let chart = LanguageBreakdownChart::from_stats(&stats);
    let svg = chart.render();

    assert!(svg.contains("<title>Language Breakdown by SLOC</title>"));
}

#[test]
fn render_hover_tooltips() {
    let files = vec![make_file("a.rs", "Rust", 100)];
    let stats = make_stats_with_languages(files);
    let chart = LanguageBreakdownChart::from_stats(&stats);
    let svg = chart.render();

    // Bar elements should have title for hover
    assert!(svg.contains("<title>Rust: 100</title>"));
}

// ============================================================================
// Builder pattern tests
// ============================================================================

#[test]
fn with_width() {
    let files = vec![make_file("a.rs", "Rust", 100)];
    let stats = make_stats_with_languages(files);
    let chart = LanguageBreakdownChart::from_stats(&stats).with_width(600.0);
    let svg = chart.render();

    // viewBox should reflect the new width
    assert!(svg.contains("viewBox=\"0 0 600"));
}

#[test]
fn with_color() {
    use crate::output::svg::ChartColor;

    let files = vec![make_file("a.rs", "Rust", 100)];
    let stats = make_stats_with_languages(files);
    let chart = LanguageBreakdownChart::from_stats(&stats).with_color(ChartColor::hex("#ff5733"));
    let svg = chart.render();

    assert!(svg.contains("#ff5733"));
}

// ============================================================================
// Edge case tests
// ============================================================================

#[test]
fn handles_special_characters_in_language_names() {
    let files = vec![make_file("test.cpp", "C++", 100)];
    let stats = make_stats_with_languages(files);
    let chart = LanguageBreakdownChart::from_stats(&stats);
    let svg = chart.render();

    // C++ should be HTML-escaped in the SVG (+ is safe, but & < > would be escaped)
    assert!(svg.contains("C++"));
}

#[test]
fn handles_language_name_with_ampersand() {
    // Simulating a language with special HTML characters
    let files = vec![make_file("test.x", "A&B", 100)];
    let stats = make_stats_with_languages(files);
    let chart = LanguageBreakdownChart::from_stats(&stats);
    let svg = chart.render();

    // Ampersand should be escaped
    assert!(svg.contains("A&amp;B"));
}
