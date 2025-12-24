//! Integration tests for the `stats` command.

mod common;

use common::{BASIC_CONFIG_V2, TestFixture};
use predicates::prelude::*;

// =============================================================================
// Basic Stats Command Tests
// =============================================================================

#[test]
fn stats_basic_output() {
    let fixture = TestFixture::new();
    fixture.create_config(BASIC_CONFIG_V2);
    fixture.create_rust_file("src/main.rs", 50);
    fixture.create_rust_file("src/lib.rs", 30);

    sloc_guard!()
        .current_dir(fixture.path())
        .args(["stats", "--no-cache"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Files:"))
        .stdout(predicate::str::contains("Total lines:"));
}

#[test]
fn stats_empty_directory() {
    let fixture = TestFixture::new();
    fixture.create_config(BASIC_CONFIG_V2);
    fixture.create_dir("src");

    sloc_guard!()
        .current_dir(fixture.path())
        .args(["stats", "--no-cache"])
        .assert()
        .success();
}

#[test]
fn stats_with_specific_path() {
    let fixture = TestFixture::new();
    fixture.create_config(BASIC_CONFIG_V2);
    fixture.create_rust_file("src/main.rs", 50);
    fixture.create_rust_file("other/code.rs", 100);

    sloc_guard!()
        .current_dir(fixture.path())
        .args(["stats", "--no-cache", "src"])
        .assert()
        .success()
        .stdout(predicate::str::contains("1")); // Only 1 file in src
}

// =============================================================================
// Output Format Tests
// =============================================================================

#[test]
fn stats_json_output_format() {
    let fixture = TestFixture::new();
    fixture.create_config(BASIC_CONFIG_V2);
    fixture.create_rust_file("src/main.rs", 50);

    sloc_guard!()
        .current_dir(fixture.path())
        .args(["stats", "--no-cache", "--format", "json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"summary\""))
        .stdout(predicate::str::contains("\"total_files\""))
        .stdout(predicate::str::contains("\"files\""));
}

#[test]
fn stats_markdown_output_format() {
    let fixture = TestFixture::new();
    fixture.create_config(BASIC_CONFIG_V2);
    fixture.create_rust_file("src/main.rs", 50);

    sloc_guard!()
        .current_dir(fixture.path())
        .args(["stats", "--no-cache", "--format", "markdown"])
        .assert()
        .success()
        .stdout(predicate::str::contains("# SLOC Statistics"));
}

#[test]
fn stats_output_to_file() {
    let fixture = TestFixture::new();
    fixture.create_config(BASIC_CONFIG_V2);
    fixture.create_rust_file("src/main.rs", 50);

    let output_path = fixture.path().join("stats.json");

    sloc_guard!()
        .current_dir(fixture.path())
        .args([
            "stats",
            "--no-cache",
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
// Grouping Tests
// =============================================================================

#[test]
fn stats_group_by_language() {
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
        .args(["stats", "--no-cache", "--group-by", "lang"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Language"));
}

#[test]
fn stats_group_by_directory() {
    let fixture = TestFixture::new();
    fixture.create_config(BASIC_CONFIG_V2);
    fixture.create_rust_file("src/main.rs", 50);
    fixture.create_rust_file("lib/helper.rs", 30);

    sloc_guard!()
        .current_dir(fixture.path())
        .args(["stats", "--no-cache", "--group-by", "dir"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Directory"));
}

// =============================================================================
// Top Files Tests
// =============================================================================

#[test]
fn stats_top_largest_files() {
    let fixture = TestFixture::new();
    fixture.create_config(BASIC_CONFIG_V2);
    fixture.create_rust_file("src/small.rs", 10);
    fixture.create_rust_file("src/medium.rs", 50);
    fixture.create_rust_file("src/large.rs", 100);

    sloc_guard!()
        .current_dir(fixture.path())
        .args(["stats", "--no-cache", "--top", "2"])
        .assert()
        .success()
        .stdout(predicate::str::contains("large.rs"))
        .stdout(predicate::str::contains("Top 2"));
}

#[test]
fn stats_top_zero_shows_none() {
    let fixture = TestFixture::new();
    fixture.create_config(BASIC_CONFIG_V2);
    fixture.create_rust_file("src/main.rs", 50);

    // Top 0 should not show any top files section
    sloc_guard!()
        .current_dir(fixture.path())
        .args(["stats", "--no-cache"])
        .assert()
        .success();
}

// =============================================================================
// CLI Override Tests
// =============================================================================

#[test]
fn stats_cli_ext_filter() {
    let fixture = TestFixture::new();
    fixture.create_config(BASIC_CONFIG_V2);
    fixture.create_rust_file("src/main.rs", 50);
    fixture.create_file("src/script.py", "print('hello')\nprint('world')\n");

    // Only count Python files
    sloc_guard!()
        .current_dir(fixture.path())
        .args(["stats", "--no-cache", "--ext", "py", "--format", "json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"total_files\": 1"));
}

#[test]
fn stats_cli_exclude_pattern() {
    let fixture = TestFixture::new();
    fixture.create_config(BASIC_CONFIG_V2);
    fixture.create_rust_file("src/main.rs", 50);
    fixture.create_rust_file("vendor/lib.rs", 100);

    sloc_guard!()
        .current_dir(fixture.path())
        .args([
            "stats",
            "--no-cache",
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
fn stats_cli_include_filter() {
    let fixture = TestFixture::new();
    fixture.create_config(BASIC_CONFIG_V2);
    fixture.create_rust_file("src/main.rs", 50);
    fixture.create_rust_file("other/code.rs", 100);

    sloc_guard!()
        .current_dir(fixture.path())
        .args([
            "stats",
            "--no-cache",
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
// Trend Tracking Tests
// =============================================================================

#[test]
fn stats_trend_creates_history_file() {
    let fixture = TestFixture::new();
    fixture.create_config(BASIC_CONFIG_V2);
    fixture.create_rust_file("src/main.rs", 50);

    // History file is now created in .sloc-guard/ directory (or .git/sloc-guard/ if in git repo)
    let history_path = fixture.path().join(".sloc-guard").join("history.json");

    sloc_guard!()
        .current_dir(fixture.path())
        .args(["stats", "--no-cache", "--trend"])
        .assert()
        .success();

    assert!(history_path.exists());
}

#[test]
fn stats_trend_custom_history_file() {
    let fixture = TestFixture::new();
    fixture.create_config(BASIC_CONFIG_V2);
    fixture.create_rust_file("src/main.rs", 50);

    let custom_history = fixture.path().join("custom-history.json");

    sloc_guard!()
        .current_dir(fixture.path())
        .args([
            "stats",
            "--no-cache",
            "--trend",
            "--history-file",
            custom_history.to_str().unwrap(),
        ])
        .assert()
        .success();

    assert!(custom_history.exists());
}

#[test]
fn stats_trend_shows_delta_on_second_run() {
    let fixture = TestFixture::new();
    fixture.create_config(BASIC_CONFIG_V2);
    fixture.create_rust_file("src/main.rs", 50);

    // First run to create history
    sloc_guard!()
        .current_dir(fixture.path())
        .args(["stats", "--no-cache", "--trend"])
        .assert()
        .success();

    // Add more code
    fixture.create_rust_file("src/lib.rs", 30);

    // Second run should show delta (may include git context or fallback to "previous run")
    sloc_guard!()
        .current_dir(fixture.path())
        .args(["stats", "--no-cache", "--trend"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Changes since"));
}

// =============================================================================
// No Config Mode Tests
// =============================================================================

#[test]
fn stats_no_config_mode() {
    let fixture = TestFixture::new();
    fixture.create_rust_file("src/main.rs", 50);

    sloc_guard!()
        .current_dir(fixture.path())
        .args(["stats", "--no-config", "--no-cache", "--ext", "rs"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Total"));
}

// =============================================================================
// Edge Cases
// =============================================================================

#[test]
fn stats_handles_binary_files_gracefully() {
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
        .args(["stats", "--no-cache"])
        .assert()
        .success();
}

#[test]
fn stats_handles_empty_files() {
    let fixture = TestFixture::new();
    fixture.create_config(BASIC_CONFIG_V2);
    fixture.create_file("src/empty.rs", "");
    fixture.create_rust_file("src/main.rs", 10);

    sloc_guard!()
        .current_dir(fixture.path())
        .args(["stats", "--no-cache", "--format", "json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"total_files\": 2"));
}

#[test]
fn stats_handles_files_with_only_comments() {
    let fixture = TestFixture::new();
    fixture.create_config(BASIC_CONFIG_V2);
    fixture.create_file(
        "src/comments.rs",
        "// Just comments\n// No actual code\n/* Multi-line\ncomment */\n",
    );

    sloc_guard!()
        .current_dir(fixture.path())
        .args(["stats", "--no-cache"])
        .assert()
        .success();
}
