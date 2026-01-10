//! Output format tests - json, sarif, markdown, html, file output, report-json.

use crate::common::{BASIC_CONFIG_V2, TestFixture};
use crate::sloc_guard;
use predicates::prelude::*;

// =============================================================================
// Output Format Tests
// =============================================================================

#[test]
fn check_json_output_format() {
    let fixture = TestFixture::new();
    fixture.create_config(BASIC_CONFIG_V2);
    fixture.create_rust_file("src/main.rs", 10);

    sloc_guard!()
        .current_dir(fixture.path())
        .args(["check", "--no-sloc-cache", "--format", "json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"summary\""))
        .stdout(predicate::str::contains("\"results\""));
}

#[test]
fn check_sarif_output_format() {
    let fixture = TestFixture::new();
    fixture.create_config(BASIC_CONFIG_V2);
    fixture.create_rust_file("src/main.rs", 10);

    sloc_guard!()
        .current_dir(fixture.path())
        .args(["check", "--no-sloc-cache", "--format", "sarif"])
        .assert()
        .success()
        .stdout(predicate::str::contains("$schema"))
        .stdout(predicate::str::contains("2.1.0"));
}

#[test]
fn check_markdown_output_format() {
    let fixture = TestFixture::new();
    fixture.create_config(BASIC_CONFIG_V2);
    fixture.create_rust_file("src/main.rs", 10);

    sloc_guard!()
        .current_dir(fixture.path())
        .args(["check", "--no-sloc-cache", "--format", "markdown"])
        .assert()
        .success()
        .stdout(predicate::str::contains("## SLOC Guard Results"));
}

#[test]
fn check_html_output_format() {
    let fixture = TestFixture::new();
    fixture.create_config(BASIC_CONFIG_V2);
    fixture.create_rust_file("src/main.rs", 10);

    sloc_guard!()
        .current_dir(fixture.path())
        .args(["check", "--no-sloc-cache", "--format", "html"])
        .assert()
        .success()
        .stdout(predicate::str::contains("<!DOCTYPE html>"))
        .stdout(predicate::str::contains("SLOC Guard"));
}

#[test]
fn check_output_to_file() {
    let fixture = TestFixture::new();
    fixture.create_config(BASIC_CONFIG_V2);
    fixture.create_rust_file("src/main.rs", 10);

    let output_path = fixture.path().join("report.json");

    sloc_guard!()
        .current_dir(fixture.path())
        .args([
            "check",
            "--no-sloc-cache",
            "--format",
            "json",
            "--output",
            output_path.to_str().unwrap(),
        ])
        .assert()
        .success();

    assert!(output_path.exists());
    let content = std::fs::read_to_string(&output_path).unwrap();
    assert!(content.contains("\"summary\""));
}

// =============================================================================
// Report JSON Tests
// =============================================================================

#[test]
fn check_report_json_creates_stats_file() {
    let fixture = TestFixture::new();
    fixture.create_config(BASIC_CONFIG_V2);
    fixture.create_rust_file("src/main.rs", 30);
    fixture.create_rust_file("src/lib.rs", 20);

    let stats_path = fixture.path().join("stats.json");

    sloc_guard!()
        .current_dir(fixture.path())
        .args([
            "check",
            "--no-sloc-cache",
            "--quiet",
            "--report-json",
            stats_path.to_str().unwrap(),
        ])
        .assert()
        .success();

    assert!(stats_path.exists());
    let content = std::fs::read_to_string(&stats_path).unwrap();
    assert!(content.contains("\"total_files\""));
    assert!(content.contains("\"by_language\""));
}
