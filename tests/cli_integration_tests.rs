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
        .arg("--no-gitignore")
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
        .arg("--no-gitignore")
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
        .arg("--no-gitignore")
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
        .arg("--no-gitignore")
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
        .arg("--no-gitignore")
        .arg(temp_dir.path())
        .arg("--no-config")
        .assert()
        .success();

    // Override to 50 should fail
    cmd()
        .arg("check")
        .arg("--no-gitignore")
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
        .arg("--no-gitignore")
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
        .arg("--no-gitignore")
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
        .arg("--no-gitignore")
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
        .arg("--no-gitignore")
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
        .arg("--no-gitignore")
        .arg(temp_dir.path())
        .arg("--no-config")
        .arg("--max-lines")
        .arg("50")
        .assert()
        .success();

    // Without skip_comments, all 60 lines count -> fails at limit 50
    cmd()
        .arg("check")
        .arg("--no-gitignore")
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
        .arg("--no-gitignore")
        .arg(temp_dir.path())
        .arg("--no-config")
        .arg("--max-lines")
        .arg("50")
        .assert()
        .success();

    // Without skip_blank, all 60 lines count -> fails
    cmd()
        .arg("check")
        .arg("--no-gitignore")
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
        .arg("--no-gitignore")
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
        .arg("--no-gitignore")
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
        .arg("--no-gitignore")
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
        .arg("--no-gitignore")
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
        .arg("--no-gitignore")
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
        .arg("--no-gitignore")
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
        .arg("--no-gitignore")
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
        .arg("--no-gitignore")
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
        .arg("--no-gitignore")
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
        .arg("--no-gitignore")
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
        .arg("--no-gitignore")
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
fn sarif_format_works() {
    let temp_dir = TempDir::new().unwrap();
    fs::write(temp_dir.path().join("main.rs"), "fn main() {}\n").unwrap();

    cmd()
        .arg("check")
        .arg("--no-gitignore")
        .arg(temp_dir.path())
        .arg("--no-config")
        .arg("--format")
        .arg("sarif")
        .assert()
        .success()
        .stdout(predicate::str::contains("$schema"))
        .stdout(predicate::str::contains("sarif-schema-2.1.0"));
}

#[test]
fn markdown_format_works() {
    let temp_dir = TempDir::new().unwrap();
    fs::write(temp_dir.path().join("main.rs"), "fn main() {}\n").unwrap();

    cmd()
        .arg("check")
        .arg("--no-gitignore")
        .arg(temp_dir.path())
        .arg("--no-config")
        .arg("--format")
        .arg("markdown")
        .assert()
        .success()
        .stdout(predicate::str::contains("## SLOC Guard Results"))
        .stdout(predicate::str::contains("| Total Files |"))
        .stdout(predicate::str::contains("| ✅ Passed |"));
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

// ============================================================================
// Baseline Command Integration Tests
// ============================================================================

#[test]
fn baseline_update_creates_file() {
    let temp_dir = TempDir::new().unwrap();

    // Create a file that exceeds the default limit
    let rust_file = temp_dir.path().join("large.rs");
    let content = generate_lines(600, "let x");
    fs::write(&rust_file, content).unwrap();

    let baseline_path = temp_dir.path().join("baseline.json");

    cmd()
        .arg("baseline")
        .arg("update")
        .arg("--no-gitignore")
        .arg(temp_dir.path())
        .arg("--no-config")
        .arg("-o")
        .arg(&baseline_path)
        .assert()
        .success()
        .stdout(predicate::str::contains("Baseline created with 1 file"));

    assert!(baseline_path.exists());
    let content = fs::read_to_string(&baseline_path).unwrap();
    assert!(content.contains("\"version\": 1"));
    assert!(content.contains("large.rs"));
}

#[test]
fn baseline_update_empty_for_passing_files() {
    let temp_dir = TempDir::new().unwrap();

    // Create a small file that passes
    let rust_file = temp_dir.path().join("small.rs");
    fs::write(&rust_file, "fn main() {}\n").unwrap();

    let baseline_path = temp_dir.path().join("baseline.json");

    cmd()
        .arg("baseline")
        .arg("update")
        .arg("--no-gitignore")
        .arg(temp_dir.path())
        .arg("--no-config")
        .arg("-o")
        .arg(&baseline_path)
        .assert()
        .success()
        .stdout(predicate::str::contains("Baseline created with 0 file"));
}

#[test]
fn baseline_update_with_config() {
    let temp_dir = TempDir::new().unwrap();

    // Create config with low limit
    let config_path = temp_dir.path().join("config.toml");
    fs::write(
        &config_path,
        r#"
[default]
max_lines = 5
extensions = ["rs"]
"#,
    )
    .unwrap();

    // Create a file that exceeds the config limit but not default
    let rust_file = temp_dir.path().join("medium.rs");
    let content = generate_lines(10, "let x");
    fs::write(&rust_file, content).unwrap();

    let baseline_path = temp_dir.path().join("baseline.json");

    cmd()
        .arg("baseline")
        .arg("update")
        .arg("--no-gitignore")
        .arg(temp_dir.path())
        .arg("-c")
        .arg(&config_path)
        .arg("-o")
        .arg(&baseline_path)
        .assert()
        .success()
        .stdout(predicate::str::contains("Baseline created with 1 file"));
}

// ============================================================================
// Check with Baseline Integration Tests
// ============================================================================

#[test]
fn check_with_baseline_grandfathers_violations() {
    let temp_dir = TempDir::new().unwrap();

    // Create a large file that would fail
    let rust_file = temp_dir.path().join("large.rs");
    let content = generate_lines(600, "let x");
    fs::write(&rust_file, &content).unwrap();

    // First, create a baseline
    let baseline_path = temp_dir.path().join("baseline.json");
    cmd()
        .arg("baseline")
        .arg("update")
        .arg("--no-gitignore")
        .arg(temp_dir.path())
        .arg("--no-config")
        .arg("-o")
        .arg(&baseline_path)
        .assert()
        .success();

    // Now check with baseline - should pass (grandfathered)
    // Without verbose mode, grandfathered files show in summary only
    cmd()
        .arg("check")
        .arg("--no-gitignore")
        .arg(temp_dir.path())
        .arg("--no-config")
        .arg("--baseline")
        .arg(&baseline_path)
        .assert()
        .success()
        .stdout(predicate::str::contains("1 grandfathered"));

    // With verbose mode, grandfathered files show with GRANDFATHERED status
    cmd()
        .arg("-v")
        .arg("check")
        .arg("--no-gitignore")
        .arg(temp_dir.path())
        .arg("--no-config")
        .arg("--baseline")
        .arg(&baseline_path)
        .assert()
        .success()
        .stdout(predicate::str::contains("GRANDFATHERED"))
        .stdout(predicate::str::contains("large.rs"));
}

#[test]
fn check_with_baseline_fails_for_new_violations() {
    let temp_dir = TempDir::new().unwrap();

    // Create an empty baseline
    let baseline_path = temp_dir.path().join("baseline.json");
    fs::write(&baseline_path, r#"{"version": 1, "files": {}}"#).unwrap();

    // Create a large file that would fail (not in baseline)
    let rust_file = temp_dir.path().join("large.rs");
    let content = generate_lines(600, "let x");
    fs::write(&rust_file, content).unwrap();

    // Check with baseline - should fail (not grandfathered)
    cmd()
        .arg("check")
        .arg("--no-gitignore")
        .arg(temp_dir.path())
        .arg("--no-config")
        .arg("--baseline")
        .arg(&baseline_path)
        .assert()
        .code(1)
        .stdout(predicate::str::contains("FAILED"));
}

#[test]
fn check_baseline_file_not_found_error() {
    let temp_dir = TempDir::new().unwrap();
    fs::write(temp_dir.path().join("main.rs"), "fn main() {}\n").unwrap();

    cmd()
        .arg("check")
        .arg("--no-gitignore")
        .arg(temp_dir.path())
        .arg("--no-config")
        .arg("--baseline")
        .arg("nonexistent.json")
        .assert()
        .code(2)
        .stderr(predicate::str::contains("not found"));
}

// ============================================================================
// Path Rules Integration Tests
// ============================================================================

#[test]
fn check_path_rules_override_extension_rules() {
    let temp_dir = TempDir::new().unwrap();

    // Create config with path-based rule
    let config_path = temp_dir.path().join("config.toml");
    fs::write(
        &config_path,
        r#"
[default]
max_lines = 10
extensions = ["rs"]

[[path_rules]]
pattern = "**/generated/**"
max_lines = 1000
"#,
    )
    .unwrap();

    // Create directory structure
    let generated_dir = temp_dir.path().join("generated");
    fs::create_dir(&generated_dir).unwrap();

    // Create a file in generated/ with 50 lines (exceeds default but under path_rule)
    let generated_file = generated_dir.join("types.rs");
    let content = generate_lines(50, "let x");
    fs::write(&generated_file, content).unwrap();

    // Create a file outside generated/ with same lines (should fail)
    let normal_file = temp_dir.path().join("main.rs");
    let content = generate_lines(50, "let y");
    fs::write(&normal_file, content).unwrap();

    // Check - generated/types.rs should pass, main.rs should fail
    cmd()
        .arg("check")
        .arg("--no-gitignore")
        .arg(temp_dir.path())
        .arg("-c")
        .arg(&config_path)
        .assert()
        .code(1)
        .stdout(predicate::str::contains("main.rs"))
        .stdout(predicate::str::contains("FAILED"))
        .stdout(predicate::str::contains("1 passed")); // generated/types.rs
}

#[test]
fn check_path_rules_warn_threshold() {
    let temp_dir = TempDir::new().unwrap();

    // Create config with path-based rule with custom warn_threshold
    let config_path = temp_dir.path().join("config.toml");
    fs::write(
        &config_path,
        r#"
[default]
max_lines = 100
extensions = ["rs"]
warn_threshold = 0.9

[[path_rules]]
pattern = "**/generated/**"
max_lines = 100
warn_threshold = 1.0
"#,
    )
    .unwrap();

    // Create directory structure
    let generated_dir = temp_dir.path().join("generated");
    fs::create_dir(&generated_dir).unwrap();

    // Create a file at 95% capacity in generated/ (should NOT warn due to warn_threshold = 1.0)
    let generated_file = generated_dir.join("types.rs");
    let content = generate_lines(95, "let x");
    fs::write(&generated_file, content).unwrap();

    // Create a file at 95% capacity outside generated/ (should warn)
    let normal_file = temp_dir.path().join("main.rs");
    let content = generate_lines(95, "let y");
    fs::write(&normal_file, content).unwrap();

    cmd()
        .arg("check")
        .arg("--no-gitignore")
        .arg(temp_dir.path())
        .arg("-c")
        .arg(&config_path)
        .assert()
        .success()
        .stdout(predicate::str::contains("WARNING"))
        .stdout(predicate::str::contains("main.rs"))
        .stdout(predicate::str::contains("1 warning"));
}

// ============================================================================
// Strict Mode Integration Tests
// ============================================================================

#[test]
fn check_strict_mode_fails_on_warning() {
    let temp_dir = TempDir::new().unwrap();

    // Create a file at 95% of limit (triggers warning at 0.9 threshold)
    let rust_file = temp_dir.path().join("nearmax.rs");
    let content = generate_lines(95, "let x");
    fs::write(&rust_file, content).unwrap();

    // Without strict mode - should pass with warning
    cmd()
        .arg("check")
        .arg("--no-gitignore")
        .arg(temp_dir.path())
        .arg("--no-config")
        .arg("--max-lines")
        .arg("100")
        .arg("--warn-threshold")
        .arg("0.9")
        .assert()
        .success()
        .stdout(predicate::str::contains("WARNING"));

    // With strict mode - should fail
    cmd()
        .arg("check")
        .arg("--no-gitignore")
        .arg(temp_dir.path())
        .arg("--no-config")
        .arg("--max-lines")
        .arg("100")
        .arg("--warn-threshold")
        .arg("0.9")
        .arg("--strict")
        .assert()
        .code(1)
        .stdout(predicate::str::contains("WARNING"));
}

#[test]
fn check_strict_mode_from_config() {
    let temp_dir = TempDir::new().unwrap();

    // Create config with strict mode enabled
    let config_path = temp_dir.path().join("config.toml");
    fs::write(
        &config_path,
        r#"
[default]
max_lines = 100
extensions = ["rs"]
warn_threshold = 0.9
strict = true
"#,
    )
    .unwrap();

    // Create a file at 95% of limit
    let rust_file = temp_dir.path().join("nearmax.rs");
    let content = generate_lines(95, "let x");
    fs::write(&rust_file, content).unwrap();

    // Should fail due to strict mode in config
    cmd()
        .arg("check")
        .arg("--no-gitignore")
        .arg(temp_dir.path())
        .arg("-c")
        .arg(&config_path)
        .assert()
        .code(1)
        .stdout(predicate::str::contains("WARNING"));
}

// ============================================================================
// Inline Ignore Directive Integration Tests
// ============================================================================

#[test]
fn check_inline_ignore_file_skips_file() {
    let temp_dir = TempDir::new().unwrap();

    // Create a large file with ignore directive
    let rust_file = temp_dir.path().join("ignored.rs");
    let mut content = String::from("// sloc-guard:ignore-file\n");
    content.push_str(&generate_lines(600, "let x"));
    fs::write(&rust_file, content).unwrap();

    // Should pass (file is ignored)
    cmd()
        .arg("check")
        .arg("--no-gitignore")
        .arg(temp_dir.path())
        .arg("--no-config")
        .assert()
        .success()
        .stdout(predicate::str::contains("0 files checked"));
}

#[test]
fn check_inline_ignore_file_in_first_10_lines() {
    let temp_dir = TempDir::new().unwrap();

    // Create file with ignore directive on line 10
    let rust_file = temp_dir.path().join("ignored.rs");
    let mut content = String::new();
    for i in 0..9 {
        let _ = writeln!(content, "// header line {i}");
    }
    content.push_str("// sloc-guard:ignore-file\n");
    content.push_str(&generate_lines(600, "let x"));
    fs::write(&rust_file, content).unwrap();

    // Should pass (directive within first 10 lines)
    cmd()
        .arg("check")
        .arg("--no-gitignore")
        .arg(temp_dir.path())
        .arg("--no-config")
        .assert()
        .success()
        .stdout(predicate::str::contains("0 files checked"));
}

#[test]
fn check_inline_ignore_after_line_10_not_effective() {
    let temp_dir = TempDir::new().unwrap();

    // Create file with ignore directive on line 11 (beyond first 10)
    let rust_file = temp_dir.path().join("notignored.rs");
    let mut content = String::new();
    for i in 0..10 {
        let _ = writeln!(content, "// header line {i}");
    }
    content.push_str("// sloc-guard:ignore-file\n");
    content.push_str(&generate_lines(600, "let x"));
    fs::write(&rust_file, content).unwrap();

    // Should fail (directive beyond first 10 lines is not effective)
    cmd()
        .arg("check")
        .arg("--no-gitignore")
        .arg(temp_dir.path())
        .arg("--no-config")
        .assert()
        .code(1)
        .stdout(predicate::str::contains("FAILED"));
}

// ============================================================================
// Verbose Mode Integration Tests
// ============================================================================

#[test]
fn check_verbose_shows_passed_files() {
    let temp_dir = TempDir::new().unwrap();

    // Create a small file that passes
    let rust_file = temp_dir.path().join("small.rs");
    fs::write(&rust_file, "fn main() {}\n").unwrap();

    // Without verbose - passed files not shown in detail
    let output = cmd()
        .arg("check")
        .arg("--no-gitignore")
        .arg(temp_dir.path())
        .arg("--no-config")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let output_str = String::from_utf8_lossy(&output);
    assert!(!output_str.contains("PASSED") || !output_str.contains("small.rs"));

    // With verbose - passed files shown
    cmd()
        .arg("-v")
        .arg("check")
        .arg("--no-gitignore")
        .arg(temp_dir.path())
        .arg("--no-config")
        .assert()
        .success()
        .stdout(predicate::str::contains("PASSED"))
        .stdout(predicate::str::contains("small.rs"));
}

#[test]
fn check_double_verbose_shows_more_detail() {
    let temp_dir = TempDir::new().unwrap();

    // Create a small file
    let rust_file = temp_dir.path().join("small.rs");
    fs::write(&rust_file, "fn main() {\n    // comment\n}\n").unwrap();

    // With -vv should show more detail
    cmd()
        .arg("-vv")
        .arg("check")
        .arg("--no-gitignore")
        .arg(temp_dir.path())
        .arg("--no-config")
        .assert()
        .success()
        .stdout(predicate::str::contains("small.rs"));
}

// ============================================================================
// Multi-language Integration Tests
// ============================================================================

#[test]
fn check_multiple_languages() {
    let temp_dir = TempDir::new().unwrap();

    // Create files in different languages
    fs::write(temp_dir.path().join("main.rs"), "fn main() {}\n").unwrap();
    fs::write(temp_dir.path().join("app.go"), "package main\n").unwrap();
    fs::write(temp_dir.path().join("script.py"), "print('hello')\n").unwrap();
    fs::write(temp_dir.path().join("index.js"), "console.log('hi');\n").unwrap();
    fs::write(temp_dir.path().join("main.ts"), "const x: number = 1;\n").unwrap();
    fs::write(temp_dir.path().join("lib.c"), "int main() { return 0; }\n").unwrap();
    fs::write(
        temp_dir.path().join("lib.cpp"),
        "int main() { return 0; }\n",
    )
    .unwrap();

    cmd()
        .arg("check")
        .arg("--no-gitignore")
        .arg(temp_dir.path())
        .arg("--no-config")
        .assert()
        .success()
        .stdout(predicate::str::contains("7 files checked"))
        .stdout(predicate::str::contains("7 passed"));
}

#[test]
fn stats_multiple_languages() {
    let temp_dir = TempDir::new().unwrap();

    fs::write(
        temp_dir.path().join("main.rs"),
        "fn main() {\n    let x = 1;\n}\n",
    )
    .unwrap();
    fs::write(temp_dir.path().join("app.py"), "x = 1\ny = 2\n").unwrap();

    cmd()
        .arg("stats")
        .arg("--no-gitignore")
        .arg(temp_dir.path())
        .arg("--no-config")
        .assert()
        .success()
        .stdout(predicate::str::contains("Files: 2"));
}

// ============================================================================
// Comment Handling Integration Tests
// ============================================================================

#[test]
fn check_multiline_comments_skipped() {
    let temp_dir = TempDir::new().unwrap();

    // Create a file with multi-line comments
    let rust_file = temp_dir.path().join("commented.rs");
    let content = r"
fn main() {
    /*
    This is a
    multi-line
    comment that
    spans many
    lines
    */
    let x = 1;
}
";
    fs::write(&rust_file, content).unwrap();

    // Only code lines should count (fn main, let x, and closing braces)
    cmd()
        .arg("check")
        .arg("--no-gitignore")
        .arg(temp_dir.path())
        .arg("--no-config")
        .arg("--max-lines")
        .arg("5")
        .assert()
        .success();
}

#[test]
fn check_python_comments_skipped() {
    let temp_dir = TempDir::new().unwrap();

    let py_file = temp_dir.path().join("script.py");
    let content = r"
# Single line comment
x = 1
'''
This is a
multi-line
string/comment
'''
y = 2
";
    fs::write(&py_file, content).unwrap();

    cmd()
        .arg("check")
        .arg("--no-gitignore")
        .arg(temp_dir.path())
        .arg("--no-config")
        .arg("--ext")
        .arg("py")
        .arg("--max-lines")
        .arg("5")
        .assert()
        .success();
}
