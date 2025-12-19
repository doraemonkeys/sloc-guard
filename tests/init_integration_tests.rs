//! Integration tests for the `init` command.

mod common;

use common::TestFixture;
use predicates::prelude::*;

// =============================================================================
// Basic Init Command Tests
// =============================================================================

#[test]
fn init_creates_default_config_file() {
    let fixture = TestFixture::new();

    sloc_guard!()
        .current_dir(fixture.path())
        .args(["init"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Created configuration file"));

    let config_path = fixture.path().join(".sloc-guard.toml");
    assert!(config_path.exists());

    let content = std::fs::read_to_string(&config_path).unwrap();
    assert!(content.contains("max_lines"));
    assert!(content.contains("extensions"));
}

#[test]
fn init_creates_config_at_custom_path() {
    let fixture = TestFixture::new();

    let custom_path = fixture.path().join("custom-config.toml");

    sloc_guard!()
        .current_dir(fixture.path())
        .args(["init", "--output", custom_path.to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("Created configuration file"));

    assert!(custom_path.exists());
}

#[test]
fn init_fails_if_config_exists() {
    let fixture = TestFixture::new();
    fixture.create_file(".sloc-guard.toml", "# existing config\n");

    sloc_guard!()
        .current_dir(fixture.path())
        .args(["init"])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("already exists"));
}

#[test]
fn init_force_overwrites_existing_config() {
    let fixture = TestFixture::new();
    fixture.create_file(".sloc-guard.toml", "# old config\nmax_lines = 999\n");

    sloc_guard!()
        .current_dir(fixture.path())
        .args(["init", "--force"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Created configuration file"));

    let config_path = fixture.path().join(".sloc-guard.toml");
    let content = std::fs::read_to_string(&config_path).unwrap();
    // Should be the default template, not the old content
    assert!(content.contains("[default]"));
    assert!(!content.contains("max_lines = 999"));
}

#[test]
fn init_fails_without_parent_directories() {
    let fixture = TestFixture::new();

    let nested_path = fixture.path().join("config/nested/.sloc-guard.toml");

    // Init does not create parent directories, so this should fail
    sloc_guard!()
        .current_dir(fixture.path())
        .args(["init", "--output", nested_path.to_str().unwrap()])
        .assert()
        .code(2);
}

#[test]
fn init_succeeds_with_existing_parent_directory() {
    let fixture = TestFixture::new();
    fixture.create_dir("config");

    let nested_path = fixture.path().join("config/.sloc-guard.toml");

    sloc_guard!()
        .current_dir(fixture.path())
        .args(["init", "--output", nested_path.to_str().unwrap()])
        .assert()
        .success();

    assert!(nested_path.exists());
}

// =============================================================================
// Config Template Content Tests
// =============================================================================

#[test]
fn init_template_contains_required_sections() {
    let fixture = TestFixture::new();

    sloc_guard!()
        .current_dir(fixture.path())
        .args(["init"])
        .assert()
        .success();

    let config_path = fixture.path().join(".sloc-guard.toml");
    let content = std::fs::read_to_string(&config_path).unwrap();

    // Check for essential configuration options
    assert!(content.contains("max_lines"));
    assert!(content.contains("extensions"));
    assert!(content.contains("skip_comments"));
    assert!(content.contains("skip_blank"));
    assert!(content.contains("warn_threshold"));
}

#[test]
fn init_template_is_valid_toml() {
    let fixture = TestFixture::new();

    sloc_guard!()
        .current_dir(fixture.path())
        .args(["init"])
        .assert()
        .success();

    let config_path = fixture.path().join(".sloc-guard.toml");
    let content = std::fs::read_to_string(&config_path).unwrap();

    // Should be parseable as TOML (will panic if invalid)
    let _: toml::Value = toml::from_str(&content).expect("Generated config should be valid TOML");
}

#[test]
fn init_created_config_can_be_used_by_check() {
    let fixture = TestFixture::new();
    fixture.create_rust_file("src/main.rs", 10);

    // Initialize config
    sloc_guard!()
        .current_dir(fixture.path())
        .args(["init"])
        .assert()
        .success();

    // Check should work with the generated config
    sloc_guard!()
        .current_dir(fixture.path())
        .args(["check", "--no-cache", "--quiet"])
        .assert()
        .success();
}
