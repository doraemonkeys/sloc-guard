//! Core check command tests - basic behavior, CLI overrides, verbosity, error handling.

use crate::common::{BASIC_CONFIG_V2, STRICT_CONFIG_V2, TestFixture};
use crate::sloc_guard;
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
