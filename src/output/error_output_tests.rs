use super::*;

fn make_output(use_colors: bool) -> ErrorOutput {
    ErrorOutput::with_colors(use_colors)
}

#[test]
fn error_without_colors_basic() {
    let out = make_output(false);
    let mut buf = Vec::new();
    out.write_error(&mut buf, "Config", "invalid threshold", None, None);
    let result = String::from_utf8(buf).unwrap();
    assert_eq!(result, "✖ Config: invalid threshold\n");
}

#[test]
fn error_without_colors_with_detail() {
    let out = make_output(false);
    let mut buf = Vec::new();
    out.write_error(
        &mut buf,
        "Config",
        "invalid threshold",
        Some("max_lines must be positive"),
        None,
    );
    let result = String::from_utf8(buf).unwrap();
    assert!(result.contains("✖ Config: invalid threshold\n"));
    assert!(result.contains("  × max_lines must be positive\n"));
}

#[test]
fn error_without_colors_with_suggestion() {
    let out = make_output(false);
    let mut buf = Vec::new();
    out.write_error(
        &mut buf,
        "Git",
        "not a git repository",
        None,
        Some("Run 'git init' or run inside a git repository"),
    );
    let result = String::from_utf8(buf).unwrap();
    assert!(result.contains("✖ Git: not a git repository\n"));
    assert!(result.contains("  help: Run 'git init' or run inside a git repository\n"));
}

#[test]
fn error_without_colors_full() {
    let out = make_output(false);
    let mut buf = Vec::new();
    out.write_error(
        &mut buf,
        "Config",
        "invalid pattern",
        Some("globset parse error: unexpected '*'"),
        Some("Check glob syntax documentation"),
    );
    let result = String::from_utf8(buf).unwrap();
    assert!(result.contains("✖ Config: invalid pattern\n"));
    assert!(result.contains("  × globset parse error: unexpected '*'\n"));
    assert!(result.contains("  help: Check glob syntax documentation\n"));
}

#[test]
fn warning_without_colors_basic() {
    let out = make_output(false);
    let mut buf = Vec::new();
    out.write_warning(&mut buf, "deprecated config option", None, None);
    let result = String::from_utf8(buf).unwrap();
    assert_eq!(result, "⚠ Warning: deprecated config option\n");
}

#[test]
fn warning_without_colors_with_detail() {
    let out = make_output(false);
    let mut buf = Vec::new();
    out.write_warning(
        &mut buf,
        "rule expired",
        Some("content.rules[0] expired on 2024-01-01"),
        None,
    );
    let result = String::from_utf8(buf).unwrap();
    assert!(result.contains("⚠ Warning: rule expired\n"));
    assert!(result.contains("  × content.rules[0] expired on 2024-01-01\n"));
}

#[test]
fn warning_without_colors_with_suggestion() {
    let out = make_output(false);
    let mut buf = Vec::new();
    out.write_warning(
        &mut buf,
        "no baseline found",
        None,
        Some("Use --baseline to specify a baseline file"),
    );
    let result = String::from_utf8(buf).unwrap();
    assert!(result.contains("⚠ Warning: no baseline found\n"));
    assert!(result.contains("  help: Use --baseline to specify a baseline file\n"));
}

#[test]
fn error_with_colors_contains_ansi() {
    let out = make_output(true);
    let mut buf = Vec::new();
    out.write_error(&mut buf, "Config", "test error", None, None);
    let result = String::from_utf8(buf).unwrap();
    // Verify ANSI codes are present
    assert!(result.contains("\x1b["));
    assert!(result.contains("✖ Config:"));
    assert!(result.contains("test error"));
}

#[test]
fn warning_with_colors_contains_ansi() {
    let out = make_output(true);
    let mut buf = Vec::new();
    out.write_warning(&mut buf, "test warning", None, None);
    let result = String::from_utf8(buf).unwrap();
    // Verify ANSI codes are present
    assert!(result.contains("\x1b["));
    assert!(result.contains("⚠ Warning:"));
    assert!(result.contains("test warning"));
}

#[test]
fn error_with_colors_full_message() {
    let out = make_output(true);
    let mut buf = Vec::new();
    out.write_error(
        &mut buf,
        "FileRead",
        "failed to read",
        Some("permission denied"),
        Some("Check file permissions"),
    );
    let result = String::from_utf8(buf).unwrap();
    // Verify structure
    assert!(result.contains("✖ FileRead:"));
    assert!(result.contains("failed to read"));
    assert!(result.contains("× permission denied"));
    assert!(result.contains("help:"));
    assert!(result.contains("Check file permissions"));
}

#[test]
fn default_creates_stderr_output() {
    // Just verify it doesn't panic
    let _ = ErrorOutput::default();
}

#[test]
fn new_with_always_mode() {
    let out = ErrorOutput::new(ColorMode::Always);
    // use_colors should be true
    let mut buf = Vec::new();
    out.write_error(&mut buf, "Test", "msg", None, None);
    let result = String::from_utf8(buf).unwrap();
    assert!(result.contains("\x1b[")); // ANSI present
}

#[test]
fn new_with_never_mode() {
    let out = ErrorOutput::new(ColorMode::Never);
    // use_colors should be false
    let mut buf = Vec::new();
    out.write_error(&mut buf, "Test", "msg", None, None);
    let result = String::from_utf8(buf).unwrap();
    assert!(!result.contains("\x1b[")); // No ANSI codes
    assert_eq!(result, "✖ Test: msg\n");
}

// NOTE: NO_COLOR environment variable detection is NOT tested here.
// Reason: std::env::set_var/remove_var are unsafe in Rust Edition 2024
// due to potential data races in multithreaded tests.
// The is_no_color_set() function is a trivial wrapper around std::env::var(),
// and testing standard library behavior is unnecessary.
// The actual color output behavior is tested via new_with_always_mode/new_with_never_mode.

#[test]
fn info_without_colors_basic() {
    let out = make_output(false);
    let mut buf = Vec::new();
    out.write_info(&mut buf, "Using preset: rust-strict", None, None);
    let result = String::from_utf8(buf).unwrap();
    assert_eq!(result, "ℹ Using preset: rust-strict\n");
}

#[test]
fn info_without_colors_with_detail() {
    let out = make_output(false);
    let mut buf = Vec::new();
    out.write_info(
        &mut buf,
        "Using preset: rust-strict",
        Some("max_lines = 500"),
        None,
    );
    let result = String::from_utf8(buf).unwrap();
    assert!(result.contains("ℹ Using preset: rust-strict\n"));
    assert!(result.contains("  × max_lines = 500\n"));
}

#[test]
fn info_without_colors_with_suggestion() {
    let out = make_output(false);
    let mut buf = Vec::new();
    out.write_info(
        &mut buf,
        "Using preset: rust-strict",
        None,
        Some("Run `sloc-guard config show` to see effective settings"),
    );
    let result = String::from_utf8(buf).unwrap();
    assert!(result.contains("ℹ Using preset: rust-strict\n"));
    assert!(result.contains("  help: Run `sloc-guard config show` to see effective settings\n"));
}

#[test]
fn info_without_colors_full() {
    let out = make_output(false);
    let mut buf = Vec::new();
    out.write_info(
        &mut buf,
        "Using preset: rust-strict",
        Some("max_lines = 500"),
        Some("Run `sloc-guard config show` to see effective settings"),
    );
    let result = String::from_utf8(buf).unwrap();
    assert!(result.contains("ℹ Using preset: rust-strict\n"));
    assert!(result.contains("  × max_lines = 500\n"));
    assert!(result.contains("  help: Run `sloc-guard config show` to see effective settings\n"));
}

#[test]
fn info_with_colors_contains_ansi() {
    let out = make_output(true);
    let mut buf = Vec::new();
    out.write_info(&mut buf, "Using preset: rust-strict", None, None);
    let result = String::from_utf8(buf).unwrap();
    // Verify ANSI codes are present (cyan color)
    assert!(result.contains("\x1b["));
    assert!(result.contains("ℹ"));
    assert!(result.contains("Using preset: rust-strict"));
}

#[test]
fn info_with_colors_full_message() {
    let out = make_output(true);
    let mut buf = Vec::new();
    out.write_info(
        &mut buf,
        "Using preset: rust-strict",
        Some("max_lines = 500"),
        Some("Run `sloc-guard config show` to see effective settings"),
    );
    let result = String::from_utf8(buf).unwrap();
    // Verify structure
    assert!(result.contains("ℹ"));
    assert!(result.contains("Using preset: rust-strict"));
    assert!(result.contains("× max_lines = 500"));
    assert!(result.contains("help:"));
    assert!(result.contains("Run `sloc-guard config show` to see effective settings"));
}
