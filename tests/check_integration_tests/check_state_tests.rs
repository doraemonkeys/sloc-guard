//! State management tests - baseline, auto-snapshot, and line counting behavior.

use crate::common::{BASIC_CONFIG_V2, STRICT_CONFIG_V2, TestFixture};
use crate::sloc_guard;
use predicates::prelude::*;

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
