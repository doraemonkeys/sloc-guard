use crate::output::{ColorMode, OutputFormat, ProjectStatistics};

use super::*;

#[test]
fn format_stats_output_text() {
    let stats = ProjectStatistics::new(vec![]);
    let output =
        format_stats_output(OutputFormat::Text, &stats, ColorMode::Never, None, None).unwrap();
    assert!(output.contains("Summary"));
}

#[test]
fn format_stats_output_json() {
    let stats = ProjectStatistics::new(vec![]);
    let output =
        format_stats_output(OutputFormat::Json, &stats, ColorMode::Never, None, None).unwrap();
    assert!(output.contains("summary"));
}

#[test]
fn format_stats_output_sarif_not_implemented() {
    let stats = ProjectStatistics::new(vec![]);
    let result = format_stats_output(OutputFormat::Sarif, &stats, ColorMode::Never, None, None);
    assert!(result.is_err());
}

#[test]
fn format_stats_output_markdown_works() {
    let stats = ProjectStatistics::new(vec![]);
    let result = format_stats_output(OutputFormat::Markdown, &stats, ColorMode::Never, None, None);
    assert!(result.is_ok());
    let output = result.unwrap();
    assert!(output.contains("## SLOC Statistics"));
    assert!(output.contains("| Total Files | 0 |"));
}

#[test]
fn format_stats_output_html_works() {
    let stats = ProjectStatistics::new(vec![]);
    let result = format_stats_output(OutputFormat::Html, &stats, ColorMode::Never, None, None);
    assert!(result.is_ok());
    let output = result.unwrap();
    assert!(output.contains("<!DOCTYPE html>"));
    assert!(output.contains("Total Files"));
}
