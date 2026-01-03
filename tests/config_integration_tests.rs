//! Integration tests for the `config` command.

mod common;

use common::{BASIC_CONFIG_V2, TestFixture};
use predicates::prelude::*;

// =============================================================================
// Config Validate Tests
// =============================================================================

#[test]
fn config_validate_valid_config() {
    let fixture = TestFixture::new();
    fixture.create_config(BASIC_CONFIG_V2);

    sloc_guard!()
        .current_dir(fixture.path())
        .args(["config", "validate"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Configuration is valid"));
}

#[test]
fn config_validate_custom_path() {
    let fixture = TestFixture::new();
    fixture.create_file("custom.toml", BASIC_CONFIG_V2);

    sloc_guard!()
        .current_dir(fixture.path())
        .args(["config", "validate", "--config", "custom.toml"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Configuration is valid"));
}

#[test]
fn config_validate_invalid_toml_syntax() {
    let fixture = TestFixture::new();
    fixture.create_file(".sloc-guard.toml", "invalid [[[ toml");

    sloc_guard!()
        .current_dir(fixture.path())
        .args(["config", "validate"])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("error"));
}

#[test]
fn config_validate_missing_file() {
    let fixture = TestFixture::new();

    sloc_guard!()
        .current_dir(fixture.path())
        .args(["config", "validate"])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("not found"));
}

#[test]
fn config_validate_invalid_warn_threshold() {
    let fixture = TestFixture::new();
    fixture.create_file(
        ".sloc-guard.toml",
        r#"
version = "2"

[content]
max_lines = 500
extensions = ["rs"]
warn_threshold = 1.5
"#,
    );

    sloc_guard!()
        .current_dir(fixture.path())
        .args(["config", "validate"])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("warn_threshold"));
}

#[test]
fn config_validate_invalid_glob_pattern() {
    let fixture = TestFixture::new();
    fixture.create_file(
        ".sloc-guard.toml",
        r#"
version = "2"

[scanner]
exclude = ["[invalid"]
"#,
    );

    sloc_guard!()
        .current_dir(fixture.path())
        .args(["config", "validate"])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("InvalidPattern"));
}

// =============================================================================
// Config Show Tests
// =============================================================================

#[test]
fn config_show_text_format() {
    let fixture = TestFixture::new();
    fixture.create_config(BASIC_CONFIG_V2);

    sloc_guard!()
        .current_dir(fixture.path())
        .args(["config", "show"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Effective Configuration"))
        .stdout(predicate::str::contains("max_lines"));
}

#[test]
fn config_show_json_format() {
    let fixture = TestFixture::new();
    fixture.create_config(BASIC_CONFIG_V2);

    sloc_guard!()
        .current_dir(fixture.path())
        .args(["config", "show", "--format", "json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("{"))
        .stdout(predicate::str::contains("\"content\""));
}

#[test]
fn config_show_custom_path() {
    let fixture = TestFixture::new();
    fixture.create_file("my-config.toml", BASIC_CONFIG_V2);

    sloc_guard!()
        .current_dir(fixture.path())
        .args(["config", "show", "--config", "my-config.toml"])
        .assert()
        .success()
        .stdout(predicate::str::contains("max_lines"));
}

#[test]
fn config_show_with_content_rules() {
    let fixture = TestFixture::new();
    fixture.create_file(
        ".sloc-guard.toml",
        r#"
version = "2"

[content]
max_lines = 500
extensions = ["rs", "go"]

[[content.rules]]
pattern = "**/*.rs"
max_lines = 300

[[content.rules]]
pattern = "**/*.go"
max_lines = 400
"#,
    );

    sloc_guard!()
        .current_dir(fixture.path())
        .args(["config", "show"])
        .assert()
        .success()
        .stdout(predicate::str::contains("[[content.rules]]"))
        .stdout(predicate::str::contains("**/*.rs"))
        .stdout(predicate::str::contains("**/*.go"));
}

#[test]
fn config_show_with_scanner_exclude_patterns() {
    let fixture = TestFixture::new();
    fixture.create_file(
        ".sloc-guard.toml",
        r#"
version = "2"

[content]
max_lines = 500
extensions = ["rs"]

[scanner]
exclude = ["**/target/**", "**/vendor/**"]
"#,
    );

    sloc_guard!()
        .current_dir(fixture.path())
        .args(["config", "show"])
        .assert()
        .success()
        .stdout(predicate::str::contains("[scanner]"))
        .stdout(predicate::str::contains("target"))
        .stdout(predicate::str::contains("vendor"));
}

#[test]
fn config_show_missing_file_uses_defaults() {
    let fixture = TestFixture::new();

    sloc_guard!()
        .current_dir(fixture.path())
        .args(["config", "show"])
        .assert()
        .success()
        .stdout(predicate::str::contains("max_lines"));
}

// =============================================================================
// Config JSON Output Structure Tests
// =============================================================================

#[test]
fn config_show_json_parseable() {
    let fixture = TestFixture::new();
    fixture.create_config(BASIC_CONFIG_V2);

    let output = sloc_guard!()
        .current_dir(fixture.path())
        .args(["config", "show", "--format", "json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json_str = String::from_utf8_lossy(&output);
    let _: serde_json::Value = serde_json::from_str(&json_str).expect("Should be valid JSON");
}

#[test]
fn config_show_json_contains_expected_fields() {
    let fixture = TestFixture::new();
    fixture.create_config(BASIC_CONFIG_V2);

    let output = sloc_guard!()
        .current_dir(fixture.path())
        .args(["config", "show", "--format", "json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json_str = String::from_utf8_lossy(&output);
    let json: serde_json::Value = serde_json::from_str(&json_str).unwrap();

    // V2 config uses "content" section instead of "default"
    assert!(json["content"].is_object());
    assert!(json["content"]["max_lines"].is_number());
    assert!(json["content"]["extensions"].is_array());
}

// =============================================================================
// Extends Policy Tests
// =============================================================================

#[test]
fn extends_policy_refresh_flag_is_recognized() {
    let fixture = TestFixture::new();
    fixture.create_config(BASIC_CONFIG_V2);

    // Test that --extends-policy=refresh is recognized and works with local config
    sloc_guard!()
        .current_dir(fixture.path())
        .args(["config", "show", "--extends-policy=refresh"])
        .assert()
        .success()
        .stdout(predicate::str::contains("max_lines"));
}

#[test]
fn extends_policy_refresh_with_preset() {
    let fixture = TestFixture::new();
    fixture.create_file(
        ".sloc-guard.toml",
        r#"
version = "2"
extends = "preset:rust-strict"

[content]
max_lines = 600
"#,
    );

    // ForceRefresh policy with a preset should work (presets don't use cache but share the code path)
    sloc_guard!()
        .current_dir(fixture.path())
        .args(["config", "show", "--extends-policy=refresh"])
        .assert()
        .success()
        .stdout(predicate::str::contains("max_lines"));
}

#[test]
fn extends_policy_offline_with_local_extends() {
    let fixture = TestFixture::new();

    // Create a base config
    fixture.create_file(
        "base.toml",
        r#"
version = "2"

[content]
max_lines = 200
extensions = ["rs"]
"#,
    );

    // Create main config that extends from local file
    fixture.create_file(
        ".sloc-guard.toml",
        r#"
version = "2"
extends = "base.toml"

[content]
max_lines = 400
"#,
    );

    // Offline policy should work with local extends (no network needed)
    sloc_guard!()
        .current_dir(fixture.path())
        .args(["config", "show", "--extends-policy=offline"])
        .assert()
        .success()
        .stdout(predicate::str::contains("max_lines"));
}

#[test]
fn extends_policy_normal_is_default() {
    let fixture = TestFixture::new();
    fixture.create_file(
        ".sloc-guard.toml",
        r#"
version = "2"
extends = "preset:rust-strict"
"#,
    );

    // Without --extends-policy flag, should use normal policy
    sloc_guard!()
        .current_dir(fixture.path())
        .args(["config", "show"])
        .assert()
        .success()
        .stdout(predicate::str::contains("max_lines"));
}

#[test]
fn extends_policy_refresh_with_check_command() {
    let fixture = TestFixture::new();
    fixture.create_file(
        ".sloc-guard.toml",
        r#"
version = "2"
extends = "preset:rust-strict"

[content]
max_lines = 1000
"#,
    );
    fixture.create_rust_file("src/main.rs", 10);

    // Check command with --extends-policy=refresh should work
    sloc_guard!()
        .current_dir(fixture.path())
        .args(["check", "--extends-policy=refresh"])
        .assert()
        .success();
}
