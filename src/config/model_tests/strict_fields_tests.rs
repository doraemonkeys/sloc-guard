//! Unknown-field rejection (`deny_unknown_fields`) across all config structs.
//!
//! A misspelled or misplaced key must hard-error instead of being silently
//! dropped: a config that "looks like it works" but is ignored is worse than
//! one that fails loudly.

use super::*;

/// Assert that the TOML is rejected and the error names the offending field.
fn assert_unknown_field(toml_str: &str, field: &str) {
    let err = toml::from_str::<Config>(toml_str).unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains(field),
        "expected unknown-field error naming '{field}', got: {msg}"
    );
}

#[test]
fn top_level_unknown_field_rejected() {
    assert_unknown_field("not_a_section = 1", "not_a_section");
}

#[test]
fn top_level_unknown_table_rejected() {
    assert_unknown_field("[strcture]\nmax_files = 10", "strcture");
}

#[test]
fn scanner_unknown_field_rejected() {
    assert_unknown_field(
        r#"
        [scanner]
        gitignore = true
        whitelist = ["src/**"]
        "#,
        "whitelist",
    );
}

#[test]
fn content_unknown_field_rejected() {
    assert_unknown_field(
        r"
        [content]
        max_linez = 100
        ",
        "max_linez",
    );
}

#[test]
fn content_rule_unknown_field_rejected() {
    assert_unknown_field(
        r#"
        [[content.rules]]
        pattern = "**/*.rs"
        max_lines = 100
        skip_blanks = true
        "#,
        "skip_blanks",
    );
}

#[test]
fn sarif_unknown_field_rejected() {
    assert_unknown_field(
        r#"
        [sarif]
        level = "error"
        "#,
        "level",
    );
}

#[test]
fn baseline_unknown_field_rejected() {
    assert_unknown_field(
        r#"
        [baseline]
        mode = "warn"
        "#,
        "mode",
    );
}

#[test]
fn trend_unknown_field_rejected() {
    assert_unknown_field(
        r"
        [trend]
        max_entrys = 5
        ",
        "max_entrys",
    );
}

#[test]
fn stats_unknown_field_rejected() {
    // top_count belongs under [stats.report], not [stats]
    assert_unknown_field(
        r"
        [stats]
        top_count = 5
        ",
        "top_count",
    );
}

#[test]
fn stats_report_unknown_field_rejected() {
    assert_unknown_field(
        r"
        [stats.report]
        top = 5
        ",
        "top",
    );
}

#[test]
fn check_unknown_field_rejected() {
    assert_unknown_field(
        r"
        [check]
        fail_fats = true
        ",
        "fail_fats",
    );
}

#[test]
fn custom_language_unknown_field_rejected() {
    assert_unknown_field(
        r#"
        [languages.mylang]
        extension = ["ml"]
        "#,
        "extension",
    );
}
