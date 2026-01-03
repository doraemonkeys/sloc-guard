//! Integration tests for the `check` command.

mod common;

use common::{BASIC_CONFIG_V2, CONFIG_WITH_RULES, STRICT_CONFIG_V2, TestFixture};
use predicates::prelude::*;

// =============================================================================
// Basic Check Command Tests
// =============================================================================

#[test]
fn check_passes_with_small_files() {
    let fixture = TestFixture::new();
    fixture.create_config(BASIC_CONFIG_V2);
    fixture.create_rust_file("src/main.rs", 10);
    fixture.create_rust_file("src/lib.rs", 20);

    sloc_guard!()
        .current_dir(fixture.path())
        .args(["check", "--no-sloc-cache", "--quiet"])
        .assert()
        .success();
}

#[test]
fn check_fails_when_file_exceeds_limit() {
    let fixture = TestFixture::new();
    fixture.create_config(STRICT_CONFIG_V2);
    fixture.create_rust_file("src/large.rs", 50);

    sloc_guard!()
        .current_dir(fixture.path())
        .args(["check", "--no-sloc-cache", "--quiet"])
        .assert()
        .code(1);
}

#[test]
fn check_warns_when_near_threshold() {
    let fixture = TestFixture::new();
    fixture.create_config(BASIC_CONFIG_V2);
    // 85 lines is > 80% of 100 limit (warn_threshold = 0.8)
    fixture.create_rust_file("src/warning.rs", 85);

    sloc_guard!()
        .current_dir(fixture.path())
        .args(["check", "--no-sloc-cache"])
        .assert()
        .success()
        .stdout(predicate::str::contains("WARNING"));
}

#[test]
fn check_strict_mode_fails_on_warnings() {
    let fixture = TestFixture::new();
    fixture.create_config(BASIC_CONFIG_V2);
    // 85 lines is > 80% of 100 limit, triggers warning
    fixture.create_rust_file("src/warning.rs", 85);

    sloc_guard!()
        .current_dir(fixture.path())
        .args(["check", "--no-sloc-cache", "--strict", "--quiet"])
        .assert()
        .code(1);
}

#[test]
fn check_warn_only_mode_always_succeeds() {
    let fixture = TestFixture::new();
    fixture.create_config(STRICT_CONFIG_V2);
    fixture.create_rust_file("src/large.rs", 100);

    sloc_guard!()
        .current_dir(fixture.path())
        .args(["check", "--no-sloc-cache", "--warn-only", "--quiet"])
        .assert()
        .success();
}

// =============================================================================
// CLI Override Tests
// =============================================================================

#[test]
fn check_cli_max_lines_override() {
    let fixture = TestFixture::new();
    fixture.create_config(BASIC_CONFIG_V2);
    fixture.create_rust_file("src/code.rs", 50);

    // Default config allows 100 lines, but CLI sets 30
    sloc_guard!()
        .current_dir(fixture.path())
        .args(["check", "--no-sloc-cache", "--quiet", "--max-lines", "30"])
        .assert()
        .code(1);

    // Now set higher limit - should pass
    sloc_guard!()
        .current_dir(fixture.path())
        .args(["check", "--no-sloc-cache", "--quiet", "--max-lines", "100"])
        .assert()
        .success();
}

#[test]
fn check_cli_ext_override() {
    let fixture = TestFixture::new();
    fixture.create_config(BASIC_CONFIG_V2);
    fixture.create_rust_file("src/code.rs", 200); // Exceeds limit
    fixture.create_file("src/code.py", "print('hello')\n");

    // Only check .py files (not .rs), should pass
    sloc_guard!()
        .current_dir(fixture.path())
        .args(["check", "--no-sloc-cache", "--quiet", "--ext", "py"])
        .assert()
        .success();

    // Check .rs files, should fail
    sloc_guard!()
        .current_dir(fixture.path())
        .args(["check", "--no-sloc-cache", "--quiet", "--ext", "rs"])
        .assert()
        .code(1);
}

#[test]
fn check_cli_exclude_pattern() {
    let fixture = TestFixture::new();
    fixture.create_config(BASIC_CONFIG_V2);
    fixture.create_rust_file("src/code.rs", 10);
    fixture.create_rust_file("vendor/large.rs", 200);

    // Exclude vendor with full glob pattern, should pass
    sloc_guard!()
        .current_dir(fixture.path())
        .args([
            "check",
            "--no-sloc-cache",
            "--quiet",
            "--exclude",
            "**/vendor/**",
        ])
        .assert()
        .success();

    // Without exclude, should fail
    sloc_guard!()
        .current_dir(fixture.path())
        .args(["check", "--no-sloc-cache", "--quiet"])
        .assert()
        .code(1);
}

#[test]
fn check_cli_include_filter() {
    let fixture = TestFixture::new();
    fixture.create_config(BASIC_CONFIG_V2);
    fixture.create_rust_file("src/code.rs", 10);
    fixture.create_rust_file("other/large.rs", 200);

    // Only include src, should pass
    sloc_guard!()
        .current_dir(fixture.path())
        .args(["check", "--no-sloc-cache", "--quiet", "--include", "src"])
        .assert()
        .success();
}

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
// Content Rules Tests
// =============================================================================

#[test]
fn check_content_rules_apply_pattern_limits() {
    let fixture = TestFixture::new();
    fixture.create_config(CONFIG_WITH_RULES);
    // Test file uses higher limit (500 lines) - should pass
    fixture.create_rust_file("tests/test_main.rs", 150);

    // Only check tests directory which has higher limit
    sloc_guard!()
        .current_dir(fixture.path())
        .args(["check", "--no-sloc-cache", "--quiet", "--include", "tests"])
        .assert()
        .success();

    // Test file exceeding test rule limit should fail
    fixture.create_rust_file("tests/test_large.rs", 600);

    sloc_guard!()
        .current_dir(fixture.path())
        .args(["check", "--no-sloc-cache", "--quiet", "--include", "tests"])
        .assert()
        .code(1);
}

// =============================================================================
// Structure Check Tests
// =============================================================================

#[test]
fn check_structure_max_files_violation() {
    let fixture = TestFixture::new();
    fixture.create_config(STRICT_CONFIG_V2); // max_files = 2
    fixture.create_rust_file("src/file1.rs", 5);
    fixture.create_rust_file("src/file2.rs", 5);
    fixture.create_rust_file("src/file3.rs", 5); // Exceeds limit

    sloc_guard!()
        .current_dir(fixture.path())
        .args(["check", "src", "--no-sloc-cache", "--quiet"])
        .assert()
        .code(1);
}

#[test]
fn check_structure_max_dirs_violation() {
    let fixture = TestFixture::new();
    fixture.create_config(STRICT_CONFIG_V2); // max_dirs = 1
    fixture.create_dir("src/sub1");
    fixture.create_dir("src/sub2"); // Exceeds limit
    fixture.create_rust_file("src/main.rs", 5);

    sloc_guard!()
        .current_dir(fixture.path())
        .args(["check", "src", "--no-sloc-cache", "--quiet"])
        .assert()
        .code(1);
}

#[test]
fn check_structure_cli_override() {
    let fixture = TestFixture::new();
    fixture.create_config(BASIC_CONFIG_V2);
    // Create 15 files (exceeds default max_files=10)
    for i in 0..15 {
        fixture.create_rust_file(&format!("src/file{i}.rs"), 5);
    }

    // Without override, should fail
    sloc_guard!()
        .current_dir(fixture.path())
        .args(["check", "src", "--no-sloc-cache", "--quiet"])
        .assert()
        .code(1);

    // With CLI override to allow more files
    sloc_guard!()
        .current_dir(fixture.path())
        .args([
            "check",
            "src",
            "--no-sloc-cache",
            "--quiet",
            "--max-files",
            "20",
        ])
        .assert()
        .success();
}

#[test]
fn check_structure_allowlist_violation() {
    let fixture = TestFixture::new();
    // Config with allowlist rule: only .rs files allowed in src
    fixture.create_config(
        r#"
version = "2"

[scanner]
gitignore = false
exclude = []

[content]
max_lines = 100
extensions = ["rs"]

[[structure.rules]]
scope = "**/src"
allow_extensions = [".rs"]
"#,
    );
    fixture.create_rust_file("src/main.rs", 5);
    // Create a disallowed file
    fixture.create_file("src/config.json", "{}");

    sloc_guard!()
        .current_dir(fixture.path())
        .args(["check", "src", "--no-sloc-cache"])
        .assert()
        .code(1)
        .stdout(predicate::str::contains("disallowed file"));
}

#[test]
fn check_structure_global_allowlist_violation() {
    let fixture = TestFixture::new();
    // Global allowlist: only .rs files allowed anywhere
    fixture.create_config(
        r#"
version = "2"

[scanner]
gitignore = false
exclude = []

[content]
max_lines = 100
extensions = ["rs"]

[structure]
allow_extensions = [".rs"]
"#,
    );
    fixture.create_rust_file("src/main.rs", 5);
    fixture.create_file("src/config.json", "{}");

    sloc_guard!()
        .current_dir(fixture.path())
        .args(["check", "src", "--no-sloc-cache"])
        .assert()
        .code(1)
        .stdout(predicate::str::contains("disallowed file"));
}

#[test]
fn check_structure_global_deny_extension_violation() {
    let fixture = TestFixture::new();
    // Global denylist: deny *.json anywhere
    fixture.create_config(
        r#"
version = "2"

[scanner]
gitignore = false
exclude = []

[content]
max_lines = 100
extensions = ["rs"]

[structure]
deny_extensions = [".json"]
"#,
    );
    fixture.create_rust_file("src/main.rs", 5);
    fixture.create_file("src/config.json", "{}");

    sloc_guard!()
        .current_dir(fixture.path())
        .args(["check", "src", "--no-sloc-cache"])
        .assert()
        .code(1)
        .stdout(predicate::str::contains("denied file"));
}

// =============================================================================
// Comment/Blank Line Counting Tests
// =============================================================================

#[test]
fn check_skip_comments_by_default() {
    let fixture = TestFixture::new();
    fixture.create_config(BASIC_CONFIG_V2); // max_lines = 100, skip_comments = true
    // 50 code + 100 comments = 150 total, but only 50 count
    fixture.create_rust_file_with_comments("src/commented.rs", 50, 100, 0);

    sloc_guard!()
        .current_dir(fixture.path())
        .args(["check", "--no-sloc-cache", "--quiet"])
        .assert()
        .success();
}

#[test]
fn check_count_comments_flag() {
    let fixture = TestFixture::new();
    fixture.create_config(BASIC_CONFIG_V2); // max_lines = 100
    // 50 code + 60 comments = 110 when counting comments
    fixture.create_rust_file_with_comments("src/commented.rs", 50, 60, 0);

    // Without --count-comments: passes (only 50 code lines)
    sloc_guard!()
        .current_dir(fixture.path())
        .args(["check", "--no-sloc-cache", "--quiet"])
        .assert()
        .success();

    // With --count-comments: fails (110 lines)
    sloc_guard!()
        .current_dir(fixture.path())
        .args(["check", "--no-sloc-cache", "--quiet", "--count-comments"])
        .assert()
        .code(1);
}

#[test]
fn check_count_blank_flag() {
    let fixture = TestFixture::new();
    fixture.create_config(BASIC_CONFIG_V2); // max_lines = 100
    // 50 code + 60 blanks = 110 when counting blanks
    fixture.create_rust_file_with_comments("src/spacey.rs", 50, 0, 60);

    // Without --count-blank: passes (only 50 code lines)
    sloc_guard!()
        .current_dir(fixture.path())
        .args(["check", "--no-sloc-cache", "--quiet"])
        .assert()
        .success();

    // With --count-blank: fails (110 lines)
    sloc_guard!()
        .current_dir(fixture.path())
        .args(["check", "--no-sloc-cache", "--quiet", "--count-blank"])
        .assert()
        .code(1);
}

// =============================================================================
// Baseline Tests
// =============================================================================

#[test]
fn check_update_baseline_creates_file() {
    let fixture = TestFixture::new();
    fixture.create_config(STRICT_CONFIG_V2);
    fixture.create_rust_file("src/large.rs", 50);

    let baseline_path = fixture.path().join(".sloc-guard-baseline.json");

    sloc_guard!()
        .current_dir(fixture.path())
        .args([
            "check",
            "--no-sloc-cache",
            "--quiet",
            "--baseline",
            baseline_path.to_str().unwrap(),
            "--update-baseline",
        ])
        .assert()
        .code(1); // Still fails, but baseline is created

    assert!(baseline_path.exists());
    let content = std::fs::read_to_string(&baseline_path).unwrap();
    assert!(content.contains("\"version\""));
}

#[test]
fn check_with_baseline_grandfathers_violations() {
    let fixture = TestFixture::new();
    fixture.create_config(STRICT_CONFIG_V2);
    fixture.create_rust_file("src/large.rs", 50);

    let baseline_path = fixture.path().join(".sloc-guard-baseline.json");

    // First run: create baseline
    sloc_guard!()
        .current_dir(fixture.path())
        .args([
            "check",
            "--no-sloc-cache",
            "--quiet",
            "--baseline",
            baseline_path.to_str().unwrap(),
            "--update-baseline",
        ])
        .assert()
        .code(1);

    // Second run with baseline: should pass (grandfathered)
    sloc_guard!()
        .current_dir(fixture.path())
        .args([
            "check",
            "--no-sloc-cache",
            "--quiet",
            "--baseline",
            baseline_path.to_str().unwrap(),
        ])
        .assert()
        .success();
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

// =============================================================================
// No Config Mode Tests
// =============================================================================

#[test]
fn check_no_config_uses_defaults() {
    let fixture = TestFixture::new();
    fixture.create_rust_file("src/small.rs", 10);

    sloc_guard!()
        .current_dir(fixture.path())
        .args(["check", "--no-config", "--no-sloc-cache", "--quiet"])
        .assert()
        .success();
}

#[test]
fn check_no_config_with_cli_args() {
    // Test no-config mode using the project's own source code
    // This ensures reliability across different test environments (including tarpaulin)
    sloc_guard!()
        .args([
            "check",
            "src",
            "--no-config",
            "--no-sloc-cache",
            "--quiet",
            "--max-lines",
            "1", // Very low limit to guarantee failure
            "--ext",
            "rs",
        ])
        .assert()
        .code(1);
}

// =============================================================================
// Verbose and Quiet Mode Tests
// =============================================================================

#[test]
fn check_quiet_mode_suppresses_output() {
    let fixture = TestFixture::new();
    fixture.create_config(BASIC_CONFIG_V2);
    fixture.create_rust_file("src/main.rs", 10);

    let output = sloc_guard!()
        .current_dir(fixture.path())
        .args(["check", "--no-sloc-cache", "--quiet"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    // Quiet mode should have minimal output
    assert!(output.is_empty() || output.len() < 50);
}

#[test]
fn check_verbose_mode_shows_details() {
    let fixture = TestFixture::new();
    fixture.create_config(BASIC_CONFIG_V2);
    fixture.create_rust_file("src/main.rs", 10);

    sloc_guard!()
        .current_dir(fixture.path())
        .args(["check", "--no-sloc-cache", "-v"])
        .assert()
        .success()
        .stdout(predicate::str::contains("PASS"));
}

// =============================================================================
// Error Handling Tests
// =============================================================================

#[test]
fn check_invalid_config_returns_error() {
    let fixture = TestFixture::new();
    fixture.create_file(".sloc-guard.toml", "invalid [[[ toml syntax");
    fixture.create_rust_file("src/main.rs", 10);

    sloc_guard!()
        .current_dir(fixture.path())
        .args(["check", "--no-sloc-cache"])
        .assert()
        .code(2)
        // Syntax errors show line/column and "Config" error type
        .stderr(predicate::str::contains("Config"))
        .stderr(predicate::str::contains("line"));
}

#[test]
fn check_nonexistent_path_handles_gracefully() {
    let fixture = TestFixture::new();
    fixture.create_config(BASIC_CONFIG_V2);

    sloc_guard!()
        .current_dir(fixture.path())
        .args(["check", "--no-sloc-cache", "nonexistent_directory"])
        .assert()
        .success(); // Empty directory scan should pass
}

// =============================================================================
// Color Output Tests
// =============================================================================

#[test]
fn check_color_never_disables_colors() {
    let fixture = TestFixture::new();
    fixture.create_config(BASIC_CONFIG_V2);
    fixture.create_rust_file("src/main.rs", 10);

    let output = sloc_guard!()
        .current_dir(fixture.path())
        .args(["check", "--no-sloc-cache", "--color", "never"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    // Should not contain ANSI escape codes
    let output_str = String::from_utf8_lossy(&output);
    assert!(!output_str.contains("\x1b["));
}

// =============================================================================
// Auto-Snapshot Tests
// =============================================================================

#[test]
fn check_auto_snapshot_creates_history_file() {
    let fixture = TestFixture::new();
    fixture.create_config(
        r#"
version = "2"

[scanner]
gitignore = false
exclude = []

[content]
extensions = ["rs"]
max_lines = 100

[trend]
auto_snapshot_on_check = true
"#,
    );
    fixture.create_rust_file("src/main.rs", 10);

    // Create .sloc-guard directory for history (non-git repo)
    fixture.create_dir(".sloc-guard");

    sloc_guard!()
        .current_dir(fixture.path())
        .args(["check", "--no-sloc-cache"])
        .assert()
        .success()
        .stderr(predicate::str::contains("Auto-snapshot recorded"));

    // Verify history file was created
    let history_path = fixture.path().join(".sloc-guard/history.json");
    assert!(history_path.exists());

    let content = std::fs::read_to_string(&history_path).unwrap();
    assert!(content.contains("\"total_files\""));
    assert!(content.contains("\"code\""));
}

#[test]
fn check_auto_snapshot_disabled_by_default() {
    let fixture = TestFixture::new();
    fixture.create_config(BASIC_CONFIG_V2);
    fixture.create_rust_file("src/main.rs", 10);

    // Create .sloc-guard directory
    fixture.create_dir(".sloc-guard");

    sloc_guard!()
        .current_dir(fixture.path())
        .args(["check", "--no-sloc-cache"])
        .assert()
        .success()
        .stderr(predicate::str::contains("Auto-snapshot").not());

    // History file should NOT be created
    let history_path = fixture.path().join(".sloc-guard/history.json");
    assert!(!history_path.exists());
}

#[test]
fn check_auto_snapshot_respects_min_interval() {
    let fixture = TestFixture::new();
    // Set a very high min_interval_secs to ensure second snapshot is skipped
    fixture.create_config(
        r#"
version = "2"

[scanner]
gitignore = false
exclude = []

[content]
extensions = ["rs"]
max_lines = 100

[trend]
auto_snapshot_on_check = true
min_interval_secs = 3600
"#,
    );
    fixture.create_rust_file("src/main.rs", 10);

    // Create .sloc-guard directory
    fixture.create_dir(".sloc-guard");

    // First run - should create snapshot
    sloc_guard!()
        .current_dir(fixture.path())
        .args(["check", "--no-sloc-cache"])
        .assert()
        .success()
        .stderr(predicate::str::contains("Auto-snapshot recorded"));

    // Second run - should skip due to min_interval_secs (with verbose flag)
    sloc_guard!()
        .current_dir(fixture.path())
        .args(["check", "--no-sloc-cache", "-v"])
        .assert()
        .success()
        .stderr(predicate::str::contains("Skipping auto-snapshot"));
}
