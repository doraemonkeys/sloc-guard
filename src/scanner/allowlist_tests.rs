use std::path::Path;

use super::*;

#[test]
fn allowlist_rule_builder_creates_rule() {
    let rule = AllowlistRuleBuilder::new("src/**".to_string())
        .with_extensions(vec![".rs".to_string()])
        .build()
        .unwrap();
    assert_eq!(rule.pattern, "src/**");
    assert_eq!(rule.allow_extensions, vec![".rs".to_string()]);
}

#[test]
fn allowlist_rule_builder_with_patterns() {
    let rule = AllowlistRuleBuilder::new("src/**".to_string())
        .with_patterns(vec!["*.config".to_string()])
        .build()
        .unwrap();
    assert!(!rule.allow_patterns.is_empty());
}

#[test]
fn allowlist_rule_matches_directory() {
    let rule = AllowlistRuleBuilder::new("src/**".to_string())
        .with_extensions(vec![".rs".to_string()])
        .build()
        .unwrap();
    assert!(rule.matches_directory(Path::new("src/lib")));
    assert!(!rule.matches_directory(Path::new("tests/lib")));
}

#[test]
fn allowlist_rule_file_matches_extension() {
    let rule = AllowlistRuleBuilder::new("src/**".to_string())
        .with_extensions(vec![".rs".to_string(), ".toml".to_string()])
        .build()
        .unwrap();
    assert!(rule.file_matches(Path::new("src/main.rs")));
    assert!(rule.file_matches(Path::new("src/Cargo.toml")));
    assert!(!rule.file_matches(Path::new("src/config.json")));
}

#[test]
fn allowlist_rule_file_matches_pattern() {
    let rule = AllowlistRuleBuilder::new("src/**".to_string())
        .with_patterns(vec!["Makefile".to_string()])
        .build()
        .unwrap();
    assert!(rule.file_matches(Path::new("src/Makefile")));
    assert!(!rule.file_matches(Path::new("src/config.json")));
}

#[test]
fn allowlist_rule_invalid_pattern_returns_error() {
    let result = AllowlistRuleBuilder::new("[invalid".to_string())
        .with_extensions(vec![".rs".to_string()])
        .build();
    assert!(result.is_err());
}

#[test]
fn allowlist_rule_file_no_match_empty_allowlist() {
    let rule = AllowlistRuleBuilder::new("src/**".to_string())
        .with_extensions(vec![])
        .with_patterns(vec![])
        .build()
        .unwrap();
    assert!(!rule.file_matches(Path::new("src/main.rs")));
}

#[test]
fn allowlist_rule_builder_invalid_allow_pattern() {
    let result = AllowlistRuleBuilder::new("src/**".to_string())
        .with_patterns(vec!["[invalid".to_string()])
        .build();
    assert!(result.is_err());
}

#[test]
fn allowlist_rule_empty_extension_list() {
    let rule = AllowlistRuleBuilder::new("src/**".to_string())
        .with_extensions(vec![])
        .build()
        .unwrap();
    assert!(rule.allow_extensions.is_empty());
}

#[test]
fn allowlist_rule_matches_pattern() {
    let rule = AllowlistRuleBuilder::new("src/**".to_string())
        .with_patterns(vec!["Makefile".to_string(), "*.config".to_string()])
        .build()
        .unwrap();
    assert!(rule.file_matches(Path::new("src/Makefile")));
    assert!(rule.file_matches(Path::new("src/app.config")));
    assert!(!rule.file_matches(Path::new("src/random.txt")));
}

#[test]
fn allowlist_rule_file_matches_by_full_path() {
    let rule = AllowlistRuleBuilder::new("src/**".to_string())
        .with_patterns(vec!["**/special.txt".to_string()])
        .build()
        .unwrap();
    assert!(rule.file_matches(Path::new("src/nested/special.txt")));
}

#[test]
fn allowlist_rule_no_extension_match_when_file_has_no_extension() {
    let rule = AllowlistRuleBuilder::new("src/**".to_string())
        .with_extensions(vec![".rs".to_string()])
        .build()
        .unwrap();

    assert!(!rule.file_matches(Path::new("src/Makefile")));
}

#[test]
fn allowlist_rule_matches_directory_partial() {
    let rule = AllowlistRuleBuilder::new("**/src/**".to_string())
        .with_extensions(vec![".rs".to_string()])
        .build()
        .unwrap();

    assert!(rule.matches_directory(Path::new("project/src/lib")));
    assert!(rule.matches_directory(Path::new("src/nested/deep")));
    assert!(!rule.matches_directory(Path::new("tests/unit")));
}

#[test]
fn allowlist_rule_extension_match_with_dot() {
    let rule = AllowlistRuleBuilder::new("src/**".to_string())
        .with_extensions(vec![".rs".to_string(), ".toml".to_string()])
        .build()
        .unwrap();

    assert!(rule.file_matches(Path::new("src/main.rs")));
    assert!(rule.file_matches(Path::new("src/Cargo.toml")));
    assert!(!rule.file_matches(Path::new("src/data.json")));
}
