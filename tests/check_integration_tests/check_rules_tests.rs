//! Content rules, structure checks, and path normalization tests.

use crate::common::{BASIC_CONFIG_V2, CONFIG_WITH_RULES, STRICT_CONFIG_V2, TestFixture};
use crate::sloc_guard;
use predicates::prelude::*;

// =============================================================================
// Content Rules Tests
// =============================================================================

#[test]
fn check_content_rules_apply_pattern_limits() {
    let fixture = TestFixture::new();
    fixture.create_config(CONFIG_WITH_RULES);
    // Test file uses higher limit (500 lines) - should pass
    fixture.create_rust_file("tests/test_main.rs", 150);

    // Only check tests directory which has higher limit
    sloc_guard!()
        .current_dir(fixture.path())
        .args(["check", "--no-sloc-cache", "--quiet", "--include", "tests"])
        .assert()
        .success();

    // Test file exceeding test rule limit should fail
    fixture.create_rust_file("tests/test_large.rs", 600);

    sloc_guard!()
        .current_dir(fixture.path())
        .args(["check", "--no-sloc-cache", "--quiet", "--include", "tests"])
        .assert()
        .code(1);
}

// =============================================================================
// Structure Check Tests
// =============================================================================

#[test]
fn check_structure_max_files_violation() {
    let fixture = TestFixture::new();
    fixture.create_config(STRICT_CONFIG_V2); // max_files = 2
    fixture.create_rust_file("src/file1.rs", 5);
    fixture.create_rust_file("src/file2.rs", 5);
    fixture.create_rust_file("src/file3.rs", 5); // Exceeds limit

    sloc_guard!()
        .current_dir(fixture.path())
        .args(["check", "src", "--no-sloc-cache", "--quiet"])
        .assert()
        .code(1);
}

#[test]
fn check_structure_max_dirs_violation() {
    let fixture = TestFixture::new();
    fixture.create_config(STRICT_CONFIG_V2); // max_dirs = 1
    fixture.create_dir("src/sub1");
    fixture.create_dir("src/sub2"); // Exceeds limit
    fixture.create_rust_file("src/main.rs", 5);

    sloc_guard!()
        .current_dir(fixture.path())
        .args(["check", "src", "--no-sloc-cache", "--quiet"])
        .assert()
        .code(1);
}

#[test]
fn check_structure_cli_override() {
    let fixture = TestFixture::new();
    fixture.create_config(BASIC_CONFIG_V2);
    // Create 15 files (exceeds default max_files=10)
    for i in 0..15 {
        fixture.create_rust_file(&format!("src/file{i}.rs"), 5);
    }

    // Without override, should fail
    sloc_guard!()
        .current_dir(fixture.path())
        .args(["check", "src", "--no-sloc-cache", "--quiet"])
        .assert()
        .code(1);

    // With CLI override to allow more files
    sloc_guard!()
        .current_dir(fixture.path())
        .args([
            "check",
            "src",
            "--no-sloc-cache",
            "--quiet",
            "--max-files",
            "20",
        ])
        .assert()
        .success();
}

#[test]
fn check_structure_allowlist_violation() {
    let fixture = TestFixture::new();
    // Config with allowlist rule: only .rs files allowed in src
    fixture.create_config(
        r#"
version = "2"

[scanner]
gitignore = false
exclude = []

[content]
max_lines = 100
extensions = ["rs"]

[[structure.rules]]
scope = "**/src"
allow_extensions = [".rs"]
"#,
    );
    fixture.create_rust_file("src/main.rs", 5);
    // Create a disallowed file
    fixture.create_file("src/config.json", "{}");

    sloc_guard!()
        .current_dir(fixture.path())
        .args(["check", "src", "--no-sloc-cache"])
        .assert()
        .code(1)
        .stdout(predicate::str::contains("disallowed file"));
}

#[test]
fn check_structure_global_allowlist_violation() {
    let fixture = TestFixture::new();
    // Global allowlist: only .rs files allowed anywhere
    fixture.create_config(
        r#"
version = "2"

[scanner]
gitignore = false
exclude = []

[content]
max_lines = 100
extensions = ["rs"]

[structure]
allow_extensions = [".rs"]
"#,
    );
    fixture.create_rust_file("src/main.rs", 5);
    fixture.create_file("src/config.json", "{}");

    sloc_guard!()
        .current_dir(fixture.path())
        .args(["check", "src", "--no-sloc-cache"])
        .assert()
        .code(1)
        .stdout(predicate::str::contains("disallowed file"));
}

#[test]
fn check_structure_global_deny_extension_violation() {
    let fixture = TestFixture::new();
    // Global denylist: deny *.json anywhere
    fixture.create_config(
        r#"
version = "2"

[scanner]
gitignore = false
exclude = []

[content]
max_lines = 100
extensions = ["rs"]

[structure]
deny_extensions = [".json"]
"#,
    );
    fixture.create_rust_file("src/main.rs", 5);
    fixture.create_file("src/config.json", "{}");

    sloc_guard!()
        .current_dir(fixture.path())
        .args(["check", "src", "--no-sloc-cache"])
        .assert()
        .code(1)
        .stdout(predicate::str::contains("denied file"));
}

// =============================================================================
// Path Normalization Tests
// =============================================================================

/// Tests that paths with "./" prefix match the same content rules as plain paths.
#[test]
fn check_path_normalization_dot_slash_matches_rules() {
    let fixture = TestFixture::new();
    // Config with a rule that gives test files a higher limit
    fixture.create_config(
        r#"
version = "2"

[scanner]
gitignore = false
exclude = []

[content]
extensions = ["rs"]
max_lines = 50

[[content.rules]]
pattern = "src/**/*_tests.rs"
max_lines = 200
"#,
    );
    // Create a test file with 100 lines (exceeds default 50, but under rule 200)
    fixture.create_rust_file("src/cache/cache_tests.rs", 100);

    // Check with plain path - should pass (matches rule with 200 limit)
    sloc_guard!()
        .current_dir(fixture.path())
        .args([
            "check",
            "--no-sloc-cache",
            "--quiet",
            "src/cache/cache_tests.rs",
        ])
        .assert()
        .success();

    // Check with "./" prefix - should also pass (same rule should apply)
    sloc_guard!()
        .current_dir(fixture.path())
        .args([
            "check",
            "--no-sloc-cache",
            "--quiet",
            "./src/cache/cache_tests.rs",
        ])
        .assert()
        .success();
}

/// Tests that paths with ".\" prefix (Windows style) match the same content rules.
#[test]
fn check_path_normalization_dot_backslash_matches_rules() {
    let fixture = TestFixture::new();
    fixture.create_config(
        r#"
version = "2"

[scanner]
gitignore = false
exclude = []

[content]
extensions = ["rs"]
max_lines = 50

[[content.rules]]
pattern = "tests/**"
max_lines = 300
"#,
    );
    // Create a test file with 150 lines (exceeds default 50, but under rule 300)
    fixture.create_rust_file("tests/integration.rs", 150);

    // Check with backslash path - should pass (matches rule with 300 limit)
    sloc_guard!()
        .current_dir(fixture.path())
        .args([
            "check",
            "--no-sloc-cache",
            "--quiet",
            ".\\tests\\integration.rs",
        ])
        .assert()
        .success();

    // Check with forward slash for comparison - should also pass
    sloc_guard!()
        .current_dir(fixture.path())
        .args([
            "check",
            "--no-sloc-cache",
            "--quiet",
            "tests/integration.rs",
        ])
        .assert()
        .success();
}

/// Tests that without path normalization, files would fail (proves the rule is effective).
#[test]
fn check_path_normalization_rule_is_effective() {
    let fixture = TestFixture::new();
    fixture.create_config(
        r#"
version = "2"

[scanner]
gitignore = false
exclude = []

[content]
extensions = ["rs"]
max_lines = 50

[[content.rules]]
pattern = "src/**/*_tests.rs"
max_lines = 200
"#,
    );
    // Create a non-test file with 100 lines (exceeds default 50, no matching rule)
    fixture.create_rust_file("src/cache/mod.rs", 100);

    // Non-test file should fail (no rule match, uses default 50 limit)
    sloc_guard!()
        .current_dir(fixture.path())
        .args(["check", "--no-sloc-cache", "--quiet", "src/cache/mod.rs"])
        .assert()
        .code(1);

    // Same with "./" prefix - should also fail
    sloc_guard!()
        .current_dir(fixture.path())
        .args(["check", "--no-sloc-cache", "--quiet", "./src/cache/mod.rs"])
        .assert()
        .code(1);
}

/// Tests checking from a subdirectory with relative paths.
///
/// When running from project root with path `src/cache/cache_tests.rs`:
/// 1. Pattern `src/**/*_tests.rs` matches → uses 200 limit instead of 50
///
/// Verification: Creates two files with same line count (100 lines):
/// - `src/cache/cache_tests.rs` - matches rule pattern → PASS (100 < 200)
/// - `src/cache/mod.rs` - no rule match → FAIL (100 > 50 default)
#[test]
fn check_from_subdirectory_with_relative_paths() {
    let fixture = TestFixture::new();
    fixture.create_config(
        r#"
version = "2"

[scanner]
gitignore = false
exclude = []

[content]
extensions = ["rs"]
max_lines = 50

[[content.rules]]
pattern = "src/**/*_tests.rs"
max_lines = 200
"#,
    );
    fixture.create_rust_file("src/cache/cache_tests.rs", 100);
    // Also create a non-test file with same lines to prove the rule matters
    fixture.create_rust_file("src/cache/mod.rs", 100);

    // Test file should pass - matches rule with 200 limit
    sloc_guard!()
        .current_dir(fixture.path())
        .args([
            "check",
            "--no-sloc-cache",
            "--quiet",
            "src/cache/cache_tests.rs",
        ])
        .assert()
        .success();

    // Verification: Non-test file with same lines should FAIL (default 50 limit)
    // This proves the test file passed because of the rule, not a bug
    sloc_guard!()
        .current_dir(fixture.path())
        .args(["check", "--no-sloc-cache", "--quiet", "src/cache/mod.rs"])
        .assert()
        .code(1);

    // Also verify "./" prefix works consistently
    sloc_guard!()
        .current_dir(fixture.path())
        .args([
            "check",
            "--no-sloc-cache",
            "--quiet",
            "./src/cache/cache_tests.rs",
        ])
        .assert()
        .success();
}

/// Tests that content.exclude patterns also respect path normalization.
#[test]
fn check_path_normalization_exclude_patterns() {
    let fixture = TestFixture::new();
    fixture.create_config(
        r#"
version = "2"

[scanner]
gitignore = false
exclude = []

[content]
extensions = ["rs"]
max_lines = 50
exclude = ["**/generated/**"]
"#,
    );
    // Create a file in excluded directory with 100 lines (would exceed limit)
    fixture.create_rust_file("src/generated/types.rs", 100);
    // Create a normal file that passes
    fixture.create_rust_file("src/main.rs", 10);

    // Excluded path with "./" prefix - should be excluded and pass
    sloc_guard!()
        .current_dir(fixture.path())
        .args([
            "check",
            "--no-sloc-cache",
            "--quiet",
            "./src/generated/types.rs",
        ])
        .assert()
        .success();

    // Excluded path with backslash - should also be excluded and pass
    sloc_guard!()
        .current_dir(fixture.path())
        .args([
            "check",
            "--no-sloc-cache",
            "--quiet",
            ".\\src\\generated\\types.rs",
        ])
        .assert()
        .success();
}

/// Tests checking entire directory with mixed path formats.
#[test]
fn check_directory_with_normalized_paths() {
    let fixture = TestFixture::new();
    fixture.create_config(
        r#"
version = "2"

[scanner]
gitignore = false
exclude = []

[content]
extensions = ["rs"]
max_lines = 50

[[content.rules]]
pattern = "src/**/*_tests.rs"
max_lines = 200
"#,
    );
    // Test file with 100 lines - should pass due to rule
    fixture.create_rust_file("src/cache/cache_tests.rs", 100);
    // Regular file with 30 lines - should pass due to default limit
    fixture.create_rust_file("src/lib.rs", 30);

    // Check directory with "./" prefix
    sloc_guard!()
        .current_dir(fixture.path())
        .args(["check", "--no-sloc-cache", "--quiet", "./src"])
        .assert()
        .success();

    // Check directory with backslash
    sloc_guard!()
        .current_dir(fixture.path())
        .args(["check", "--no-sloc-cache", "--quiet", ".\\src"])
        .assert()
        .success();
}

/// Tests path normalization with structure rules for directories.
/// Note: structure.rules `scope` patterns must match the directory path, not its contents.
/// Using `**/generated` to match any directory named "generated".
#[test]
fn check_structure_path_normalization() {
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

[structure]
max_files = 2
max_dirs = 1

[[structure.rules]]
scope = "**/generated"
max_files = 100
max_dirs = 10
"#,
    );
    // Create many files in generated (allowed by rule)
    for i in 0..5 {
        fixture.create_rust_file(&format!("src/generated/file{i}.rs"), 10);
    }

    // Check with "./" prefix - should use structure rule
    sloc_guard!()
        .current_dir(fixture.path())
        .args(["check", "--no-sloc-cache", "--quiet", "./src/generated"])
        .assert()
        .success();

    // Check with backslash - should also use structure rule
    sloc_guard!()
        .current_dir(fixture.path())
        .args(["check", "--no-sloc-cache", "--quiet", ".\\src\\generated"])
        .assert()
        .success();
}
