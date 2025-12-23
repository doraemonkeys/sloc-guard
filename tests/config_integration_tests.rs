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
[default]
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
[default]
max_lines = 500
extensions = ["rs"]

[exclude]
patterns = ["[invalid"]
"#,
    );

    sloc_guard!()
        .current_dir(fixture.path())
        .args(["config", "validate"])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("pattern"));
}

#[test]
fn config_validate_empty_override_path() {
    let fixture = TestFixture::new();
    fixture.create_file(
        ".sloc-guard.toml",
        r#"
[default]
max_lines = 500
extensions = ["rs"]

[[override]]
path = ""
max_lines = 1000
"#,
    );

    sloc_guard!()
        .current_dir(fixture.path())
        .args(["config", "validate"])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("path"));
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
fn config_show_with_rules() {
    let fixture = TestFixture::new();
    fixture.create_file(
        ".sloc-guard.toml",
        r#"
[default]
max_lines = 500
extensions = ["rs", "go"]

[rules.rust]
extensions = ["rs"]
max_lines = 300

[rules.golang]
extensions = ["go"]
max_lines = 400
"#,
    );

    sloc_guard!()
        .current_dir(fixture.path())
        .args(["config", "show"])
        .assert()
        .success()
        .stdout(predicate::str::contains("[rules.rust]"))
        .stdout(predicate::str::contains("[rules.golang]"));
}

#[test]
fn config_show_with_overrides() {
    let fixture = TestFixture::new();
    fixture.create_file(
        ".sloc-guard.toml",
        r#"
[default]
max_lines = 500
extensions = ["rs"]

[[override]]
path = "src/legacy.rs"
max_lines = 1000
reason = "Legacy code"
"#,
    );

    sloc_guard!()
        .current_dir(fixture.path())
        .args(["config", "show"])
        .assert()
        .success()
        .stdout(predicate::str::contains("[[override]]  # DEPRECATED"))
        .stdout(predicate::str::contains("src/legacy.rs"))
        .stdout(predicate::str::contains("Legacy code"));
}

#[test]
fn config_show_with_exclude_patterns() {
    let fixture = TestFixture::new();
    fixture.create_file(
        ".sloc-guard.toml",
        r#"
[default]
max_lines = 500
extensions = ["rs"]

[exclude]
patterns = ["**/target/**", "**/vendor/**"]
"#,
    );

    sloc_guard!()
        .current_dir(fixture.path())
        .args(["config", "show"])
        .assert()
        .success()
        .stdout(predicate::str::contains("[exclude]"))
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
