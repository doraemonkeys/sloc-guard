use super::*;

// =============================================================================
// StructureConfig Default and Basic Tests
// =============================================================================

#[test]
fn structure_config_default_values() {
    let config = StructureConfig::default();
    assert!(config.max_files.is_none());
    assert!(config.max_dirs.is_none());
    assert!(config.count_exclude.is_empty());
    assert!(config.rules.is_empty());
}

#[test]
fn config_structure_default_empty() {
    let config = Config::default();
    assert!(config.structure.max_files.is_none());
    assert!(config.structure.max_dirs.is_none());
    assert!(config.structure.count_exclude.is_empty());
    assert!(config.structure.rules.is_empty());
}

#[test]
fn config_deserialize_structure_global_limits() {
    let toml_str = r#"
        version = "2"

        [structure]
        max_files = 10
        max_dirs = 5
        count_exclude = ["*.md", ".gitkeep"]
    "#;

    let config: Config = toml::from_str(toml_str).unwrap();
    assert_eq!(config.structure.max_files, Some(10));
    assert_eq!(config.structure.max_dirs, Some(5));
    assert_eq!(config.structure.count_exclude, vec!["*.md", ".gitkeep"]);
}

#[test]
fn config_deserialize_structure_with_rules() {
    let toml_str = r#"
        version = "2"

        [structure]
        max_files = 10
        max_dirs = 5

        [[structure.rules]]
        scope = "src/generated/**"
        max_files = 50

        [[structure.rules]]
        scope = "tests/**"
        max_files = 20
        max_dirs = 10
    "#;

    let config: Config = toml::from_str(toml_str).unwrap();
    assert_eq!(config.structure.rules.len(), 2);

    let rule0 = &config.structure.rules[0];
    assert_eq!(rule0.scope, "src/generated/**");
    assert_eq!(rule0.max_files, Some(50));
    assert!(rule0.max_dirs.is_none());

    let rule1 = &config.structure.rules[1];
    assert_eq!(rule1.scope, "tests/**");
    assert_eq!(rule1.max_files, Some(20));
    assert_eq!(rule1.max_dirs, Some(10));
}

#[test]
fn config_deserialize_structure_only_rules() {
    let toml_str = r#"
        version = "2"

        [[structure.rules]]
        scope = "vendor/**"
        max_files = 100
    "#;

    let config: Config = toml::from_str(toml_str).unwrap();
    assert!(config.structure.max_files.is_none());
    assert!(config.structure.max_dirs.is_none());
    assert_eq!(config.structure.rules.len(), 1);
    assert_eq!(config.structure.rules[0].scope, "vendor/**");
}

// =============================================================================
// Warn Threshold Tests
// =============================================================================

#[test]
fn config_deserialize_structure_warn_threshold() {
    let toml_str = r#"
        version = "2"

        [structure]
        max_files = 50
        max_dirs = 10
        warn_threshold = 0.9
    "#;

    let config: Config = toml::from_str(toml_str).unwrap();
    assert_eq!(config.structure.max_files, Some(50));
    assert_eq!(config.structure.max_dirs, Some(10));
    assert_eq!(config.structure.warn_threshold, Some(0.9));
}

#[test]
fn config_deserialize_structure_rule_warn_threshold() {
    let toml_str = r#"
        version = "2"

        [structure]
        max_files = 50
        warn_threshold = 0.9

        [[structure.rules]]
        scope = "src/generated/**"
        max_files = 100
        warn_threshold = 0.8
    "#;

    let config: Config = toml::from_str(toml_str).unwrap();
    assert_eq!(config.structure.warn_threshold, Some(0.9));
    assert_eq!(config.structure.rules.len(), 1);
    assert_eq!(config.structure.rules[0].warn_threshold, Some(0.8));
}

#[test]
fn structure_config_warn_threshold_default_none() {
    let config = StructureConfig::default();
    assert!(config.warn_threshold.is_none());
}

// =============================================================================
// deny_files and deny_dirs Tests
// =============================================================================

#[test]
fn config_deserialize_structure_deny_files() {
    let toml_str = r#"
        [structure]
        deny_files = ["*.bak", "secrets.*", ".DS_Store"]
    "#;

    let config: Config = toml::from_str(toml_str).unwrap();
    assert_eq!(
        config.structure.deny_files,
        vec!["*.bak", "secrets.*", ".DS_Store"]
    );
}

#[test]
fn config_deserialize_structure_deny_dirs() {
    let toml_str = r#"
        [structure]
        deny_dirs = ["__pycache__", "node_modules", ".git"]
    "#;

    let config: Config = toml::from_str(toml_str).unwrap();
    assert_eq!(
        config.structure.deny_dirs,
        vec!["__pycache__", "node_modules", ".git"]
    );
}

#[test]
fn config_deserialize_structure_deny_files_and_dirs_combined() {
    let toml_str = r#"
        [structure]
        max_files = 50
        deny_files = ["*.bak", "secrets.*"]
        deny_dirs = ["__pycache__", "node_modules"]
        deny_extensions = ["dll", "exe"]
    "#;

    let config: Config = toml::from_str(toml_str).unwrap();
    assert_eq!(config.structure.max_files, Some(50));
    assert_eq!(config.structure.deny_files, vec!["*.bak", "secrets.*"]);
    assert_eq!(
        config.structure.deny_dirs,
        vec!["__pycache__", "node_modules"]
    );
    assert_eq!(config.structure.deny_extensions, vec!["dll", "exe"]);
}

#[test]
fn config_deserialize_structure_rule_deny_files() {
    let toml_str = r#"
        [[structure.rules]]
        scope = "src/**"
        deny_files = ["util.rs", "helper.rs"]
    "#;

    let config: Config = toml::from_str(toml_str).unwrap();
    assert_eq!(config.structure.rules.len(), 1);
    assert_eq!(
        config.structure.rules[0].deny_files,
        vec!["util.rs", "helper.rs"]
    );
}

#[test]
fn config_deserialize_structure_rule_deny_dirs() {
    let toml_str = r#"
        [[structure.rules]]
        scope = "src/**"
        deny_dirs = ["temp_*", "__snapshots__"]
    "#;

    let config: Config = toml::from_str(toml_str).unwrap();
    assert_eq!(config.structure.rules.len(), 1);
    assert_eq!(
        config.structure.rules[0].deny_dirs,
        vec!["temp_*", "__snapshots__"]
    );
}

#[test]
fn config_deserialize_structure_rule_deny_files_and_dirs() {
    let toml_str = r#"
        [[structure.rules]]
        scope = "tests/**"
        max_files = 100
        deny_files = ["common.rs"]
        deny_dirs = ["fixtures"]
    "#;

    let config: Config = toml::from_str(toml_str).unwrap();
    assert_eq!(config.structure.rules.len(), 1);
    let rule = &config.structure.rules[0];
    assert_eq!(rule.scope, "tests/**");
    assert_eq!(rule.max_files, Some(100));
    assert_eq!(rule.deny_files, vec!["common.rs"]);
    assert_eq!(rule.deny_dirs, vec!["fixtures"]);
}

#[test]
fn config_deserialize_deny_file_patterns_alias() {
    // Test backward compatibility: deny_file_patterns should be aliased to deny_files
    let toml_str = r#"
        [structure]
        deny_file_patterns = ["*.bak", "temp_*"]
    "#;

    let config: Config = toml::from_str(toml_str).unwrap();
    assert_eq!(config.structure.deny_files, vec!["*.bak", "temp_*"]);
}

#[test]
fn config_deserialize_rule_deny_file_patterns_alias() {
    // Test backward compatibility at rule level
    let toml_str = r#"
        [[structure.rules]]
        scope = "src/**"
        deny_file_patterns = ["backup*"]
    "#;

    let config: Config = toml::from_str(toml_str).unwrap();
    assert_eq!(config.structure.rules[0].deny_files, vec!["backup*"]);
}

#[test]
fn structure_config_deny_files_default_empty() {
    let config = StructureConfig::default();
    assert!(config.deny_files.is_empty());
    assert!(config.deny_dirs.is_empty());
}

#[test]
fn config_deserialize_structure_all_deny_types() {
    let toml_str = r#"
        [structure]
        deny_extensions = [".exe", ".dll"]
        deny_patterns = ["*.bak"]
        deny_files = ["secrets.*"]
        deny_dirs = ["__pycache__"]
    "#;

    let config: Config = toml::from_str(toml_str).unwrap();
    assert_eq!(config.structure.deny_extensions, vec![".exe", ".dll"]);
    assert_eq!(config.structure.deny_patterns, vec!["*.bak"]);
    assert_eq!(config.structure.deny_files, vec!["secrets.*"]);
    assert_eq!(config.structure.deny_dirs, vec!["__pycache__"]);
}

// =============================================================================
// SiblingRule Deserialization Tests
// =============================================================================

#[test]
fn sibling_rule_directed_deserialize() {
    let toml_str = r#"
        [[structure.rules]]
        scope = "src/**"

        [[structure.rules.siblings]]
        match = "*.tsx"
        require = "{stem}.test.tsx"
    "#;

    let config: Config = toml::from_str(toml_str).unwrap();
    assert_eq!(config.structure.rules.len(), 1);
    assert_eq!(config.structure.rules[0].siblings.len(), 1);

    match &config.structure.rules[0].siblings[0] {
        SiblingRule::Directed {
            match_pattern,
            require,
            ..
        } => {
            assert_eq!(match_pattern, "*.tsx");
            assert_eq!(require.as_patterns(), vec!["{stem}.test.tsx"]);
        }
        SiblingRule::Group { .. } => panic!("Expected Directed rule"),
    }
}

#[test]
fn sibling_rule_group_deserialize() {
    let toml_str = r#"
        [[structure.rules]]
        scope = "src/**"

        [[structure.rules.siblings]]
        group = ["{stem}.tsx", "{stem}.test.tsx"]
    "#;

    let config: Config = toml::from_str(toml_str).unwrap();
    assert_eq!(config.structure.rules.len(), 1);
    assert_eq!(config.structure.rules[0].siblings.len(), 1);

    match &config.structure.rules[0].siblings[0] {
        SiblingRule::Group { group, .. } => {
            assert_eq!(group.len(), 2);
            assert_eq!(group[0], "{stem}.tsx");
            assert_eq!(group[1], "{stem}.test.tsx");
        }
        SiblingRule::Directed { .. } => panic!("Expected Group rule"),
    }
}

#[test]
fn sibling_rule_ambiguous_match_and_group_rejected() {
    // Ambiguous: has both match/require AND group
    let toml_str = r#"
        [[structure.rules]]
        scope = "src/**"

        [[structure.rules.siblings]]
        match = "*.tsx"
        require = "{stem}.test.tsx"
        group = ["{stem}.tsx", "{stem}.spec.tsx"]
    "#;

    let result: Result<Config, _> = toml::from_str(toml_str);
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("Ambiguous") || err_msg.contains("ambiguous"));
}

#[test]
fn sibling_rule_empty_rejected() {
    // Neither match/require nor group
    let toml_str = r#"
        [[structure.rules]]
        scope = "src/**"

        [[structure.rules.siblings]]
        severity = "warn"
    "#;

    let result: Result<Config, _> = toml::from_str(toml_str);
    assert!(result.is_err());
}

#[test]
fn sibling_rule_directed_missing_require_rejected() {
    // Has match but no require
    let toml_str = r#"
        [[structure.rules]]
        scope = "src/**"

        [[structure.rules.siblings]]
        match = "*.tsx"
    "#;

    let result: Result<Config, _> = toml::from_str(toml_str);
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("require"));
}

#[test]
fn sibling_rule_directed_missing_match_rejected() {
    // Has require but no match
    let toml_str = r#"
        [[structure.rules]]
        scope = "src/**"

        [[structure.rules.siblings]]
        require = "{stem}.test.tsx"
    "#;

    let result: Result<Config, _> = toml::from_str(toml_str);
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("match"));
}

// =============================================================================
// Unknown Field Rejection Tests
// =============================================================================

#[test]
fn structure_unknown_field_rejected() {
    let toml_str = r"
        [structure]
        max_filez = 10
    ";

    let err = toml::from_str::<Config>(toml_str).unwrap_err();
    assert!(err.to_string().contains("max_filez"));
}

#[test]
fn rule_unknown_field_rejected() {
    let toml_str = r#"
        [[structure.rules]]
        scope = "src/**"
        warn_threshhold = 0.8
    "#;

    let err = toml::from_str::<Config>(toml_str).unwrap_err();
    assert!(err.to_string().contains("warn_threshhold"));
}

#[test]
fn rule_count_exclude_deserializes() {
    let toml_str = r#"
        [[structure.rules]]
        scope = "src/**"
        count_exclude = ["*.md", "src/**/*.gen"]
    "#;

    let config: Config = toml::from_str(toml_str).unwrap();
    assert_eq!(
        config.structure.rules[0].count_exclude,
        vec!["*.md", "src/**/*.gen"]
    );
}

#[test]
fn rule_count_exclude_defaults_to_empty() {
    let toml_str = r#"
        [[structure.rules]]
        scope = "src/**"
        max_files = 10
    "#;

    let config: Config = toml::from_str(toml_str).unwrap();
    assert!(config.structure.rules[0].count_exclude.is_empty());
}

#[test]
fn sibling_rule_unknown_field_rejected() {
    let toml_str = r#"
        [[structure.rules]]
        scope = "src/**"

        [[structure.rules.siblings]]
        match = "*.tsx"
        require = "{stem}.test.tsx"
        severty = "warn"
    "#;

    let err = toml::from_str::<Config>(toml_str).unwrap_err();
    assert!(err.to_string().contains("severty"));
}

// =============================================================================
// SiblingRequire Shape Tests
// =============================================================================

#[test]
fn sibling_require_rejects_table() {
    let toml_str = r#"
        [[structure.rules]]
        scope = "src/**"

        [[structure.rules.siblings]]
        match = "*.tsx"
        require = { pattern = "{stem}.test.tsx" }
    "#;

    let result: Result<Config, _> = toml::from_str(toml_str);
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("a sibling pattern string or an array of pattern strings"),
        "got: {err_msg}"
    );
}

#[test]
fn sibling_require_rejects_integer() {
    let toml_str = r#"
        [[structure.rules]]
        scope = "src/**"

        [[structure.rules.siblings]]
        match = "*.tsx"
        require = 5
    "#;

    let result: Result<Config, _> = toml::from_str(toml_str);
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("a sibling pattern string or an array of pattern strings"),
        "got: {err_msg}"
    );
}

#[test]
fn sibling_require_rejects_array_of_non_strings() {
    let toml_str = r#"
        [[structure.rules]]
        scope = "src/**"

        [[structure.rules.siblings]]
        match = "*.tsx"
        require = [1, 2]
    "#;

    let result: Result<Config, _> = toml::from_str(toml_str);
    assert!(result.is_err());
}

#[test]
fn sibling_require_accepts_string_and_array() {
    let toml_str = r#"
        [[structure.rules]]
        scope = "src/**"

        [[structure.rules.siblings]]
        match = "*.tsx"
        require = "{stem}.test.tsx"

        [[structure.rules.siblings]]
        match = "*.rs"
        require = ["{stem}_tests.rs", "{stem}.md"]
    "#;

    let config: Config = toml::from_str(toml_str).unwrap();
    let siblings = &config.structure.rules[0].siblings;
    assert_eq!(siblings.len(), 2);

    match &siblings[0] {
        SiblingRule::Directed { require, .. } => {
            assert_eq!(require.as_patterns(), vec!["{stem}.test.tsx"]);
        }
        SiblingRule::Group { .. } => panic!("Expected Directed rule"),
    }
    match &siblings[1] {
        SiblingRule::Directed { require, .. } => {
            assert_eq!(require.as_patterns(), vec!["{stem}_tests.rs", "{stem}.md"]);
        }
        SiblingRule::Group { .. } => panic!("Expected Directed rule"),
    }
}
