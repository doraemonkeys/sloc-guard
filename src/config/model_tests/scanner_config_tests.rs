use super::*;

#[test]
fn scanner_config_default_excludes_git_directory() {
    // Regression test: .git/** must always be excluded by default.
    // Without this, structure checks would traverse .git/objects and fail
    // with "Directories: 253 (limit: 10)" type errors.
    let config = ScannerConfig::default();
    assert!(
        config.exclude.contains(&".git/**".to_string()),
        "Default scanner.exclude must contain '.git/**' to prevent structure checks on git internals"
    );
}

#[test]
fn scanner_config_default_gitignore_enabled() {
    let config = ScannerConfig::default();
    assert!(config.gitignore, "gitignore should be enabled by default");
}

#[test]
fn config_default_scanner_excludes_git() {
    // Verify Config::default() propagates the scanner defaults correctly
    let config = Config::default();
    assert!(
        config.scanner.exclude.contains(&".git/**".to_string()),
        "Config::default() must have .git/** in scanner.exclude"
    );
}

#[test]
fn config_deserialize_without_scanner_uses_git_exclude_default() {
    // When scanner.exclude is not specified, the default should include .git/**
    let toml_str = r#"
        version = "2"

        [content]
        max_lines = 500
    "#;

    let config: Config = toml::from_str(toml_str).unwrap();
    assert!(
        config.scanner.exclude.contains(&".git/**".to_string()),
        "Parsing config without scanner.exclude should use default containing .git/**"
    );
}

/// When `scanner.exclude` is explicitly set, it should replace (not merge with) the default.
///
/// Design intent: User's explicit config replaces default; they must include `.git/**` if needed.
/// This is a deliberate replace-not-merge semantic to give users full control.
#[test]
fn config_deserialize_with_explicit_scanner_exclude_replaces_default() {
    let toml_str = r#"
        version = "2"

        [scanner]
        exclude = ["target/**", "node_modules/**"]
    "#;

    let config: Config = toml::from_str(toml_str).unwrap();
    assert_eq!(config.scanner.exclude, vec!["target/**", "node_modules/**"]);
    assert!(
        !config.scanner.exclude.contains(&".git/**".to_string()),
        "Explicit exclude should replace default, not merge"
    );
}

#[test]
fn config_deserialize_scanner_exclude_with_git() {
    // User explicitly includes .git/** along with other patterns
    let toml_str = r#"
        version = "2"

        [scanner]
        exclude = ["target/**", ".git/**", "vendor/**"]
    "#;

    let config: Config = toml::from_str(toml_str).unwrap();
    assert!(config.scanner.exclude.contains(&".git/**".to_string()));
    assert!(config.scanner.exclude.contains(&"target/**".to_string()));
    assert!(config.scanner.exclude.contains(&"vendor/**".to_string()));
}
