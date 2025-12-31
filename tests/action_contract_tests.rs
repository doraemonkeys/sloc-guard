//! Contract tests for GitHub Action integration.
//!
//! These tests verify CLI behavior that the GitHub Action depends on.
//! If any of these tests fail, the action.yml likely needs updating.
//!
//! Action dependencies:
//! - CLI options: --config, --strict, --baseline, --diff, --write-json, --write-sarif, --format, --color
//! - JSON output: `.summary.{total_files, passed, failed, warnings, grandfathered}`
//! - Exit codes: 0 = success, 1 = violations found, 2 = config error

mod common;

use common::{BASIC_CONFIG_V2, STRICT_CONFIG_V2, TestFixture};
use predicates::prelude::*;

// =============================================================================
// CLI Option Existence Tests
// These verify that options used by action.yml exist and work
// =============================================================================

#[test]
fn action_contract_strict_option_exists() {
    // Action uses: CMD+=(--strict) when fail-on-warning is true
    let fixture = TestFixture::new();
    fixture.create_config(BASIC_CONFIG_V2);
    // 85 lines triggers warning with warn_threshold = 0.8
    fixture.create_rust_file("src/warning.rs", 85);

    // Without --strict: exits 0 (warnings don't fail)
    sloc_guard!()
        .current_dir(fixture.path())
        .args(["check", "--no-cache", "--quiet"])
        .assert()
        .success();

    // With --strict: exits 1 (warnings become failures)
    sloc_guard!()
        .current_dir(fixture.path())
        .args(["check", "--no-cache", "--quiet", "--strict"])
        .assert()
        .code(1);
}

#[test]
fn action_contract_config_option_exists() {
    // Action uses: CMD+=(--config "$INPUT_CONFIG_PATH")
    let fixture = TestFixture::new();
    fixture.create_file("custom.toml", BASIC_CONFIG_V2);
    fixture.create_rust_file("src/main.rs", 10);

    sloc_guard!()
        .current_dir(fixture.path())
        .args(["check", "--no-cache", "--quiet", "--config", "custom.toml"])
        .assert()
        .success();
}

#[test]
fn action_contract_baseline_option_exists() {
    // Action uses: CMD+=(--baseline "$INPUT_BASELINE")
    let fixture = TestFixture::new();
    fixture.create_config(STRICT_CONFIG_V2);
    fixture.create_rust_file("src/large.rs", 50);

    let baseline_path = fixture.path().join("baseline.json");

    // Create baseline
    sloc_guard!()
        .current_dir(fixture.path())
        .args([
            "check",
            "--no-cache",
            "--baseline",
            baseline_path.to_str().unwrap(),
            "--update-baseline",
        ])
        .assert()
        .code(1);

    // Use baseline - grandfathers violation
    sloc_guard!()
        .current_dir(fixture.path())
        .args([
            "check",
            "--no-cache",
            "--quiet",
            "--baseline",
            baseline_path.to_str().unwrap(),
        ])
        .assert()
        .success();
}

#[test]
fn action_contract_diff_option_exists() {
    // Action uses: CMD+=(--diff "$INPUT_DIFF")
    // This test just verifies the option is accepted (no actual git repo)
    let fixture = TestFixture::new();
    fixture.create_config(BASIC_CONFIG_V2);
    fixture.create_rust_file("src/main.rs", 10);

    // --diff without git repo should not crash (graceful handling)
    sloc_guard!()
        .current_dir(fixture.path())
        .args(["check", "--no-cache", "--diff", "HEAD"])
        .assert()
        // Either succeeds (no files changed) or exits 2 (git error)
        .code(predicate::in_iter([0, 2]));
}

#[test]
fn action_contract_write_json_option_exists() {
    // Action uses: CMD+=(--write-json "$JSON_FILE")
    let fixture = TestFixture::new();
    fixture.create_config(BASIC_CONFIG_V2);
    fixture.create_rust_file("src/main.rs", 10);

    let json_path = fixture.path().join("output.json");

    sloc_guard!()
        .current_dir(fixture.path())
        .args([
            "check",
            "--no-cache",
            "--write-json",
            json_path.to_str().unwrap(),
        ])
        .assert()
        .success();

    assert!(json_path.exists(), "--write-json must create the file");
}

#[test]
fn action_contract_write_sarif_option_exists() {
    // Action uses: CMD+=(--write-sarif "$SARIF_FILE")
    let fixture = TestFixture::new();
    fixture.create_config(BASIC_CONFIG_V2);
    fixture.create_rust_file("src/main.rs", 10);

    let sarif_path = fixture.path().join("output.sarif");

    sloc_guard!()
        .current_dir(fixture.path())
        .args([
            "check",
            "--no-cache",
            "--write-sarif",
            sarif_path.to_str().unwrap(),
        ])
        .assert()
        .success();

    assert!(sarif_path.exists(), "--write-sarif must create the file");
}

#[test]
fn action_contract_format_text_option_exists() {
    // Action uses: CMD+=(--format text --color never)
    let fixture = TestFixture::new();
    fixture.create_config(BASIC_CONFIG_V2);
    fixture.create_rust_file("src/main.rs", 10);

    sloc_guard!()
        .current_dir(fixture.path())
        .args([
            "check",
            "--no-cache",
            "--format",
            "text",
            "--color",
            "never",
        ])
        .assert()
        .success();
}

// =============================================================================
// JSON Output Schema Tests
// Action parses: .summary.{total_files, passed, failed, warnings, grandfathered}
// =============================================================================

#[test]
fn action_contract_json_has_summary_total_files() {
    let fixture = TestFixture::new();
    fixture.create_config(BASIC_CONFIG_V2);
    fixture.create_rust_file("src/main.rs", 10);

    let json_path = fixture.path().join("output.json");

    sloc_guard!()
        .current_dir(fixture.path())
        .args([
            "check",
            "--no-cache",
            "--write-json",
            json_path.to_str().unwrap(),
        ])
        .assert()
        .success();

    let content = std::fs::read_to_string(&json_path).unwrap();
    let json: serde_json::Value = serde_json::from_str(&content).unwrap();

    assert!(
        json.get("summary").is_some(),
        "JSON must have 'summary' field"
    );
    assert!(
        json["summary"].get("total_files").is_some(),
        "summary must have 'total_files'"
    );
    assert!(
        json["summary"]["total_files"].is_number(),
        "total_files must be a number"
    );
}

#[test]
fn action_contract_json_has_summary_passed() {
    let fixture = TestFixture::new();
    fixture.create_config(BASIC_CONFIG_V2);
    fixture.create_rust_file("src/main.rs", 10);

    let json_path = fixture.path().join("output.json");

    sloc_guard!()
        .current_dir(fixture.path())
        .args([
            "check",
            "--no-cache",
            "--write-json",
            json_path.to_str().unwrap(),
        ])
        .assert()
        .success();

    let content = std::fs::read_to_string(&json_path).unwrap();
    let json: serde_json::Value = serde_json::from_str(&content).unwrap();

    assert!(
        json["summary"].get("passed").is_some(),
        "summary must have 'passed'"
    );
    assert!(
        json["summary"]["passed"].is_number(),
        "passed must be a number"
    );
}

#[test]
fn action_contract_json_has_summary_failed() {
    let fixture = TestFixture::new();
    fixture.create_config(STRICT_CONFIG_V2);
    fixture.create_rust_file("src/large.rs", 50);

    let json_path = fixture.path().join("output.json");

    sloc_guard!()
        .current_dir(fixture.path())
        .args([
            "check",
            "--no-cache",
            "--write-json",
            json_path.to_str().unwrap(),
        ])
        .assert()
        .code(1);

    let content = std::fs::read_to_string(&json_path).unwrap();
    let json: serde_json::Value = serde_json::from_str(&content).unwrap();

    assert!(
        json["summary"].get("failed").is_some(),
        "summary must have 'failed'"
    );
    assert!(
        json["summary"]["failed"].is_number(),
        "failed must be a number"
    );
    assert!(
        json["summary"]["failed"].as_u64().unwrap() > 0,
        "failed count should be > 0 for failing check"
    );
}

#[test]
fn action_contract_json_has_summary_warnings() {
    let fixture = TestFixture::new();
    fixture.create_config(BASIC_CONFIG_V2);
    // 85 lines triggers warning (> 80% of 100)
    fixture.create_rust_file("src/warning.rs", 85);

    let json_path = fixture.path().join("output.json");

    sloc_guard!()
        .current_dir(fixture.path())
        .args([
            "check",
            "--no-cache",
            "--write-json",
            json_path.to_str().unwrap(),
        ])
        .assert()
        .success();

    let content = std::fs::read_to_string(&json_path).unwrap();
    let json: serde_json::Value = serde_json::from_str(&content).unwrap();

    assert!(
        json["summary"].get("warnings").is_some(),
        "summary must have 'warnings'"
    );
    assert!(
        json["summary"]["warnings"].is_number(),
        "warnings must be a number"
    );
    assert!(
        json["summary"]["warnings"].as_u64().unwrap() > 0,
        "warnings count should be > 0"
    );
}

#[test]
fn action_contract_json_has_summary_grandfathered() {
    let fixture = TestFixture::new();
    fixture.create_config(STRICT_CONFIG_V2);
    fixture.create_rust_file("src/large.rs", 50);

    let baseline_path = fixture.path().join("baseline.json");
    let json_path = fixture.path().join("output.json");

    // Create baseline with violation
    sloc_guard!()
        .current_dir(fixture.path())
        .args([
            "check",
            "--no-cache",
            "--baseline",
            baseline_path.to_str().unwrap(),
            "--update-baseline",
        ])
        .assert();

    // Check with baseline
    sloc_guard!()
        .current_dir(fixture.path())
        .args([
            "check",
            "--no-cache",
            "--baseline",
            baseline_path.to_str().unwrap(),
            "--write-json",
            json_path.to_str().unwrap(),
        ])
        .assert()
        .success();

    let content = std::fs::read_to_string(&json_path).unwrap();
    let json: serde_json::Value = serde_json::from_str(&content).unwrap();

    assert!(
        json["summary"].get("grandfathered").is_some(),
        "summary must have 'grandfathered'"
    );
    assert!(
        json["summary"]["grandfathered"].is_number(),
        "grandfathered must be a number"
    );
    assert!(
        json["summary"]["grandfathered"].as_u64().unwrap() > 0,
        "grandfathered count should be > 0"
    );
}

// =============================================================================
// Exit Code Tests
// Action expects: 0 = success, 1 = violations, 2 = config error
// =============================================================================

#[test]
fn action_contract_exit_code_0_on_success() {
    let fixture = TestFixture::new();
    fixture.create_config(BASIC_CONFIG_V2);
    fixture.create_rust_file("src/main.rs", 10);

    sloc_guard!()
        .current_dir(fixture.path())
        .args(["check", "--no-cache", "--quiet"])
        .assert()
        .code(0);
}

#[test]
fn action_contract_exit_code_1_on_violation() {
    let fixture = TestFixture::new();
    fixture.create_config(STRICT_CONFIG_V2);
    fixture.create_rust_file("src/large.rs", 50);

    sloc_guard!()
        .current_dir(fixture.path())
        .args(["check", "--no-cache", "--quiet"])
        .assert()
        .code(1);
}

#[test]
fn action_contract_exit_code_2_on_config_error() {
    let fixture = TestFixture::new();
    fixture.create_file(".sloc-guard.toml", "invalid [[[ toml");
    fixture.create_rust_file("src/main.rs", 10);

    sloc_guard!()
        .current_dir(fixture.path())
        .args(["check", "--no-cache"])
        .assert()
        .code(2);
}

// =============================================================================
// Combined Scenario Tests
// Simulate actual action usage patterns
// =============================================================================

#[test]
fn action_contract_full_workflow_passing() {
    // Simulates: action runs check with JSON output, parses summary
    let fixture = TestFixture::new();
    fixture.create_config(BASIC_CONFIG_V2);
    fixture.create_rust_file("src/main.rs", 10);
    fixture.create_rust_file("src/lib.rs", 20);

    let json_path = fixture.path().join("output.json");

    sloc_guard!()
        .current_dir(fixture.path())
        .args([
            "check",
            "--no-cache",
            "--format",
            "text",
            "--color",
            "never",
            "--write-json",
            json_path.to_str().unwrap(),
        ])
        .assert()
        .code(0);

    let content = std::fs::read_to_string(&json_path).unwrap();
    let json: serde_json::Value = serde_json::from_str(&content).unwrap();

    // Verify all summary fields exist and have expected values
    assert_eq!(json["summary"]["total_files"].as_u64().unwrap(), 2);
    assert_eq!(json["summary"]["passed"].as_u64().unwrap(), 2);
    assert_eq!(json["summary"]["failed"].as_u64().unwrap(), 0);
    assert_eq!(json["summary"]["warnings"].as_u64().unwrap(), 0);
    assert_eq!(json["summary"]["grandfathered"].as_u64().unwrap(), 0);
}

#[test]
fn action_contract_full_workflow_with_sarif() {
    // Simulates: action runs check with both JSON and SARIF output
    let fixture = TestFixture::new();
    fixture.create_config(BASIC_CONFIG_V2);
    fixture.create_rust_file("src/main.rs", 10);

    let json_path = fixture.path().join("output.json");
    let sarif_path = fixture.path().join("output.sarif");

    sloc_guard!()
        .current_dir(fixture.path())
        .args([
            "check",
            "--no-cache",
            "--format",
            "text",
            "--write-json",
            json_path.to_str().unwrap(),
            "--write-sarif",
            sarif_path.to_str().unwrap(),
        ])
        .assert()
        .code(0);

    // Both files should exist
    assert!(json_path.exists());
    assert!(sarif_path.exists());

    // SARIF should be valid JSON with required schema
    let sarif_content = std::fs::read_to_string(&sarif_path).unwrap();
    let sarif: serde_json::Value = serde_json::from_str(&sarif_content).unwrap();
    assert!(
        sarif.get("$schema").is_some(),
        "SARIF must have $schema field"
    );
}
