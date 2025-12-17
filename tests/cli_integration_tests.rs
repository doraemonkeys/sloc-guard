#![allow(deprecated)] // cargo_bin deprecation - still works fine

use assert_cmd::Command;
use predicates::prelude::*;
use std::fmt::Write;
use std::fs;
use tempfile::TempDir;

fn cmd() -> Command {
    Command::cargo_bin("sloc-guard").expect("binary should exist")
}

fn generate_lines(count: usize, pattern: &str) -> String {
    let mut s = String::new();
    for i in 0..count {
        let _ = writeln!(s, "{pattern}{i} = {i};");
    }
    s
}

fn generate_py_lines(count: usize) -> String {
    let mut s = String::new();
    for i in 0..count {
        let _ = writeln!(s, "x{i} = {i}");
    }
    s
}

// Note: TextFormatter only shows details for FAILED/WARNING files.
// PASSED files are only counted in the summary.

// ============================================================================
// Check Command Integration Tests
// ============================================================================

#[test]
fn check_empty_directory_exits_success() {
    let temp_dir = TempDir::new().unwrap();

    cmd()
        .arg("check")
        .arg(temp_dir.path())
        .arg("--no-config")
        .assert()
        .success()
        .stdout(predicate::str::contains("Summary"));
}

#[test]
fn check_small_rust_file_passes() {
    let temp_dir = TempDir::new().unwrap();
    let rust_file = temp_dir.path().join("small.rs");
    fs::write(&rust_file, "fn main() {\n    println!(\"Hello\");\n}\n").unwrap();

    // Only FAILED/WARNING files show details; PASSED files only appear in summary count
    cmd()
        .arg("check")
        .arg(temp_dir.path())
        .arg("--no-config")
        .assert()
        .success()
        .stdout(predicate::str::contains("1 passed"));
}

#[test]
fn check_large_file_fails() {
    let temp_dir = TempDir::new().unwrap();
    let rust_file = temp_dir.path().join("large.rs");

    // Create a file with 600 lines (exceeds default 500 limit)
    let content = generate_lines(600, "let x");
    fs::write(&rust_file, content).unwrap();

    cmd()
        .arg("check")
        .arg(temp_dir.path())
        .arg("--no-config")
        .assert()
        .code(1) // EXIT_THRESHOLD_EXCEEDED
        .stdout(predicate::str::contains("FAILED"))
        .stdout(predicate::str::contains("large.rs"));
}

#[test]
fn check_warn_only_converts_failure_to_success() {
    let temp_dir = TempDir::new().unwrap();
    let rust_file = temp_dir.path().join("large.rs");

    let content = generate_lines(600, "let x");
    fs::write(&rust_file, content).unwrap();

    cmd()
        .arg("check")
        .arg(temp_dir.path())
        .arg("--no-config")
        .arg("--warn-only")
        .assert()
        .success()
        .stdout(predicate::str::contains("FAILED"));
}

#[test]
fn check_max_lines_override() {
    let temp_dir = TempDir::new().unwrap();
    let rust_file = temp_dir.path().join("file.rs");

    // 100 lines
    let content = generate_lines(100, "let x");
    fs::write(&rust_file, content).unwrap();

    // Default 500 should pass
    cmd()
        .arg("check")
        .arg(temp_dir.path())
        .arg("--no-config")
        .assert()
        .success();

    // Override to 50 should fail
    cmd()
        .arg("check")
        .arg(temp_dir.path())
        .arg("--no-config")
        .arg("--max-lines")
        .arg("50")
        .assert()
        .code(1);
}

#[test]
fn check_extension_filter() {
    let temp_dir = TempDir::new().unwrap();

    // Create both .rs and .txt files
    fs::write(temp_dir.path().join("code.rs"), "fn main() {}\n").unwrap();
    fs::write(temp_dir.path().join("readme.txt"), "This is a readme\n").unwrap();

    // Only check .rs files - should show 1 file checked
    cmd()
        .arg("check")
        .arg(temp_dir.path())
        .arg("--no-config")
        .arg("--ext")
        .arg("rs")
        .assert()
        .success()
        .stdout(predicate::str::contains("1 files checked"));
}

#[test]
fn check_exclude_pattern() {
    let temp_dir = TempDir::new().unwrap();
    let vendor_dir = temp_dir.path().join("vendor");
    fs::create_dir(&vendor_dir).unwrap();

    fs::write(temp_dir.path().join("main.rs"), "fn main() {}\n").unwrap();
    fs::write(vendor_dir.join("lib.rs"), "fn lib() {}\n").unwrap();

    // Exclude vendor - should only check 1 file (main.rs)
    cmd()
        .arg("check")
        .arg(temp_dir.path())
        .arg("--no-config")
        .arg("-x")
        .arg("**/vendor/**")
        .assert()
        .success()
        .stdout(predicate::str::contains("1 files checked"));
}

#[test]
fn check_json_output_format() {
    let temp_dir = TempDir::new().unwrap();
    fs::write(temp_dir.path().join("main.rs"), "fn main() {}\n").unwrap();

    cmd()
        .arg("check")
        .arg(temp_dir.path())
        .arg("--no-config")
        .arg("--format")
        .arg("json")
        .assert()
        .success()
        .stdout(predicate::str::contains("\"results\""))
        .stdout(predicate::str::contains("\"summary\""));
}

#[test]
fn check_output_to_file() {
    let temp_dir = TempDir::new().unwrap();
    fs::write(temp_dir.path().join("main.rs"), "fn main() {}\n").unwrap();

    let output_file = temp_dir.path().join("report.txt");

    cmd()
        .arg("check")
        .arg(temp_dir.path())
        .arg("--no-config")
        .arg("-o")
        .arg(&output_file)
        .assert()
        .success();

    assert!(output_file.exists());
    let content = fs::read_to_string(&output_file).unwrap();
    assert!(content.contains("Summary"));
}

#[test]
fn check_no_skip_comments_counts_comments() {
    let temp_dir = TempDir::new().unwrap();
    let rust_file = temp_dir.path().join("commented.rs");

    // Create file with comments that would push it over limit
    let mut content = String::new();
    for i in 0..30 {
        let _ = writeln!(content, "let x{i} = {i};");
    }
    for _ in 0..30 {
        content.push_str("// This is a comment\n");
    }
    fs::write(&rust_file, content).unwrap();

    // With skip_comments (default), only code lines count -> 30 lines, passes at limit 50
    cmd()
        .arg("check")
        .arg(temp_dir.path())
        .arg("--no-config")
        .arg("--max-lines")
        .arg("50")
        .assert()
        .success();

    // Without skip_comments, all 60 lines count -> fails at limit 50
    cmd()
        .arg("check")
        .arg(temp_dir.path())
        .arg("--no-config")
        .arg("--max-lines")
        .arg("50")
        .arg("--no-skip-comments")
        .assert()
        .code(1);
}

#[test]
fn check_no_skip_blank_counts_blanks() {
    let temp_dir = TempDir::new().unwrap();
    let rust_file = temp_dir.path().join("blanks.rs");

    // Create file with blank lines
    let mut content = String::new();
    for i in 0..30 {
        let _ = writeln!(content, "let x{i} = {i};");
        content.push('\n'); // blank line
    }
    fs::write(&rust_file, content).unwrap();

    // With skip_blank (default), only code lines count -> 30 lines
    cmd()
        .arg("check")
        .arg(temp_dir.path())
        .arg("--no-config")
        .arg("--max-lines")
        .arg("50")
        .assert()
        .success();

    // Without skip_blank, all 60 lines count -> fails
    cmd()
        .arg("check")
        .arg(temp_dir.path())
        .arg("--no-config")
        .arg("--max-lines")
        .arg("50")
        .arg("--no-skip-blank")
        .assert()
        .code(1);
}

#[test]
fn check_warning_threshold() {
    let temp_dir = TempDir::new().unwrap();
    let rust_file = temp_dir.path().join("nearmax.rs");

    // 95 lines with max 100 -> 95% used, should trigger warning at 0.9 threshold
    let content = generate_lines(95, "let x");
    fs::write(&rust_file, content).unwrap();

    cmd()
        .arg("check")
        .arg(temp_dir.path())
        .arg("--no-config")
        .arg("--max-lines")
        .arg("100")
        .arg("--warn-threshold")
        .arg("0.9")
        .assert()
        .success()
        .stdout(predicate::str::contains("WARNING"));
}

#[test]
fn check_quiet_mode_suppresses_output() {
    let temp_dir = TempDir::new().unwrap();
    fs::write(temp_dir.path().join("main.rs"), "fn main() {}\n").unwrap();

    cmd()
        .arg("--quiet")
        .arg("check")
        .arg(temp_dir.path())
        .arg("--no-config")
        .assert()
        .success()
        .stdout(predicate::str::is_empty());
}

#[test]
fn check_color_never_no_ansi_codes() {
    let temp_dir = TempDir::new().unwrap();
    fs::write(temp_dir.path().join("main.rs"), "fn main() {}\n").unwrap();

    let output = cmd()
        .arg("--color")
        .arg("never")
        .arg("check")
        .arg(temp_dir.path())
        .arg("--no-config")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let output_str = String::from_utf8_lossy(&output);
    assert!(!output_str.contains("\x1b["));
}

// ============================================================================
// Stats Command Integration Tests
// ============================================================================

#[test]
fn stats_empty_directory() {
    let temp_dir = TempDir::new().unwrap();

    cmd()
        .arg("stats")
        .arg(temp_dir.path())
        .arg("--no-config")
        .assert()
        .success()
        .stdout(predicate::str::contains("Summary"))
        .stdout(predicate::str::contains("Files: 0"));
}

#[test]
fn stats_counts_lines() {
    let temp_dir = TempDir::new().unwrap();
    fs::write(
        temp_dir.path().join("main.rs"),
        "fn main() {\n    // comment\n    println!(\"Hi\");\n}\n",
    )
    .unwrap();

    cmd()
        .arg("stats")
        .arg(temp_dir.path())
        .arg("--no-config")
        .assert()
        .success()
        .stdout(predicate::str::contains("main.rs"))
        .stdout(predicate::str::contains("Files: 1"));
}

#[test]
fn stats_json_format() {
    let temp_dir = TempDir::new().unwrap();
    fs::write(temp_dir.path().join("main.rs"), "fn main() {}\n").unwrap();

    cmd()
        .arg("stats")
        .arg(temp_dir.path())
        .arg("--no-config")
        .arg("--format")
        .arg("json")
        .assert()
        .success()
        .stdout(predicate::str::contains("\"files\""))
        .stdout(predicate::str::contains("\"summary\""));
}

#[test]
fn stats_output_to_file() {
    let temp_dir = TempDir::new().unwrap();
    fs::write(temp_dir.path().join("main.rs"), "fn main() {}\n").unwrap();

    let output_file = temp_dir.path().join("stats.json");

    cmd()
        .arg("stats")
        .arg(temp_dir.path())
        .arg("--no-config")
        .arg("--format")
        .arg("json")
        .arg("-o")
        .arg(&output_file)
        .assert()
        .success();

    assert!(output_file.exists());
    let content = fs::read_to_string(&output_file).unwrap();
    assert!(content.contains("\"summary\""));
}

#[test]
fn stats_extension_filter() {
    let temp_dir = TempDir::new().unwrap();
    fs::write(temp_dir.path().join("main.rs"), "fn main() {}\n").unwrap();
    fs::write(temp_dir.path().join("app.py"), "print('hello')\n").unwrap();

    // Only count .py files
    cmd()
        .arg("stats")
        .arg(temp_dir.path())
        .arg("--no-config")
        .arg("--ext")
        .arg("py")
        .assert()
        .success()
        .stdout(predicate::str::contains("app.py"))
        .stdout(predicate::str::contains("main.rs").not());
}

// ============================================================================
// Init Command Integration Tests
// ============================================================================

#[test]
fn init_creates_config_file() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join(".sloc-guard.toml");

    cmd()
        .arg("init")
        .arg("-o")
        .arg(&config_path)
        .assert()
        .success()
        .stdout(predicate::str::contains("Created configuration file"));

    assert!(config_path.exists());
    let content = fs::read_to_string(&config_path).unwrap();
    assert!(content.contains("[default]"));
    assert!(content.contains("max_lines = 500"));
}

#[test]
fn init_fails_if_exists() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join(".sloc-guard.toml");

    fs::write(&config_path, "existing").unwrap();

    cmd()
        .arg("init")
        .arg("-o")
        .arg(&config_path)
        .assert()
        .code(2) // EXIT_CONFIG_ERROR
        .stderr(predicate::str::contains("already exists"));
}

#[test]
fn init_force_overwrites() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join(".sloc-guard.toml");

    fs::write(&config_path, "old content").unwrap();

    cmd()
        .arg("init")
        .arg("-o")
        .arg(&config_path)
        .arg("--force")
        .assert()
        .success();

    let content = fs::read_to_string(&config_path).unwrap();
    assert!(content.contains("[default]"));
    assert!(!content.contains("old content"));
}

// ============================================================================
// Config Command Integration Tests
// ============================================================================

#[test]
fn config_validate_valid_file() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.toml");

    let config_content = r#"
[default]
max_lines = 500
extensions = ["rs"]
skip_comments = true
skip_blank = true
warn_threshold = 0.9

[exclude]
patterns = ["**/target/**"]
"#;
    fs::write(&config_path, config_content).unwrap();

    cmd()
        .arg("config")
        .arg("validate")
        .arg("-c")
        .arg(&config_path)
        .assert()
        .success()
        .stdout(predicate::str::contains("valid"));
}

#[test]
fn config_validate_invalid_toml() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("invalid.toml");

    fs::write(&config_path, "this is { not valid").unwrap();

    cmd()
        .arg("config")
        .arg("validate")
        .arg("-c")
        .arg(&config_path)
        .assert()
        .code(2);
}

#[test]
fn config_validate_invalid_warn_threshold() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.toml");

    let config_content = r"
[default]
warn_threshold = 1.5
";
    fs::write(&config_path, config_content).unwrap();

    cmd()
        .arg("config")
        .arg("validate")
        .arg("-c")
        .arg(&config_path)
        .assert()
        .code(2)
        .stderr(predicate::str::contains("warn_threshold"));
}

#[test]
fn config_validate_invalid_glob_pattern() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.toml");

    let config_content = r#"
[exclude]
patterns = ["[invalid"]
"#;
    fs::write(&config_path, config_content).unwrap();

    cmd()
        .arg("config")
        .arg("validate")
        .arg("-c")
        .arg(&config_path)
        .assert()
        .code(2)
        .stderr(predicate::str::contains("Invalid glob"));
}

#[test]
fn config_validate_nonexistent_file() {
    cmd()
        .arg("config")
        .arg("validate")
        .arg("-c")
        .arg("nonexistent.toml")
        .assert()
        .code(2)
        .stderr(predicate::str::contains("not found"));
}

#[test]
fn config_show_text_format() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.toml");

    let config_content = r"
[default]
max_lines = 300
";
    fs::write(&config_path, config_content).unwrap();

    cmd()
        .arg("config")
        .arg("show")
        .arg("-c")
        .arg(&config_path)
        .assert()
        .success()
        .stdout(predicate::str::contains("Effective Configuration"))
        .stdout(predicate::str::contains("max_lines = 300"));
}

#[test]
fn config_show_json_format() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.toml");

    let config_content = r"
[default]
max_lines = 300
";
    fs::write(&config_path, config_content).unwrap();

    cmd()
        .arg("config")
        .arg("show")
        .arg("-c")
        .arg(&config_path)
        .arg("--format")
        .arg("json")
        .assert()
        .success()
        .stdout(predicate::str::contains("\"max_lines\": 300"));
}

// ============================================================================
// Config File Integration Tests
// ============================================================================

#[test]
fn check_respects_config_file() {
    let temp_dir = TempDir::new().unwrap();

    // Create config with low max_lines
    let config_path = temp_dir.path().join(".sloc-guard.toml");
    let config_content = r#"
[default]
max_lines = 10
extensions = ["rs"]
"#;
    fs::write(&config_path, config_content).unwrap();

    // Create rust file with 20 lines
    let rust_file = temp_dir.path().join("main.rs");
    let content = generate_lines(20, "let x");
    fs::write(&rust_file, content).unwrap();

    // Should fail because config sets max_lines = 10
    cmd()
        .arg("check")
        .arg(temp_dir.path())
        .arg("-c")
        .arg(&config_path)
        .assert()
        .code(1);
}

#[test]
fn check_respects_extension_rules() {
    let temp_dir = TempDir::new().unwrap();

    // Create config with different limits per extension
    let config_path = temp_dir.path().join(".sloc-guard.toml");
    let config_content = r#"
[default]
max_lines = 500
extensions = ["rs", "py"]

[rules.python]
extensions = ["py"]
max_lines = 10
"#;
    fs::write(&config_path, config_content).unwrap();

    // Create files: rs with 50 lines, py with 20 lines
    let rs_file = temp_dir.path().join("main.rs");
    let py_file = temp_dir.path().join("app.py");

    let rs_content = generate_lines(50, "let x");
    let py_content = generate_py_lines(20);

    fs::write(&rs_file, rs_content).unwrap();
    fs::write(&py_file, py_content).unwrap();

    // Should fail because Python file exceeds its rule limit (10)
    cmd()
        .arg("check")
        .arg(temp_dir.path())
        .arg("-c")
        .arg(&config_path)
        .assert()
        .code(1)
        .stdout(predicate::str::contains("app.py"))
        .stdout(predicate::str::contains("FAILED"));
}

#[test]
fn check_respects_file_override() {
    let temp_dir = TempDir::new().unwrap();

    // Create config with override for specific file
    // Override matching uses ends_with() on full path, so just filename works
    let config_path = temp_dir.path().join(".sloc-guard.toml");

    let config_content = r#"
[default]
max_lines = 10
extensions = ["rs"]

[[override]]
path = "legacy.rs"
max_lines = 100
"#;
    fs::write(&config_path, config_content).unwrap();

    // Create two files: one regular, one with override
    let normal_file = temp_dir.path().join("normal.rs");
    let legacy_file = temp_dir.path().join("legacy.rs");

    let content = generate_lines(50, "let x");
    fs::write(&normal_file, &content).unwrap();
    fs::write(&legacy_file, &content).unwrap();

    // normal.rs (50 lines, limit 10) -> FAILED
    // legacy.rs (50 lines, limit 100 due to override) -> PASSED
    cmd()
        .arg("check")
        .arg(temp_dir.path())
        .arg("-c")
        .arg(&config_path)
        .assert()
        .code(1)
        .stdout(predicate::str::contains("normal.rs"))
        .stdout(predicate::str::contains("FAILED"))
        .stdout(predicate::str::contains("1 passed")); // legacy.rs passes
}

// ============================================================================
// Error Handling Integration Tests
// ============================================================================

#[test]
fn sarif_format_not_implemented() {
    let temp_dir = TempDir::new().unwrap();
    fs::write(temp_dir.path().join("main.rs"), "fn main() {}\n").unwrap();

    cmd()
        .arg("check")
        .arg(temp_dir.path())
        .arg("--no-config")
        .arg("--format")
        .arg("sarif")
        .assert()
        .code(2)
        .stderr(predicate::str::contains("not yet implemented"));
}

#[test]
fn markdown_format_not_implemented() {
    let temp_dir = TempDir::new().unwrap();
    fs::write(temp_dir.path().join("main.rs"), "fn main() {}\n").unwrap();

    cmd()
        .arg("check")
        .arg(temp_dir.path())
        .arg("--no-config")
        .arg("--format")
        .arg("markdown")
        .assert()
        .code(2)
        .stderr(predicate::str::contains("not yet implemented"));
}

#[test]
fn help_displays_usage() {
    cmd()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("sloc-guard"))
        .stdout(predicate::str::contains("check"))
        .stdout(predicate::str::contains("stats"))
        .stdout(predicate::str::contains("init"));
}

#[test]
fn check_help_displays_options() {
    cmd()
        .arg("check")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("--max-lines"))
        .stdout(predicate::str::contains("--ext"))
        .stdout(predicate::str::contains("--exclude"))
        .stdout(predicate::str::contains("--warn-only"));
}
