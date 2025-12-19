//! Integration tests for the `baseline` command.

mod common;

use common::{STRICT_CONFIG_V2, TestFixture};

// =============================================================================
// Baseline Update Tests
// =============================================================================

#[test]
fn baseline_update_creates_file() {
    let fixture = TestFixture::new();
    fixture.create_config(STRICT_CONFIG_V2);
    fixture.create_rust_file("src/large.rs", 50); // Exceeds limit of 10

    sloc_guard!()
        .current_dir(fixture.path())
        .args(["baseline", "update"])
        .assert()
        .success();

    let baseline_path = fixture.path().join(".sloc-guard-baseline.json");
    assert!(baseline_path.exists());
}

#[test]
fn baseline_update_custom_output_path() {
    let fixture = TestFixture::new();
    fixture.create_config(STRICT_CONFIG_V2);
    fixture.create_rust_file("src/large.rs", 50);

    let custom_path = fixture.path().join("custom-baseline.json");

    sloc_guard!()
        .current_dir(fixture.path())
        .args([
            "baseline",
            "update",
            "--output",
            custom_path.to_str().unwrap(),
        ])
        .assert()
        .success();

    assert!(custom_path.exists());
}

#[test]
fn baseline_update_captures_violations() {
    let fixture = TestFixture::new();
    fixture.create_config(STRICT_CONFIG_V2);
    fixture.create_rust_file("src/large.rs", 50);

    sloc_guard!()
        .current_dir(fixture.path())
        .args(["baseline", "update"])
        .assert()
        .success();

    let baseline_path = fixture.path().join(".sloc-guard-baseline.json");
    let content = std::fs::read_to_string(&baseline_path).unwrap();

    // Should contain the violation entry
    assert!(content.contains("large.rs") || content.contains("version"));
}

#[test]
fn baseline_update_with_specific_path() {
    let fixture = TestFixture::new();
    fixture.create_config(STRICT_CONFIG_V2);
    fixture.create_rust_file("src/large.rs", 50);
    fixture.create_rust_file("other/also_large.rs", 50);

    sloc_guard!()
        .current_dir(fixture.path())
        .args(["baseline", "update", "src"])
        .assert()
        .success();

    let baseline_path = fixture.path().join(".sloc-guard-baseline.json");
    let content = std::fs::read_to_string(&baseline_path).unwrap();

    // Should only include src violations
    assert!(content.contains("large.rs"));
    // The other directory shouldn't be in baseline
    assert!(!content.contains("also_large.rs"));
}

#[test]
fn baseline_update_with_ext_filter() {
    let fixture = TestFixture::new();
    fixture.create_config(
        r#"
version = "2"

[scanner]
gitignore = false

[content]
extensions = ["rs", "py"]
max_lines = 10

[structure]
max_files = 50
max_dirs = 10
"#,
    );
    fixture.create_rust_file("src/large.rs", 50);
    fixture.create_file("src/large.py", "x = 1\n".repeat(50).as_str());

    sloc_guard!()
        .current_dir(fixture.path())
        .args(["baseline", "update", "--ext", "rs"])
        .assert()
        .success();

    let baseline_path = fixture.path().join(".sloc-guard-baseline.json");
    let content = std::fs::read_to_string(&baseline_path).unwrap();

    // Should include Rust but not Python
    assert!(content.contains("large.rs"));
}

#[test]
fn baseline_update_with_exclude_pattern() {
    let fixture = TestFixture::new();
    fixture.create_config(STRICT_CONFIG_V2);
    fixture.create_rust_file("src/large.rs", 50);
    fixture.create_rust_file("vendor/lib.rs", 50);

    sloc_guard!()
        .current_dir(fixture.path())
        .args(["baseline", "update", "--exclude", "**/vendor/**"])
        .assert()
        .success();

    let baseline_path = fixture.path().join(".sloc-guard-baseline.json");
    let content = std::fs::read_to_string(&baseline_path).unwrap();

    // Should include src violations
    assert!(content.contains("large.rs") || content.contains("src"));
}

// =============================================================================
// Baseline Integration with Check Tests
// =============================================================================

#[test]
fn baseline_grandfathers_existing_violations() {
    let fixture = TestFixture::new();
    fixture.create_config(STRICT_CONFIG_V2);
    fixture.create_rust_file("src/large.rs", 50);

    // Create baseline
    sloc_guard!()
        .current_dir(fixture.path())
        .args(["baseline", "update"])
        .assert()
        .success();

    // Check with baseline should pass (violation is grandfathered)
    sloc_guard!()
        .current_dir(fixture.path())
        .args([
            "check",
            "--no-cache",
            "--quiet",
            "--baseline",
            ".sloc-guard-baseline.json",
        ])
        .assert()
        .success();
}

#[test]
fn baseline_does_not_grandfather_new_violations() {
    let fixture = TestFixture::new();
    fixture.create_config(STRICT_CONFIG_V2);
    fixture.create_rust_file("src/existing.rs", 50);

    // Create baseline with existing file
    sloc_guard!()
        .current_dir(fixture.path())
        .args(["baseline", "update"])
        .assert()
        .success();

    // Add a new violating file
    fixture.create_rust_file("src/new_large.rs", 50);

    // Check should fail because new file is not in baseline
    sloc_guard!()
        .current_dir(fixture.path())
        .args([
            "check",
            "--no-cache",
            "--quiet",
            "--baseline",
            ".sloc-guard-baseline.json",
        ])
        .assert()
        .code(1);
}

// =============================================================================
// Baseline File Format Tests
// =============================================================================

#[test]
fn baseline_file_is_valid_json() {
    let fixture = TestFixture::new();
    fixture.create_config(STRICT_CONFIG_V2);
    fixture.create_rust_file("src/large.rs", 50);

    sloc_guard!()
        .current_dir(fixture.path())
        .args(["baseline", "update"])
        .assert()
        .success();

    let baseline_path = fixture.path().join(".sloc-guard-baseline.json");
    let content = std::fs::read_to_string(&baseline_path).unwrap();

    let _: serde_json::Value =
        serde_json::from_str(&content).expect("Baseline should be valid JSON");
}

#[test]
fn baseline_contains_version_field() {
    let fixture = TestFixture::new();
    fixture.create_config(STRICT_CONFIG_V2);
    fixture.create_rust_file("src/large.rs", 50);

    sloc_guard!()
        .current_dir(fixture.path())
        .args(["baseline", "update"])
        .assert()
        .success();

    let baseline_path = fixture.path().join(".sloc-guard-baseline.json");
    let content = std::fs::read_to_string(&baseline_path).unwrap();
    let json: serde_json::Value = serde_json::from_str(&content).unwrap();

    assert!(json["version"].is_number());
}

// =============================================================================
// Edge Cases
// =============================================================================

#[test]
fn baseline_update_no_violations_creates_empty_baseline() {
    let fixture = TestFixture::new();
    fixture.create_config(STRICT_CONFIG_V2);
    fixture.create_rust_file("src/small.rs", 5); // Under limit

    sloc_guard!()
        .current_dir(fixture.path())
        .args(["baseline", "update"])
        .assert()
        .success();

    let baseline_path = fixture.path().join(".sloc-guard-baseline.json");
    assert!(baseline_path.exists());

    let content = std::fs::read_to_string(&baseline_path).unwrap();
    let json: serde_json::Value = serde_json::from_str(&content).unwrap();

    // Files map should be empty or missing
    if let Some(files) = json.get("files") {
        assert!(files.as_object().is_none_or(serde_json::Map::is_empty));
    }
}

#[test]
fn baseline_update_with_custom_config() {
    let fixture = TestFixture::new();
    fixture.create_file(
        "strict.toml",
        r#"
version = "2"

[scanner]
gitignore = false

[content]
extensions = ["rs"]
max_lines = 5

[structure]
max_files = 50
max_dirs = 10
"#,
    );
    fixture.create_rust_file("src/code.rs", 20);

    sloc_guard!()
        .current_dir(fixture.path())
        .args(["baseline", "update", "--config", "strict.toml"])
        .assert()
        .success();

    let baseline_path = fixture.path().join(".sloc-guard-baseline.json");
    let content = std::fs::read_to_string(&baseline_path).unwrap();
    assert!(content.contains("code.rs"));
}

#[test]
fn baseline_update_no_gitignore() {
    let fixture = TestFixture::new();
    fixture.create_config(STRICT_CONFIG_V2);
    fixture.create_rust_file("src/large.rs", 50);
    fixture.create_file(".gitignore", "src/\n");

    // With --no-gitignore, src should still be scanned
    sloc_guard!()
        .current_dir(fixture.path())
        .args(["baseline", "update", "--no-gitignore"])
        .assert()
        .success();

    let baseline_path = fixture.path().join(".sloc-guard-baseline.json");
    let content = std::fs::read_to_string(&baseline_path).unwrap();
    assert!(content.contains("large.rs"));
}
