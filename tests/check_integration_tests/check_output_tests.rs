//! Output format tests - json, sarif, markdown, html, file output, report-json.

use crate::common::{BASIC_CONFIG_V2, TestFixture};
use crate::sloc_guard;
use predicates::prelude::*;

// =============================================================================
// Output Format Tests
// =============================================================================

#[test]
fn check_json_output_format() {
    let fixture = TestFixture::new();
    fixture.create_config(BASIC_CONFIG_V2);
    fixture.create_rust_file("src/main.rs", 10);

    sloc_guard!()
        .current_dir(fixture.path())
        .args(["check", "--no-sloc-cache", "--format", "json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"summary\""))
        .stdout(predicate::str::contains("\"results\""));
}

#[test]
fn nested_invocation_has_same_rules_and_logical_paths_as_project_root() {
    let fixture = TestFixture::new();
    fixture.create_config(
        r#"
version = "2"

[scanner]
gitignore = false
exclude = ["core/generated/**"]

[content]
extensions = ["rs"]
max_lines = 50

[[content.rules]]
pattern = "core/**"
max_lines = 200
"#,
    );
    fixture.create_rust_file("core/session/receive_test.rs", 100);
    fixture.create_rust_file("core/generated/ignored.rs", 100);

    let root_output = sloc_guard!()
        .current_dir(fixture.path())
        .args(["check", "core", "--no-sloc-cache", "--format", "json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let nested_output = sloc_guard!()
        .current_dir(fixture.path().join("core"))
        .args(["check", ".", "--no-sloc-cache", "--format", "json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let root_json: serde_json::Value = serde_json::from_slice(&root_output).unwrap();
    let nested_json: serde_json::Value = serde_json::from_slice(&nested_output).unwrap();
    assert_eq!(nested_json, root_json);
    assert_eq!(
        root_json["results"][0]["path"],
        "core/session/receive_test.rs"
    );
    assert_eq!(root_json["results"][0]["limit"], 200);
}

#[test]
fn nested_invocation_preserves_project_relative_structure_depth() {
    let fixture = TestFixture::new();
    fixture.create_config(
        r#"
version = "2"

[scanner]
gitignore = false

[content]
extensions = ["rs"]
max_lines = 500

[structure]
max_depth = 1
"#,
    );
    fixture.create_rust_file("core/session/receive.rs", 1);

    let root_output = sloc_guard!()
        .current_dir(fixture.path())
        .args(["check", "core", "--no-sloc-cache", "--format", "json"])
        .assert()
        .code(1)
        .get_output()
        .stdout
        .clone();
    let nested_output = sloc_guard!()
        .current_dir(fixture.path().join("core"))
        .args(["check", ".", "--no-sloc-cache", "--format", "json"])
        .assert()
        .code(1)
        .get_output()
        .stdout
        .clone();

    let root_json: serde_json::Value = serde_json::from_slice(&root_output).unwrap();
    let nested_json: serde_json::Value = serde_json::from_slice(&nested_output).unwrap();
    assert_eq!(nested_json, root_json);
    assert!(
        root_json["results"]
            .as_array()
            .unwrap()
            .iter()
            .any(|result| {
                result["path"] == "core/session"
                    && result["violation_category"]["violation_type"]["type"] == "max_depth"
            })
    );
}

#[test]
fn root_structure_rule_uses_dot_identity() {
    let fixture = TestFixture::new();
    fixture.create_config(
        r#"
version = "2"

[scanner]
gitignore = false

[content]
extensions = ["rs"]
max_lines = 500

[structure]

[[structure.rules]]
scope = "."
max_files = 0
"#,
    );
    fixture.create_rust_file("main.rs", 1);

    let output = sloc_guard!()
        .current_dir(fixture.path())
        .args(["check", "--no-sloc-cache", "--format", "json"])
        .assert()
        .code(1)
        .get_output()
        .stdout
        .clone();
    let json: serde_json::Value = serde_json::from_slice(&output).unwrap();

    assert!(json["results"].as_array().unwrap().iter().any(|result| {
        result["path"] == "."
            && result["violation_category"]["violation_type"]["type"] == "file_count"
    }));
}

#[test]
fn enabling_structure_checks_does_not_broaden_scanner_excludes() {
    let fixture = TestFixture::new();
    fixture.create_config(
        r#"
version = "2"

[scanner]
gitignore = false
exclude = ["vendor/**"]

[content]
extensions = ["rs"]
max_lines = 5

[structure]
max_files = 100
"#,
    );
    fixture.create_rust_file("vendor/ignored.rs", 20);
    fixture.create_rust_file("apps/vendor/large.rs", 20);

    sloc_guard!()
        .current_dir(fixture.path())
        .args(["check", "--no-sloc-cache", "--format", "json"])
        .assert()
        .code(1)
        .stdout(predicate::str::contains("apps/vendor/large.rs"))
        .stdout(predicate::str::contains("vendor/ignored.rs").not());
}

#[test]
fn nested_invocation_applies_directory_rules_to_the_same_logical_directory() {
    let fixture = TestFixture::new();
    fixture.create_config(
        r#"
version = "2"

[scanner]
gitignore = false

[content]
extensions = ["rs"]
max_lines = 500

[structure]

[[structure.rules]]
scope = "core"
allow_dirs = ["session"]
"#,
    );
    fixture.create_rust_file("core/session/file.rs", 1);

    let root_output = sloc_guard!()
        .current_dir(fixture.path())
        .args(["check", "core", "--no-sloc-cache", "--format", "json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let nested_output = sloc_guard!()
        .current_dir(fixture.path().join("core"))
        .args(["check", ".", "--no-sloc-cache", "--format", "json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let root_json: serde_json::Value = serde_json::from_slice(&root_output).unwrap();
    let nested_json: serde_json::Value = serde_json::from_slice(&nested_output).unwrap();
    assert_eq!(nested_json, root_json);
}

#[test]
fn check_sarif_output_format() {
    let fixture = TestFixture::new();
    fixture.create_config(BASIC_CONFIG_V2);
    fixture.create_rust_file("src/main.rs", 10);

    sloc_guard!()
        .current_dir(fixture.path())
        .args(["check", "--no-sloc-cache", "--format", "sarif"])
        .assert()
        .success()
        .stdout(predicate::str::contains("$schema"))
        .stdout(predicate::str::contains("2.1.0"));
}

#[test]
fn check_markdown_output_format() {
    let fixture = TestFixture::new();
    fixture.create_config(BASIC_CONFIG_V2);
    fixture.create_rust_file("src/main.rs", 10);

    sloc_guard!()
        .current_dir(fixture.path())
        .args(["check", "--no-sloc-cache", "--format", "markdown"])
        .assert()
        .success()
        .stdout(predicate::str::contains("## SLOC Guard Results"));
}

#[test]
fn check_html_output_format() {
    let fixture = TestFixture::new();
    fixture.create_config(BASIC_CONFIG_V2);
    fixture.create_rust_file("src/main.rs", 10);

    sloc_guard!()
        .current_dir(fixture.path())
        .args(["check", "--no-sloc-cache", "--format", "html"])
        .assert()
        .success()
        .stdout(predicate::str::contains("<!DOCTYPE html>"))
        .stdout(predicate::str::contains("SLOC Guard"));
}

#[test]
fn check_output_to_file() {
    let fixture = TestFixture::new();
    fixture.create_config(BASIC_CONFIG_V2);
    fixture.create_rust_file("src/main.rs", 10);

    let output_path = fixture.path().join("report.json");

    sloc_guard!()
        .current_dir(fixture.path())
        .args([
            "check",
            "--no-sloc-cache",
            "--format",
            "json",
            "--output",
            output_path.to_str().unwrap(),
        ])
        .assert()
        .success();

    assert!(output_path.exists());
    let content = std::fs::read_to_string(&output_path).unwrap();
    assert!(content.contains("\"summary\""));
}

// =============================================================================
// Report JSON Tests
// =============================================================================

#[test]
fn check_report_json_creates_stats_file() {
    let fixture = TestFixture::new();
    fixture.create_config(BASIC_CONFIG_V2);
    fixture.create_rust_file("src/main.rs", 30);
    fixture.create_rust_file("src/lib.rs", 20);

    let stats_path = fixture.path().join("stats.json");

    sloc_guard!()
        .current_dir(fixture.path())
        .args([
            "check",
            "--no-sloc-cache",
            "--quiet",
            "--report-json",
            stats_path.to_str().unwrap(),
        ])
        .assert()
        .success();

    assert!(stats_path.exists());
    let content = std::fs::read_to_string(&stats_path).unwrap();
    assert!(content.contains("\"total_files\""));
    assert!(content.contains("\"by_language\""));
}
