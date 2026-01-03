//! Integration tests for the `stats` command.

mod common;

use common::{BASIC_CONFIG_V2, TestFixture};
use predicates::prelude::*;

// =============================================================================
// Summary Subcommand Tests
// =============================================================================

#[test]
fn stats_summary_basic_output() {
    let fixture = TestFixture::new();
    fixture.create_config(BASIC_CONFIG_V2);
    fixture.create_rust_file("src/main.rs", 50);
    fixture.create_rust_file("src/lib.rs", 30);

    sloc_guard!()
        .current_dir(fixture.path())
        .args(["stats", "summary", "--no-sloc-cache"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Files:"))
        .stdout(predicate::str::contains("Total lines:"));
}

#[test]
fn stats_summary_empty_directory() {
    let fixture = TestFixture::new();
    fixture.create_config(BASIC_CONFIG_V2);
    fixture.create_dir("src");

    sloc_guard!()
        .current_dir(fixture.path())
        .args(["stats", "summary", "--no-sloc-cache"])
        .assert()
        .success();
}

#[test]
fn stats_summary_with_specific_path() {
    let fixture = TestFixture::new();
    fixture.create_config(BASIC_CONFIG_V2);
    fixture.create_rust_file("src/main.rs", 50);
    fixture.create_rust_file("other/code.rs", 100);

    sloc_guard!()
        .current_dir(fixture.path())
        .args(["stats", "summary", "--no-sloc-cache", "src"])
        .assert()
        .success()
        .stdout(predicate::str::contains("1")); // Only 1 file in src
}

#[test]
fn stats_summary_json_format() {
    let fixture = TestFixture::new();
    fixture.create_config(BASIC_CONFIG_V2);
    fixture.create_rust_file("src/main.rs", 50);

    sloc_guard!()
        .current_dir(fixture.path())
        .args(["stats", "summary", "--no-sloc-cache", "--format", "json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"summary\""))
        .stdout(predicate::str::contains("\"total_files\""));
}

#[test]
fn stats_summary_markdown_format() {
    let fixture = TestFixture::new();
    fixture.create_config(BASIC_CONFIG_V2);
    fixture.create_rust_file("src/main.rs", 50);

    sloc_guard!()
        .current_dir(fixture.path())
        .args(["stats", "summary", "--no-sloc-cache", "--format", "md"])
        .assert()
        .success()
        .stdout(predicate::str::contains("# SLOC Statistics"));
}

// =============================================================================
// Files Subcommand Tests
// =============================================================================

#[test]
fn stats_files_shows_all_files() {
    let fixture = TestFixture::new();
    fixture.create_config(BASIC_CONFIG_V2);
    fixture.create_rust_file("src/main.rs", 50);
    fixture.create_rust_file("src/lib.rs", 30);

    sloc_guard!()
        .current_dir(fixture.path())
        .args(["stats", "files", "--no-sloc-cache"])
        .assert()
        .success()
        .stdout(predicate::str::contains("main.rs"))
        .stdout(predicate::str::contains("lib.rs"));
}

#[test]
fn stats_files_top_largest_files() {
    let fixture = TestFixture::new();
    fixture.create_config(BASIC_CONFIG_V2);
    fixture.create_rust_file("src/small.rs", 10);
    fixture.create_rust_file("src/medium.rs", 50);
    fixture.create_rust_file("src/large.rs", 100);

    sloc_guard!()
        .current_dir(fixture.path())
        .args(["stats", "files", "--no-sloc-cache", "--top", "2"])
        .assert()
        .success()
        .stdout(predicate::str::contains("large.rs"))
        .stdout(predicate::str::contains("Files (2 total):"));
}

#[test]
fn stats_files_sort_by_code() {
    let fixture = TestFixture::new();
    fixture.create_config(BASIC_CONFIG_V2);
    fixture.create_rust_file("src/small.rs", 10);
    fixture.create_rust_file("src/large.rs", 100);

    sloc_guard!()
        .current_dir(fixture.path())
        .args(["stats", "files", "--no-sloc-cache", "--sort", "code"])
        .assert()
        .success();
}

// =============================================================================
// Breakdown Subcommand Tests
// =============================================================================

#[test]
fn stats_breakdown_by_language() {
    let fixture = TestFixture::new();
    fixture.create_config(
        r#"
version = "2"

[scanner]
gitignore = false

[content]
extensions = ["rs", "py"]
max_lines = 500

[structure]
max_files = 50
max_dirs = 10
"#,
    );
    fixture.create_rust_file("src/main.rs", 50);
    fixture.create_file("src/script.py", "print('hello')\n");

    sloc_guard!()
        .current_dir(fixture.path())
        .args(["stats", "breakdown", "--no-sloc-cache", "--by", "lang"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Language"));
}

#[test]
fn stats_breakdown_by_directory() {
    let fixture = TestFixture::new();
    fixture.create_config(BASIC_CONFIG_V2);
    fixture.create_rust_file("src/main.rs", 50);
    fixture.create_rust_file("lib/helper.rs", 30);

    sloc_guard!()
        .current_dir(fixture.path())
        .args(["stats", "breakdown", "--no-sloc-cache", "--by", "dir"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Directory"));
}

// =============================================================================
// Trend Subcommand Tests
// =============================================================================

#[test]
fn stats_trend_basic() {
    let fixture = TestFixture::new();
    fixture.create_config(BASIC_CONFIG_V2);
    fixture.create_rust_file("src/main.rs", 50);

    // Create some history first
    let history_path = fixture.path().join("history.json");
    std::fs::write(
        &history_path,
        r#"{"version": 1, "entries": [{"timestamp": 1735000000, "total_files": 1, "total_lines": 50, "code": 45, "comment": 3, "blank": 2}]}"#,
    ).unwrap();

    sloc_guard!()
        .current_dir(fixture.path())
        .args([
            "stats",
            "trend",
            "--no-sloc-cache",
            "--history-file",
            history_path.to_str().unwrap(),
        ])
        .assert()
        .success();
}

#[test]
fn stats_trend_with_since() {
    let fixture = TestFixture::new();
    fixture.create_config(BASIC_CONFIG_V2);
    fixture.create_rust_file("src/main.rs", 50);

    let history_path = fixture.path().join("history.json");
    std::fs::write(
        &history_path,
        r#"{"version": 1, "entries": [{"timestamp": 1735000000, "total_files": 1, "total_lines": 50, "code": 45, "comment": 3, "blank": 2}]}"#,
    ).unwrap();

    sloc_guard!()
        .current_dir(fixture.path())
        .args([
            "stats",
            "trend",
            "--no-sloc-cache",
            "--since",
            "7d",
            "--history-file",
            history_path.to_str().unwrap(),
        ])
        .assert()
        .success();
}

#[test]
fn stats_trend_empty_history() {
    let fixture = TestFixture::new();
    fixture.create_config(BASIC_CONFIG_V2);
    fixture.create_rust_file("src/main.rs", 50);

    // No history file exists - should succeed without trend delta
    let history_path = fixture.path().join("nonexistent-history.json");

    sloc_guard!()
        .current_dir(fixture.path())
        .args([
            "stats",
            "trend",
            "--no-sloc-cache",
            "--history-file",
            history_path.to_str().unwrap(),
        ])
        .assert()
        .success()
        // Should still show summary without trend section
        .stdout(predicate::str::contains("Summary"));
}

#[test]
fn stats_trend_json_format() {
    let fixture = TestFixture::new();
    fixture.create_config(BASIC_CONFIG_V2);
    fixture.create_rust_file("src/main.rs", 50);

    let history_path = fixture.path().join("history.json");
    std::fs::write(
        &history_path,
        r#"{"version": 1, "entries": [{"timestamp": 1735000000, "total_files": 1, "total_lines": 40, "code": 35, "comment": 3, "blank": 2, "git_ref": "abc1234", "git_branch": "main"}]}"#,
    ).unwrap();

    sloc_guard!()
        .current_dir(fixture.path())
        .args([
            "stats",
            "trend",
            "--no-sloc-cache",
            "--format",
            "json",
            "--history-file",
            history_path.to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"summary\""))
        .stdout(predicate::str::contains("\"trend\""));
}

#[test]
fn stats_trend_markdown_format() {
    let fixture = TestFixture::new();
    fixture.create_config(BASIC_CONFIG_V2);
    fixture.create_rust_file("src/main.rs", 50);

    let history_path = fixture.path().join("history.json");
    std::fs::write(
        &history_path,
        r#"{"version": 1, "entries": [{"timestamp": 1735000000, "total_files": 1, "total_lines": 40, "code": 35, "comment": 3, "blank": 2}]}"#,
    ).unwrap();

    sloc_guard!()
        .current_dir(fixture.path())
        .args([
            "stats",
            "trend",
            "--no-sloc-cache",
            "--format",
            "md",
            "--history-file",
            history_path.to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("# SLOC Statistics"));
}

#[test]
fn stats_trend_shows_delta_values() {
    let fixture = TestFixture::new();
    fixture.create_config(BASIC_CONFIG_V2);
    // Create file with 50 lines of code
    fixture.create_rust_file("src/main.rs", 50);

    let history_path = fixture.path().join("history.json");
    // Previous entry had 30 code lines, so delta should be ~+20
    std::fs::write(
        &history_path,
        r#"{"version": 1, "entries": [{"timestamp": 1735000000, "total_files": 1, "total_lines": 40, "code": 30, "comment": 5, "blank": 5, "git_ref": "abc1234"}]}"#,
    ).unwrap();

    sloc_guard!()
        .current_dir(fixture.path())
        .args([
            "stats",
            "trend",
            "--no-sloc-cache",
            "--history-file",
            history_path.to_str().unwrap(),
        ])
        .assert()
        .success()
        // Should show trend arrows and delta values
        .stdout(predicate::str::contains("Changes since"))
        .stdout(predicate::str::contains("Code:"));
}

#[test]
fn stats_trend_with_git_context_display() {
    let fixture = TestFixture::new();
    fixture.create_config(BASIC_CONFIG_V2);
    fixture.create_rust_file("src/main.rs", 50);

    let history_path = fixture.path().join("history.json");
    // Include git context in history entry
    std::fs::write(
        &history_path,
        r#"{"version": 1, "entries": [{"timestamp": 1735000000, "total_files": 1, "total_lines": 50, "code": 45, "comment": 3, "blank": 2, "git_ref": "a1b2c3d", "git_branch": "feature-branch"}]}"#,
    ).unwrap();

    sloc_guard!()
        .current_dir(fixture.path())
        .args([
            "stats",
            "trend",
            "--no-sloc-cache",
            "--history-file",
            history_path.to_str().unwrap(),
        ])
        .assert()
        .success()
        // Should display git commit reference in header
        .stdout(predicate::str::contains("a1b2c3d"));
}

#[test]
fn stats_trend_invalid_since_falls_back() {
    let fixture = TestFixture::new();
    fixture.create_config(BASIC_CONFIG_V2);
    fixture.create_rust_file("src/main.rs", 50);

    let history_path = fixture.path().join("history.json");
    std::fs::write(
        &history_path,
        r#"{"version": 1, "entries": [{"timestamp": 1735000000, "total_files": 1, "total_lines": 50, "code": 45, "comment": 3, "blank": 2}]}"#,
    ).unwrap();

    // Invalid duration format should warn but succeed with fallback
    sloc_guard!()
        .current_dir(fixture.path())
        .args([
            "stats",
            "trend",
            "--no-sloc-cache",
            "--since",
            "invalid_duration",
            "--history-file",
            history_path.to_str().unwrap(),
        ])
        .assert()
        .success()
        // Should still produce output (fallback to latest entry comparison)
        .stdout(predicate::str::contains("Summary"));
}

// =============================================================================
// History Subcommand Tests
// =============================================================================

#[test]
fn stats_history_empty() {
    let fixture = TestFixture::new();
    fixture.create_config(BASIC_CONFIG_V2);

    let history_path = fixture.path().join("history.json");

    sloc_guard!()
        .current_dir(fixture.path())
        .args([
            "stats",
            "history",
            "--history-file",
            history_path.to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("No history entries found"));
}

#[test]
fn stats_history_with_entries() {
    let fixture = TestFixture::new();
    fixture.create_config(BASIC_CONFIG_V2);

    let history_path = fixture.path().join("history.json");
    std::fs::write(
        &history_path,
        r#"{"version": 1, "entries": [{"timestamp": 1735048800, "total_files": 100, "total_lines": 5500, "code": 5000, "comment": 300, "blank": 200}]}"#,
    ).unwrap();

    sloc_guard!()
        .current_dir(fixture.path())
        .args([
            "stats",
            "history",
            "--history-file",
            history_path.to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("History"))
        .stdout(predicate::str::contains("Files: 100"));
}

#[test]
fn stats_history_json_format() {
    let fixture = TestFixture::new();
    fixture.create_config(BASIC_CONFIG_V2);

    let history_path = fixture.path().join("history.json");
    std::fs::write(
        &history_path,
        r#"{"version": 1, "entries": [{"timestamp": 1735048800, "total_files": 100, "total_lines": 5500, "code": 5000, "comment": 300, "blank": 200}]}"#,
    ).unwrap();

    sloc_guard!()
        .current_dir(fixture.path())
        .args([
            "stats",
            "history",
            "--format",
            "json",
            "--history-file",
            history_path.to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"count\""))
        .stdout(predicate::str::contains("\"entries\""));
}

#[test]
fn stats_history_limit() {
    let fixture = TestFixture::new();
    fixture.create_config(BASIC_CONFIG_V2);

    let history_path = fixture.path().join("history.json");
    std::fs::write(
        &history_path,
        r#"{"version": 1, "entries": [
            {"timestamp": 1735000000, "total_files": 96, "total_lines": 5100, "code": 4600, "comment": 260, "blank": 180},
            {"timestamp": 1735010000, "total_files": 97, "total_lines": 5200, "code": 4700, "comment": 270, "blank": 190},
            {"timestamp": 1735020000, "total_files": 98, "total_lines": 5300, "code": 4800, "comment": 280, "blank": 195}
        ]}"#,
    ).unwrap();

    sloc_guard!()
        .current_dir(fixture.path())
        .args([
            "stats",
            "history",
            "--limit",
            "2",
            "--history-file",
            history_path.to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("2 of 3 entries"));
}

// =============================================================================
// Report Subcommand Tests
// =============================================================================

#[test]
fn stats_report_to_file() {
    let fixture = TestFixture::new();
    fixture.create_config(BASIC_CONFIG_V2);
    fixture.create_rust_file("src/main.rs", 50);

    let output_path = fixture.path().join("report.json");

    sloc_guard!()
        .current_dir(fixture.path())
        .args([
            "stats",
            "report",
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

#[test]
fn stats_report_html_output() {
    let fixture = TestFixture::new();
    fixture.create_config(BASIC_CONFIG_V2);
    fixture.create_rust_file("src/main.rs", 50);

    let output_path = fixture.path().join("report.html");

    sloc_guard!()
        .current_dir(fixture.path())
        .args([
            "stats",
            "report",
            "--no-sloc-cache",
            "--format",
            "html",
            "--output",
            output_path.to_str().unwrap(),
        ])
        .assert()
        .success();

    assert!(output_path.exists());
    let content = std::fs::read_to_string(&output_path).unwrap();
    assert!(content.contains("<!DOCTYPE html>"));
}

// =============================================================================
// CLI Override Tests
// =============================================================================

#[test]
fn stats_summary_cli_ext_filter() {
    let fixture = TestFixture::new();
    fixture.create_config(BASIC_CONFIG_V2);
    fixture.create_rust_file("src/main.rs", 50);
    fixture.create_file("src/script.py", "print('hello')\nprint('world')\n");

    // Only count Python files
    sloc_guard!()
        .current_dir(fixture.path())
        .args([
            "stats",
            "summary",
            "--no-sloc-cache",
            "--ext",
            "py",
            "--format",
            "json",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"total_files\": 1"));
}

#[test]
fn stats_summary_cli_exclude_pattern() {
    let fixture = TestFixture::new();
    fixture.create_config(BASIC_CONFIG_V2);
    fixture.create_rust_file("src/main.rs", 50);
    fixture.create_rust_file("vendor/lib.rs", 100);

    sloc_guard!()
        .current_dir(fixture.path())
        .args([
            "stats",
            "summary",
            "--no-sloc-cache",
            "--exclude",
            "**/vendor/**",
            "--format",
            "json",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"total_files\": 1"));
}

#[test]
fn stats_summary_cli_include_filter() {
    let fixture = TestFixture::new();
    fixture.create_config(BASIC_CONFIG_V2);
    fixture.create_rust_file("src/main.rs", 50);
    fixture.create_rust_file("other/code.rs", 100);

    sloc_guard!()
        .current_dir(fixture.path())
        .args([
            "stats",
            "summary",
            "--no-sloc-cache",
            "--include",
            "src",
            "--format",
            "json",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"total_files\": 1"));
}

// =============================================================================
// No Config Mode Tests
// =============================================================================

#[test]
fn stats_summary_no_config_mode() {
    let fixture = TestFixture::new();
    fixture.create_rust_file("src/main.rs", 50);

    sloc_guard!()
        .current_dir(fixture.path())
        .args([
            "stats",
            "summary",
            "--no-config",
            "--no-sloc-cache",
            "--ext",
            "rs",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Total"));
}

// =============================================================================
// Edge Cases
// =============================================================================

#[test]
fn stats_summary_handles_binary_files_gracefully() {
    let fixture = TestFixture::new();
    fixture.create_config(BASIC_CONFIG_V2);
    fixture.create_rust_file("src/main.rs", 50);
    // Create a binary-like file (not matching extensions, so ignored)
    fixture.create_file(
        "src/data.bin",
        &[0u8, 1, 2, 3, 255]
            .map(|b| b as char)
            .iter()
            .collect::<String>(),
    );

    sloc_guard!()
        .current_dir(fixture.path())
        .args(["stats", "summary", "--no-sloc-cache"])
        .assert()
        .success();
}

#[test]
fn stats_summary_handles_empty_files() {
    let fixture = TestFixture::new();
    fixture.create_config(BASIC_CONFIG_V2);
    fixture.create_file("src/empty.rs", "");
    fixture.create_rust_file("src/main.rs", 10);

    sloc_guard!()
        .current_dir(fixture.path())
        .args(["stats", "summary", "--no-sloc-cache", "--format", "json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"total_files\": 2"));
}

#[test]
fn stats_summary_handles_files_with_only_comments() {
    let fixture = TestFixture::new();
    fixture.create_config(BASIC_CONFIG_V2);
    fixture.create_file(
        "src/comments.rs",
        "// Just comments\n// No actual code\n/* Multi-line\ncomment */\n",
    );

    sloc_guard!()
        .current_dir(fixture.path())
        .args(["stats", "summary", "--no-sloc-cache"])
        .assert()
        .success();
}

// =============================================================================
// Error Cases
// =============================================================================

#[test]
fn stats_bare_command_requires_subcommand() {
    let fixture = TestFixture::new();
    fixture.create_config(BASIC_CONFIG_V2);

    sloc_guard!()
        .current_dir(fixture.path())
        .args(["stats"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("subcommand"));
}
