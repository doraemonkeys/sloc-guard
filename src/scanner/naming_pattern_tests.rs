use std::path::Path;

use super::*;
use crate::checker::ViolationType;
use tempfile::TempDir;

struct AcceptAllFilter;

impl FileFilter for AcceptAllFilter {
    fn should_include(&self, _path: &Path) -> bool {
        true
    }
}

// =============================================================================
// AllowlistRule Naming Pattern Tests
// =============================================================================

#[test]
fn allowlist_rule_builder_with_naming_pattern() {
    let rule = AllowlistRuleBuilder::new("src/**".to_string())
        .with_naming_pattern(Some("^[A-Z][a-zA-Z0-9]*\\.tsx$".to_string()))
        .build()
        .unwrap();
    assert!(rule.naming_pattern_str.is_some());
    assert_eq!(
        rule.naming_pattern_str.unwrap(),
        "^[A-Z][a-zA-Z0-9]*\\.tsx$"
    );
}

#[test]
fn allowlist_rule_naming_pattern_matches_valid_filename() {
    let rule = AllowlistRuleBuilder::new("src/**".to_string())
        .with_naming_pattern(Some("^[A-Z][a-zA-Z0-9]*\\.tsx$".to_string()))
        .build()
        .unwrap();

    assert!(rule.filename_matches_naming_pattern(Path::new("Button.tsx")));
    assert!(rule.filename_matches_naming_pattern(Path::new("UserProfile.tsx")));
    assert!(rule.filename_matches_naming_pattern(Path::new("A.tsx")));
}

#[test]
fn allowlist_rule_naming_pattern_rejects_invalid_filename() {
    let rule = AllowlistRuleBuilder::new("src/**".to_string())
        .with_naming_pattern(Some("^[A-Z][a-zA-Z0-9]*\\.tsx$".to_string()))
        .build()
        .unwrap();

    assert!(!rule.filename_matches_naming_pattern(Path::new("button.tsx")));
    assert!(!rule.filename_matches_naming_pattern(Path::new("userProfile.tsx")));
    assert!(!rule.filename_matches_naming_pattern(Path::new("user_profile.tsx")));
    assert!(!rule.filename_matches_naming_pattern(Path::new("Button.ts")));
}

#[test]
fn allowlist_rule_no_naming_pattern_accepts_all() {
    let rule = AllowlistRuleBuilder::new("src/**".to_string())
        .with_extensions(vec![".rs".to_string()])
        .build()
        .unwrap();

    assert!(rule.filename_matches_naming_pattern(Path::new("anything.rs")));
    assert!(rule.filename_matches_naming_pattern(Path::new("123.txt")));
    assert!(rule.filename_matches_naming_pattern(Path::new("UPPERCASE.rs")));
}

#[test]
fn allowlist_rule_builder_invalid_regex_returns_error() {
    let result = AllowlistRuleBuilder::new("src/**".to_string())
        .with_naming_pattern(Some("[invalid".to_string()))
        .build();
    assert!(result.is_err());
}

// =============================================================================
// Naming Pattern Scan Tests
// =============================================================================

#[test]
fn scan_with_structure_detects_naming_violations() {
    let temp_dir = TempDir::new().unwrap();
    let src_dir = temp_dir.path().join("src");
    std::fs::create_dir(&src_dir).unwrap();
    std::fs::write(src_dir.join("Button.tsx"), "").unwrap();
    std::fs::write(src_dir.join("userCard.tsx"), "").unwrap();

    let allowlist_rule = AllowlistRuleBuilder::new("**/src".to_string())
        .with_naming_pattern(Some("^[A-Z][a-zA-Z0-9]*\\.tsx$".to_string()))
        .build()
        .unwrap();
    let config =
        StructureScanConfig::new(&[], &[], vec![allowlist_rule], Vec::new(), &[], &[]).unwrap();
    let scanner = DirectoryScanner::new(AcceptAllFilter);
    let result = scanner
        .scan_with_structure(temp_dir.path(), Some(&config))
        .unwrap();

    assert_eq!(result.allowlist_violations.len(), 1);
    assert!(
        result.allowlist_violations[0]
            .path
            .ends_with("userCard.tsx")
    );
    assert!(matches!(
        result.allowlist_violations[0].violation_type,
        ViolationType::NamingConvention { .. }
    ));
}

#[test]
fn scan_with_structure_no_naming_violation_when_pattern_matches() {
    let temp_dir = TempDir::new().unwrap();
    let src_dir = temp_dir.path().join("src");
    std::fs::create_dir(&src_dir).unwrap();
    std::fs::write(src_dir.join("Button.tsx"), "").unwrap();
    std::fs::write(src_dir.join("UserCard.tsx"), "").unwrap();

    let allowlist_rule = AllowlistRuleBuilder::new("**/src".to_string())
        .with_naming_pattern(Some("^[A-Z][a-zA-Z0-9]*\\.tsx$".to_string()))
        .build()
        .unwrap();
    let config =
        StructureScanConfig::new(&[], &[], vec![allowlist_rule], Vec::new(), &[], &[]).unwrap();
    let scanner = DirectoryScanner::new(AcceptAllFilter);
    let result = scanner
        .scan_with_structure(temp_dir.path(), Some(&config))
        .unwrap();

    assert!(result.allowlist_violations.is_empty());
}

#[test]
fn scan_with_structure_naming_violation_includes_pattern() {
    let temp_dir = TempDir::new().unwrap();
    let src_dir = temp_dir.path().join("src");
    std::fs::create_dir(&src_dir).unwrap();
    std::fs::write(src_dir.join("lowercase.tsx"), "").unwrap();

    let pattern = "^[A-Z][a-zA-Z0-9]*\\.tsx$".to_string();
    let allowlist_rule = AllowlistRuleBuilder::new("**/src".to_string())
        .with_naming_pattern(Some(pattern.clone()))
        .build()
        .unwrap();
    let config =
        StructureScanConfig::new(&[], &[], vec![allowlist_rule], Vec::new(), &[], &[]).unwrap();
    let scanner = DirectoryScanner::new(AcceptAllFilter);
    let result = scanner
        .scan_with_structure(temp_dir.path(), Some(&config))
        .unwrap();

    assert_eq!(result.allowlist_violations.len(), 1);
    match &result.allowlist_violations[0].violation_type {
        ViolationType::NamingConvention { expected_pattern } => {
            assert_eq!(expected_pattern, &pattern);
        }
        _ => panic!("Expected NamingConvention violation"),
    }
}

#[test]
fn scan_with_structure_combined_allowlist_and_naming_violations() {
    let temp_dir = TempDir::new().unwrap();
    let src_dir = temp_dir.path().join("src");
    std::fs::create_dir(&src_dir).unwrap();
    std::fs::write(src_dir.join("Button.tsx"), "").unwrap();
    std::fs::write(src_dir.join("button.tsx"), "").unwrap();
    std::fs::write(src_dir.join("config.json"), "").unwrap();

    let allowlist_rule = AllowlistRuleBuilder::new("**/src".to_string())
        .with_extensions(vec![".tsx".to_string()])
        .with_naming_pattern(Some("^[A-Z][a-zA-Z0-9]*\\.tsx$".to_string()))
        .build()
        .unwrap();
    let config =
        StructureScanConfig::new(&[], &[], vec![allowlist_rule], Vec::new(), &[], &[]).unwrap();
    let scanner = DirectoryScanner::new(AcceptAllFilter);
    let result = scanner
        .scan_with_structure(temp_dir.path(), Some(&config))
        .unwrap();

    // Two violations: config.json (disallowed) and button.tsx (naming)
    assert_eq!(result.allowlist_violations.len(), 2);
}
