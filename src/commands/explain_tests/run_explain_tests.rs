//! Integration tests for `run_explain_impl` command execution.
//!
//! Covers: path validation, file/directory handling, --sources flag,
//! config file loading, preset usage, and JSON format output.

use std::path::PathBuf;

use crate::cli::ExplainFormat;

// ============================================================================
// Path validation tests
// ============================================================================

#[test]
fn explain_non_existent_path_returns_error() {
    use crate::cli::{Cli, ColorChoice, Commands, ExtendsPolicy};

    let args = crate::cli::ExplainArgs {
        path: Some(PathBuf::from("non-existent-path-XYZ")),
        config: None,
        sources: false,
        format: ExplainFormat::Text,
    };

    let cli = Cli {
        verbose: 0,
        quiet: false,
        color: ColorChoice::Auto,
        no_config: true,
        no_extends: false,
        extends_policy: ExtendsPolicy::Normal,
        command: Commands::Explain(crate::cli::ExplainArgs {
            path: Some(PathBuf::from("non-existent-path-XYZ")),
            config: None,
            sources: false,
            format: ExplainFormat::Text,
        }),
    };

    let result = super::super::run_explain_impl(&args, &cli);

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("Path not found"));
}

// ============================================================================
// Directory handling tests
// ============================================================================

#[test]
fn run_explain_impl_with_existing_directory() {
    use crate::cli::{Cli, ColorChoice, Commands, ExtendsPolicy};
    use tempfile::TempDir;

    // Create a temporary directory
    let temp_dir = TempDir::new().unwrap();

    let args = crate::cli::ExplainArgs {
        path: Some(temp_dir.path().to_path_buf()),
        config: None,
        sources: false,
        format: ExplainFormat::Text,
    };

    let cli = Cli {
        verbose: 0,
        quiet: false,
        color: ColorChoice::Auto,
        no_config: true,
        no_extends: false,
        extends_policy: ExtendsPolicy::Normal,
        command: Commands::Explain(crate::cli::ExplainArgs {
            path: Some(temp_dir.path().to_path_buf()),
            config: None,
            sources: false,
            format: ExplainFormat::Text,
        }),
    };

    // With no_config=true and no structure rules, should show "No structure rules" message
    let result = super::super::run_explain_impl(&args, &cli);
    assert!(result.is_ok());
}

#[test]
fn run_explain_impl_directory_with_structure_config() {
    use crate::cli::{Cli, ColorChoice, Commands, ExtendsPolicy};
    use std::io::Write;
    use tempfile::TempDir;

    // Create a temporary directory with a config file
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join(".sloc-guard.toml");
    let mut config_file = std::fs::File::create(&config_path).unwrap();
    writeln!(
        config_file,
        r#"
version = "2"

[structure]
max_files = 50
max_dirs = 10
"#
    )
    .unwrap();

    let args = crate::cli::ExplainArgs {
        path: Some(temp_dir.path().to_path_buf()),
        config: Some(config_path.clone()),
        sources: false,
        format: ExplainFormat::Text,
    };

    let cli = Cli {
        verbose: 0,
        quiet: false,
        color: ColorChoice::Auto,
        no_config: false,
        no_extends: false,
        extends_policy: ExtendsPolicy::Normal,
        command: Commands::Explain(crate::cli::ExplainArgs {
            path: Some(temp_dir.path().to_path_buf()),
            config: Some(config_path),
            sources: false,
            format: ExplainFormat::Text,
        }),
    };

    let result = super::super::run_explain_impl(&args, &cli);
    assert!(result.is_ok());
}

#[test]
fn run_explain_impl_directory_with_json_format() {
    use crate::cli::{Cli, ColorChoice, Commands, ExtendsPolicy};
    use std::io::Write;
    use tempfile::TempDir;

    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join(".sloc-guard.toml");
    let mut config_file = std::fs::File::create(&config_path).unwrap();
    writeln!(
        config_file,
        r#"
version = "2"

[structure]
max_files = 25
"#
    )
    .unwrap();

    let args = crate::cli::ExplainArgs {
        path: Some(temp_dir.path().to_path_buf()),
        config: Some(config_path.clone()),
        sources: false,
        format: ExplainFormat::Json,
    };

    let cli = Cli {
        verbose: 0,
        quiet: false,
        color: ColorChoice::Auto,
        no_config: false,
        no_extends: false,
        extends_policy: ExtendsPolicy::Normal,
        command: Commands::Explain(crate::cli::ExplainArgs {
            path: Some(temp_dir.path().to_path_buf()),
            config: Some(config_path),
            sources: false,
            format: ExplainFormat::Json,
        }),
    };

    let result = super::super::run_explain_impl(&args, &cli);
    assert!(result.is_ok());
}

// ============================================================================
// --sources flag tests
// ============================================================================

#[test]
fn run_explain_impl_sources_with_no_config() {
    use crate::cli::{Cli, ColorChoice, Commands, ExtendsPolicy};

    let args = crate::cli::ExplainArgs {
        path: None,
        config: None,
        sources: true,
        format: ExplainFormat::Text,
    };

    let cli = Cli {
        verbose: 0,
        quiet: false,
        color: ColorChoice::Auto,
        no_config: true, // --no-config flag
        no_extends: false,
        extends_policy: ExtendsPolicy::Normal,
        command: Commands::Explain(crate::cli::ExplainArgs {
            path: None,
            config: None,
            sources: true,
            format: ExplainFormat::Text,
        }),
    };

    let result = super::super::run_explain_impl(&args, &cli);
    assert!(result.is_ok());
}

#[test]
fn run_explain_impl_sources_with_config_file() {
    use crate::cli::{Cli, ColorChoice, Commands, ExtendsPolicy};
    use std::io::Write;
    use tempfile::TempDir;

    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.toml");
    let mut config_file = std::fs::File::create(&config_path).unwrap();
    writeln!(
        config_file,
        r#"
version = "2"

[content]
max_lines = 300
"#
    )
    .unwrap();

    let args = crate::cli::ExplainArgs {
        path: None,
        config: Some(config_path.clone()),
        sources: true,
        format: ExplainFormat::Text,
    };

    let cli = Cli {
        verbose: 0,
        quiet: false,
        color: ColorChoice::Auto,
        no_config: false,
        no_extends: false,
        extends_policy: ExtendsPolicy::Normal,
        command: Commands::Explain(crate::cli::ExplainArgs {
            path: None,
            config: Some(config_path),
            sources: true,
            format: ExplainFormat::Text,
        }),
    };

    let result = super::super::run_explain_impl(&args, &cli);
    assert!(result.is_ok());
}

#[test]
fn run_explain_impl_sources_with_no_extends() {
    use crate::cli::{Cli, ColorChoice, Commands, ExtendsPolicy};
    use std::io::Write;
    use tempfile::TempDir;

    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.toml");
    let mut config_file = std::fs::File::create(&config_path).unwrap();
    writeln!(
        config_file,
        r#"
version = "2"
extends = "preset:rust-strict"

[content]
max_lines = 250
"#
    )
    .unwrap();

    let args = crate::cli::ExplainArgs {
        path: None,
        config: Some(config_path.clone()),
        sources: true,
        format: ExplainFormat::Text,
    };

    let cli = Cli {
        verbose: 0,
        quiet: false,
        color: ColorChoice::Auto,
        no_config: false,
        no_extends: true, // --no-extends flag
        extends_policy: ExtendsPolicy::Normal,
        command: Commands::Explain(crate::cli::ExplainArgs {
            path: None,
            config: Some(config_path),
            sources: true,
            format: ExplainFormat::Text,
        }),
    };

    let result = super::super::run_explain_impl(&args, &cli);
    assert!(result.is_ok());
}

#[test]
fn run_explain_impl_sources_json_format() {
    use crate::cli::{Cli, ColorChoice, Commands, ExtendsPolicy};
    use std::io::Write;
    use tempfile::TempDir;

    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.toml");
    let mut config_file = std::fs::File::create(&config_path).unwrap();
    writeln!(
        config_file,
        r#"
version = "2"

[content]
max_lines = 500
"#
    )
    .unwrap();

    let args = crate::cli::ExplainArgs {
        path: None,
        config: Some(config_path.clone()),
        sources: true,
        format: ExplainFormat::Json,
    };

    let cli = Cli {
        verbose: 0,
        quiet: false,
        color: ColorChoice::Auto,
        no_config: false,
        no_extends: false,
        extends_policy: ExtendsPolicy::Normal,
        command: Commands::Explain(crate::cli::ExplainArgs {
            path: None,
            config: Some(config_path),
            sources: true,
            format: ExplainFormat::Json,
        }),
    };

    let result = super::super::run_explain_impl(&args, &cli);
    assert!(result.is_ok());
}

#[test]
fn run_explain_impl_sources_no_config_with_no_extends() {
    use crate::cli::{Cli, ColorChoice, Commands, ExtendsPolicy};

    // When no config is specified and no-extends is true, should return empty result
    let args = crate::cli::ExplainArgs {
        path: None,
        config: None,
        sources: true,
        format: ExplainFormat::Text,
    };

    let cli = Cli {
        verbose: 0,
        quiet: false,
        color: ColorChoice::Auto,
        no_config: false, // Allow config discovery
        no_extends: true, // But don't follow extends
        extends_policy: ExtendsPolicy::Normal,
        command: Commands::Explain(crate::cli::ExplainArgs {
            path: None,
            config: None,
            sources: true,
            format: ExplainFormat::Text,
        }),
    };

    let result = super::super::run_explain_impl(&args, &cli);
    assert!(result.is_ok());
}

// ============================================================================
// Preset configuration tests
// ============================================================================

#[test]
fn run_explain_impl_file_with_preset_config() {
    use crate::cli::{Cli, ColorChoice, Commands, ExtendsPolicy};
    use std::io::Write;
    use tempfile::TempDir;

    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.toml");
    let mut config_file = std::fs::File::create(&config_path).unwrap();
    writeln!(
        config_file,
        r#"
version = "2"
extends = "preset:rust-strict"

[content]
max_lines = 400
"#
    )
    .unwrap();

    // Create a test file
    let test_file = temp_dir.path().join("test.rs");
    std::fs::write(&test_file, "fn main() {}").unwrap();

    let args = crate::cli::ExplainArgs {
        path: Some(test_file.clone()),
        config: Some(config_path.clone()),
        sources: false,
        format: ExplainFormat::Text,
    };

    let cli = Cli {
        verbose: 0,
        quiet: false,
        color: ColorChoice::Auto,
        no_config: false,
        no_extends: false,
        extends_policy: ExtendsPolicy::Normal,
        command: Commands::Explain(crate::cli::ExplainArgs {
            path: Some(test_file),
            config: Some(config_path),
            sources: false,
            format: ExplainFormat::Text,
        }),
    };

    let result = super::super::run_explain_impl(&args, &cli);
    assert!(result.is_ok());
}
