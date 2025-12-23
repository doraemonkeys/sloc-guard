//! Integration tests for the `explain` command.

mod common;

use common::{CONFIG_WITH_RULES, CONFIG_WITH_STRUCTURE_RULES, TestFixture};
use predicates::prelude::*;

// =============================================================================
// Basic Explain Command Tests
// =============================================================================

#[test]
fn explain_file_with_default_rules() {
    let fixture = TestFixture::new();
    fixture.create_config(
        r#"
version = "2"

[scanner]
gitignore = false

[content]
extensions = ["rs"]
max_lines = 100
warn_threshold = 0.8

[structure]
max_files = 10
max_dirs = 5
"#,
    );
    fixture.create_rust_file("src/main.rs", 50);

    sloc_guard!()
        .current_dir(fixture.path())
        .args(["explain", "src/main.rs"])
        .assert()
        .success()
        .stdout(predicate::str::contains("max_lines"))
        .stdout(predicate::str::contains("100"));
}

#[test]
fn explain_file_matching_rule() {
    let fixture = TestFixture::new();
    fixture.create_config(CONFIG_WITH_RULES);
    fixture.create_rust_file("tests/test_main.rs", 50);

    sloc_guard!()
        .current_dir(fixture.path())
        .args(["explain", "tests/test_main.rs"])
        .assert()
        .success()
        .stdout(predicate::str::contains("500")) // Rule limit for tests/**
        .stdout(predicate::str::contains("tests/**"));
}

#[test]
fn explain_directory_structure() {
    let fixture = TestFixture::new();
    fixture.create_config(CONFIG_WITH_STRUCTURE_RULES);
    fixture.create_dir("src/components/Button");
    fixture.create_rust_file("src/components/Button/index.rs", 10);

    sloc_guard!()
        .current_dir(fixture.path())
        .args(["explain", "src/components/Button"])
        .assert()
        .success()
        .stdout(predicate::str::contains("max_files"))
        .stdout(predicate::str::contains("max_dirs"));
}

// =============================================================================
// Output Format Tests
// =============================================================================

#[test]
fn explain_text_format() {
    let fixture = TestFixture::new();
    fixture.create_config(CONFIG_WITH_RULES);
    fixture.create_rust_file("src/main.rs", 50);

    sloc_guard!()
        .current_dir(fixture.path())
        .args(["explain", "src/main.rs", "--format", "text"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Content Rules"));
}

#[test]
fn explain_json_format() {
    let fixture = TestFixture::new();
    fixture.create_config(CONFIG_WITH_RULES);
    fixture.create_rust_file("src/main.rs", 50);

    sloc_guard!()
        .current_dir(fixture.path())
        .args(["explain", "src/main.rs", "--format", "json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("{"))
        .stdout(predicate::str::contains("\"path\""));
}

#[test]
fn explain_json_parseable() {
    let fixture = TestFixture::new();
    fixture.create_config(CONFIG_WITH_RULES);
    fixture.create_rust_file("src/main.rs", 50);

    let output = sloc_guard!()
        .current_dir(fixture.path())
        .args(["explain", "src/main.rs", "--format", "json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json_str = String::from_utf8_lossy(&output);
    let _: serde_json::Value = serde_json::from_str(&json_str).expect("Should be valid JSON");
}

// =============================================================================
// Rule Chain Tests
// =============================================================================

#[test]
fn explain_shows_rule_chain_for_file() {
    let fixture = TestFixture::new();
    fixture.create_config(
        r#"
version = "2"

[scanner]
gitignore = false

[content]
extensions = ["rs"]
max_lines = 100
warn_threshold = 0.8

[[content.rules]]
pattern = "src/**"
max_lines = 200

[[content.rules]]
pattern = "src/core/**"
max_lines = 150

[structure]
max_files = 10
max_dirs = 5
"#,
    );
    fixture.create_rust_file("src/core/engine.rs", 50);

    sloc_guard!()
        .current_dir(fixture.path())
        .args(["explain", "src/core/engine.rs"])
        .assert()
        .success()
        // Should show the rule chain and which one matched
        .stdout(predicate::str::contains("150")); // Last matching rule wins
}

#[test]
fn explain_shows_rule_with_reason() {
    let fixture = TestFixture::new();
    fixture.create_config(
        r#"
version = "2"

[scanner]
gitignore = false

[content]
extensions = ["rs"]
max_lines = 100

[[content.rules]]
pattern = "**/legacy.rs"
max_lines = 500
reason = "Legacy code"

[structure]
max_files = 10
max_dirs = 5
"#,
    );
    fixture.create_rust_file("src/legacy.rs", 50);

    sloc_guard!()
        .current_dir(fixture.path())
        .args(["explain", "src/legacy.rs"])
        .assert()
        .success()
        .stdout(predicate::str::contains("500"))
        .stdout(predicate::str::contains("Legacy code"));
}

// =============================================================================
// Structure Explain Tests
// =============================================================================

#[test]
fn explain_directory_with_structure_rules() {
    let fixture = TestFixture::new();
    fixture.create_config(CONFIG_WITH_STRUCTURE_RULES);
    fixture.create_dir("src/generated");

    sloc_guard!()
        .current_dir(fixture.path())
        .args(["explain", "src/generated"])
        .assert()
        .success()
        .stdout(predicate::str::contains("max_files"))
        .stdout(predicate::str::contains("100")); // From rule for src/generated
}

#[test]
fn explain_directory_with_unlimited_setting() {
    let fixture = TestFixture::new();
    fixture.create_config(CONFIG_WITH_STRUCTURE_RULES);
    fixture.create_dir("src/generated");

    // The config has max_dirs = -1 for src/generated (unlimited)
    sloc_guard!()
        .current_dir(fixture.path())
        .args(["explain", "src/generated"])
        .assert()
        .success()
        .stdout(predicate::str::contains("-1").or(predicate::str::contains("unlimited")));
}

// =============================================================================
// Custom Config Path Tests
// =============================================================================

#[test]
fn explain_with_custom_config() {
    let fixture = TestFixture::new();
    fixture.create_file(
        "custom.toml",
        r#"
version = "2"

[scanner]
gitignore = false

[content]
extensions = ["rs"]
max_lines = 999

[structure]
max_files = 10
max_dirs = 5
"#,
    );
    fixture.create_rust_file("src/main.rs", 50);

    sloc_guard!()
        .current_dir(fixture.path())
        .args(["explain", "src/main.rs", "--config", "custom.toml"])
        .assert()
        .success()
        .stdout(predicate::str::contains("999"));
}

// =============================================================================
// Edge Cases
// =============================================================================

#[test]
fn explain_nonexistent_file_returns_error() {
    let fixture = TestFixture::new();
    fixture.create_config(CONFIG_WITH_RULES);

    // Explain requires the path to exist
    sloc_guard!()
        .current_dir(fixture.path())
        .args(["explain", "src/future_file.rs"])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("not found").or(predicate::str::contains("Path")));
}

#[test]
fn explain_file_outside_extensions_filter() {
    let fixture = TestFixture::new();
    fixture.create_config(CONFIG_WITH_RULES);
    fixture.create_file("src/config.yaml", "key: value\n");

    // YAML is not in extensions, should still explain what would apply
    sloc_guard!()
        .current_dir(fixture.path())
        .args(["explain", "src/config.yaml"])
        .assert()
        .success();
}

#[test]
fn explain_deep_nested_path() {
    let fixture = TestFixture::new();
    fixture.create_config(CONFIG_WITH_RULES);
    fixture.create_rust_file("src/deep/nested/path/to/file.rs", 10);

    sloc_guard!()
        .current_dir(fixture.path())
        .args(["explain", "src/deep/nested/path/to/file.rs"])
        .assert()
        .success()
        .stdout(predicate::str::contains("max_lines"));
}

// =============================================================================
// No Config Mode Tests
// =============================================================================

#[test]
fn explain_no_config_uses_defaults() {
    let fixture = TestFixture::new();
    fixture.create_rust_file("src/main.rs", 50);

    sloc_guard!()
        .current_dir(fixture.path())
        .args(["explain", "src/main.rs", "--no-config"])
        .assert()
        .success()
        .stdout(predicate::str::contains("max_lines"));
}
