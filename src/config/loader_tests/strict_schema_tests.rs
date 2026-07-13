//! Tests for per-source schema enforcement through the loader: unknown fields
//! must be rejected with the originating file and line, even inside an extends
//! chain, while individually-partial chain members still merge and load.

use std::path::Path;

use crate::config::FileConfigLoader;
use crate::config::loader::ConfigLoader;
use crate::error::{ConfigSource, SlocGuardError};

use super::mock_fs::MockFileSystem;

fn expect_syntax(err: SlocGuardError) -> (ConfigSource, usize, String) {
    match err {
        SlocGuardError::Syntax {
            origin,
            line,
            message,
            ..
        } => (
            origin.expect("Syntax error should carry origin"),
            line,
            message,
        ),
        other => panic!("Expected Syntax error, got: {other:?}"),
    }
}

fn assert_file_origin(origin: &ConfigSource, file_name: &str) {
    match origin {
        ConfigSource::File { path } => assert!(
            path.to_string_lossy().contains(file_name),
            "expected origin '{file_name}', got: {}",
            path.display()
        ),
        other => panic!("Expected File origin, got: {other:?}"),
    }
}

#[test]
fn unknown_field_reports_origin_and_line() {
    let config_content = "version = \"2\"\n\n[content]\nmax_linez = 300\n";

    let fs = MockFileSystem::new().with_file("/project/.sloc-guard.toml", config_content);
    let loader = FileConfigLoader::with_fs(fs);
    let err = loader
        .load_from_path(Path::new("/project/.sloc-guard.toml"))
        .unwrap_err();

    let (origin, line, message) = expect_syntax(err);
    assert_file_origin(&origin, ".sloc-guard.toml");
    assert_eq!(line, 4, "expected error at the unknown key's line");
    assert!(message.contains("max_linez"), "got: {message}");
}

#[test]
fn unknown_field_in_structure_rule_rejected_by_loader() {
    let config_content = "\
[structure]
max_files = 20

[[structure.rules]]
scope = \"src/**\"
count_excludes = [\"*.md\"]
";

    let fs = MockFileSystem::new().with_file("/project/.sloc-guard.toml", config_content);
    let loader = FileConfigLoader::with_fs(fs);
    let err = loader
        .load_from_path(Path::new("/project/.sloc-guard.toml"))
        .unwrap_err();

    let (origin, line, message) = expect_syntax(err);
    assert_file_origin(&origin, ".sloc-guard.toml");
    assert_eq!(line, 6, "expected error at the count_excludes line");
    assert!(message.contains("count_excludes"), "got: {message}");
}

#[test]
fn rule_count_exclude_accepted_by_loader() {
    let config_content = "\
[structure]
max_files = 20
count_exclude = [\".gitkeep\"]

[[structure.rules]]
scope = \"src/**\"
count_exclude = [\"*.gen\"]
";

    let fs = MockFileSystem::new().with_file("/project/.sloc-guard.toml", config_content);
    let loader = FileConfigLoader::with_fs(fs);
    let result = loader
        .load_from_path(Path::new("/project/.sloc-guard.toml"))
        .unwrap();

    assert_eq!(result.config.structure.count_exclude, vec![".gitkeep"]);
    assert_eq!(
        result.config.structure.rules[0].count_exclude,
        vec!["*.gen"]
    );
}

#[test]
fn unknown_field_in_extends_base_names_base_file() {
    let base_content = "version = \"2\"\n\n[content]\nmax_linez = 100\n";
    let child_content = r#"
version = "2"
extends = "/base.toml"

[content]
max_lines = 200
"#;

    let fs = MockFileSystem::new()
        .with_file("/base.toml", base_content)
        .with_file("/child.toml", child_content);
    let loader = FileConfigLoader::with_fs(fs);
    let err = loader.load_from_path(Path::new("/child.toml")).unwrap_err();

    let (origin, line, message) = expect_syntax(err);
    assert_file_origin(&origin, "base.toml");
    assert_eq!(line, 4, "expected the line within base.toml");
    assert!(message.contains("max_linez"), "got: {message}");
}

#[test]
fn unknown_field_in_extends_child_names_child_file() {
    let base_content = r#"
version = "2"

[content]
max_lines = 100
"#;
    let child_content = "version = \"2\"\nextends = \"/base.toml\"\n\n[content]\nmax_linez = 200\n";

    let fs = MockFileSystem::new()
        .with_file("/base.toml", base_content)
        .with_file("/child.toml", child_content);
    let loader = FileConfigLoader::with_fs(fs);
    let err = loader.load_from_path(Path::new("/child.toml")).unwrap_err();

    let (origin, line, message) = expect_syntax(err);
    assert_file_origin(&origin, "child.toml");
    assert_eq!(line, 5, "expected the line within child.toml");
    assert!(message.contains("max_linez"), "got: {message}");
}

#[test]
fn unknown_field_in_config_extending_preset_names_user_file() {
    let child_content = "extends = \"preset:rust-strict\"\n\n[content]\nmax_linez = 500\n";

    let fs = MockFileSystem::new().with_file("/child.toml", child_content);
    let loader = FileConfigLoader::with_fs(fs);
    let err = loader.load_from_path(Path::new("/child.toml")).unwrap_err();

    let (origin, line, message) = expect_syntax(err);
    assert_file_origin(&origin, "child.toml");
    assert_eq!(line, 4);
    assert!(message.contains("max_linez"), "got: {message}");
}

#[test]
fn unknown_field_with_reset_markers_reports_origin() {
    // Reset markers force validation via re-rendered TOML: line numbers are
    // approximate there, but origin and the offending field must survive.
    let config_content = r#"
version = "2"

[scanner]
exclude = ["$reset", "build/**"]

[content]
max_linez = 100
"#;

    let fs = MockFileSystem::new().with_file("/project/.sloc-guard.toml", config_content);
    let loader = FileConfigLoader::with_fs(fs);
    let err = loader
        .load_from_path(Path::new("/project/.sloc-guard.toml"))
        .unwrap_err();

    let (origin, _line, message) = expect_syntax(err);
    assert_file_origin(&origin, ".sloc-guard.toml");
    assert!(message.contains("max_linez"), "got: {message}");
}

#[test]
fn load_without_extends_rejects_unknown_field() {
    let config_content = "version = \"2\"\n\n[check]\nfail_fats = true\n";

    let fs = MockFileSystem::new().with_file("/project/.sloc-guard.toml", config_content);
    let loader = FileConfigLoader::with_fs(fs);
    let err = loader
        .load_from_path_without_extends(Path::new("/project/.sloc-guard.toml"))
        .unwrap_err();

    let (origin, line, message) = expect_syntax(err);
    assert_file_origin(&origin, ".sloc-guard.toml");
    assert_eq!(line, 4);
    assert!(message.contains("fail_fats"), "got: {message}");
}

#[test]
fn partial_configs_across_extends_chain_still_load() {
    // Each file alone configures only one section; validation must run per
    // source for attribution yet still accept partial configs, because the
    // effective config is the post-merge union.
    let base_content = r"
[structure]
max_files = 10
";
    let child_content = r#"
extends = "/base.toml"

[content]
max_lines = 100
"#;

    let fs = MockFileSystem::new()
        .with_file("/base.toml", base_content)
        .with_file("/child.toml", child_content);
    let loader = FileConfigLoader::with_fs(fs);
    let result = loader.load_from_path(Path::new("/child.toml")).unwrap();

    assert_eq!(result.config.structure.max_files, Some(10));
    assert_eq!(result.config.content.max_lines, 100);
}

#[test]
fn preset_with_partial_user_config_still_loads() {
    let child_content = r#"
extends = "preset:rust-strict"

[content]
max_lines = 123
"#;

    let fs = MockFileSystem::new().with_file("/child.toml", child_content);
    let loader = FileConfigLoader::with_fs(fs);
    let result = loader.load_from_path(Path::new("/child.toml")).unwrap();

    assert_eq!(result.preset_used, Some("rust-strict".to_string()));
    assert_eq!(result.config.content.max_lines, 123);
    // Preset-provided settings survive the merge
    assert_eq!(result.config.structure.max_files, Some(20));
}

#[test]
fn deny_file_patterns_alias_accepted_by_strict_schema() {
    let config_content = r#"
[structure]
deny_file_patterns = ["*.bak"]

[[structure.rules]]
scope = "src/**"
deny_file_patterns = ["temp_*"]
"#;

    let fs = MockFileSystem::new().with_file("/project/.sloc-guard.toml", config_content);
    let loader = FileConfigLoader::with_fs(fs);
    let result = loader
        .load_from_path(Path::new("/project/.sloc-guard.toml"))
        .unwrap();

    assert_eq!(result.config.structure.deny_files, vec!["*.bak"]);
    assert_eq!(result.config.structure.rules[0].deny_files, vec!["temp_*"]);
}
