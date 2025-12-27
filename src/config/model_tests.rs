use super::*;

#[test]
fn content_config_has_expected_defaults() {
    let config = ContentConfig::default();
    assert_eq!(config.max_lines, 600);
    assert!(config.skip_comments);
    assert!(config.skip_blank);
    assert!(!config.strict);
}

#[test]
fn config_deserialize_v2_format() {
    let toml_str = r#"
        version = "2"

        [content]
        max_lines = 300
        extensions = ["rs"]
    "#;

    let config: Config = toml::from_str(toml_str).unwrap();
    assert_eq!(config.content.max_lines, 300);
    assert_eq!(config.content.extensions, vec!["rs"]);
}

#[test]
fn config_deserialize_with_content_rules() {
    let toml_str = r#"
        version = "2"

        [content]
        max_lines = 500

        [[content.rules]]
        pattern = "**/*.rs"
        max_lines = 300
    "#;

    let config: Config = toml::from_str(toml_str).unwrap();
    assert_eq!(config.content.rules.len(), 1);
    assert_eq!(config.content.rules[0].pattern, "**/*.rs");
    assert_eq!(config.content.rules[0].max_lines, 300);
}

#[test]
fn config_serialize_roundtrip() {
    let config = Config::default();
    let serialized = toml::to_string(&config).unwrap();
    let deserialized: Config = toml::from_str(&serialized).unwrap();
    assert_eq!(config, deserialized);
}

#[test]
fn config_deserialize_strict_mode() {
    let toml_str = r#"
        version = "2"

        [content]
        max_lines = 500
        strict = true
    "#;

    let config: Config = toml::from_str(toml_str).unwrap();
    assert!(config.content.strict);
}

#[test]
fn config_deserialize_strict_mode_default_false() {
    let toml_str = r"
        [content]
        max_lines = 500
    ";

    let config: Config = toml::from_str(toml_str).unwrap();
    assert!(!config.content.strict);
}

#[test]
fn config_deserialize_rule_with_warn_threshold() {
    let toml_str = r#"
        version = "2"

        [content]
        max_lines = 500

        [[content.rules]]
        pattern = "**/*.rs"
        max_lines = 300
        warn_threshold = 0.85
    "#;

    let config: Config = toml::from_str(toml_str).unwrap();
    assert_eq!(config.content.rules.len(), 1);
    assert_eq!(config.content.rules[0].max_lines, 300);
    assert_eq!(config.content.rules[0].warn_threshold, Some(0.85));
}

#[test]
fn config_deserialize_rule_without_warn_threshold() {
    let toml_str = r#"
        version = "2"

        [content]
        max_lines = 500

        [[content.rules]]
        pattern = "**/*.rs"
        max_lines = 300
    "#;

    let config: Config = toml::from_str(toml_str).unwrap();
    assert_eq!(config.content.rules.len(), 1);
    assert_eq!(config.content.rules[0].warn_threshold, None);
}

#[test]
fn config_deserialize_custom_language() {
    let toml_str = r#"
        version = "2"

        [languages.haskell]
        extensions = ["hs", "lhs"]
        single_line_comments = ["--"]
        multi_line_comments = [["{-", "-}"]]
    "#;

    let config: Config = toml::from_str(toml_str).unwrap();
    assert!(config.languages.contains_key("haskell"));
    let haskell = &config.languages["haskell"];
    assert_eq!(haskell.extensions, vec!["hs", "lhs"]);
    assert_eq!(haskell.single_line_comments, vec!["--"]);
    assert_eq!(
        haskell.multi_line_comments,
        vec![("{-".to_string(), "-}".to_string())]
    );
}

#[test]
fn config_deserialize_multiple_custom_languages() {
    let toml_str = r#"
        version = "2"

        [languages.haskell]
        extensions = ["hs"]
        single_line_comments = ["--"]
        multi_line_comments = [["{-", "-}"]]

        [languages.lua]
        extensions = ["lua"]
        single_line_comments = ["--"]
        multi_line_comments = [["--[[", "]]"]]
    "#;

    let config: Config = toml::from_str(toml_str).unwrap();
    assert_eq!(config.languages.len(), 2);
    assert!(config.languages.contains_key("haskell"));
    assert!(config.languages.contains_key("lua"));
}

#[test]
fn custom_language_config_default_values() {
    let config = CustomLanguageConfig::default();
    assert!(config.extensions.is_empty());
    assert!(config.single_line_comments.is_empty());
    assert!(config.multi_line_comments.is_empty());
}

#[test]
fn config_empty_languages_by_default() {
    let config = Config::default();
    assert!(config.languages.is_empty());
}

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

#[test]
fn config_version_defaults_to_none() {
    let config = Config::default();
    assert!(config.version.is_none());
}

#[test]
fn config_deserialize_with_version() {
    let toml_str = r#"
        version = "2"

        [content]
        max_lines = 500
    "#;

    let config: Config = toml::from_str(toml_str).unwrap();
    assert_eq!(config.version, Some("2".to_string()));
}

#[test]
fn config_deserialize_without_version() {
    let toml_str = r"
        [content]
        max_lines = 500
    ";

    let config: Config = toml::from_str(toml_str).unwrap();
    assert!(config.version.is_none());
}

#[test]
fn config_version_constant_is_two() {
    assert_eq!(CONFIG_VERSION, "2");
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
// TrendConfig Tests
// =============================================================================

#[test]
fn trend_config_default_all_none() {
    let config = TrendConfig::default();
    assert!(config.max_entries.is_none());
    assert!(config.max_age_days.is_none());
    assert!(config.min_interval_secs.is_none());
}

#[test]
fn config_has_default_trend_config() {
    let config = Config::default();
    assert!(config.trend.max_entries.is_none());
    assert!(config.trend.max_age_days.is_none());
    assert!(config.trend.min_interval_secs.is_none());
}

#[test]
fn config_deserialize_trend_section() {
    let toml_str = r"
        [trend]
        max_entries = 1000
        max_age_days = 365
        min_interval_secs = 3600
    ";

    let config: Config = toml::from_str(toml_str).unwrap();
    assert_eq!(config.trend.max_entries, Some(1000));
    assert_eq!(config.trend.max_age_days, Some(365));
    assert_eq!(config.trend.min_interval_secs, Some(3600));
}

#[test]
fn config_deserialize_trend_partial() {
    let toml_str = r"
        [trend]
        max_entries = 500
    ";

    let config: Config = toml::from_str(toml_str).unwrap();
    assert_eq!(config.trend.max_entries, Some(500));
    assert!(config.trend.max_age_days.is_none());
    assert!(config.trend.min_interval_secs.is_none());
}

#[test]
fn config_deserialize_trend_with_other_sections() {
    let toml_str = r#"
        version = "2"

        [content]
        max_lines = 500

        [baseline]
        ratchet = "warn"

        [trend]
        max_entries = 100
        max_age_days = 30
    "#;

    let config: Config = toml::from_str(toml_str).unwrap();
    assert_eq!(config.trend.max_entries, Some(100));
    assert_eq!(config.trend.max_age_days, Some(30));
    assert!(config.trend.min_interval_secs.is_none());
    // Verify other sections are unaffected
    assert_eq!(config.content.max_lines, 500);
    assert_eq!(config.baseline.ratchet, Some(RatchetMode::Warn));
}

#[test]
fn trend_config_equality() {
    let config1 = TrendConfig {
        max_entries: Some(100),
        max_age_days: Some(30),
        min_interval_secs: Some(60),
        min_code_delta: Some(10),
    };
    let config2 = TrendConfig {
        max_entries: Some(100),
        max_age_days: Some(30),
        min_interval_secs: Some(60),
        min_code_delta: Some(10),
    };
    let config3 = TrendConfig {
        max_entries: Some(200),
        ..Default::default()
    };

    assert_eq!(config1, config2);
    assert_ne!(config1, config3);
}

#[test]
fn config_serialize_includes_trend() {
    let mut config = Config::default();
    config.trend.max_entries = Some(500);
    config.trend.max_age_days = Some(90);

    let serialized = toml::to_string(&config).unwrap();
    assert!(serialized.contains("[trend]"));
    assert!(serialized.contains("max_entries = 500"));
    assert!(serialized.contains("max_age_days = 90"));
}

#[test]
fn trend_config_roundtrip_serialization() {
    let original = TrendConfig {
        max_entries: Some(1000),
        max_age_days: Some(365),
        min_interval_secs: Some(3600),
        min_code_delta: Some(25),
    };

    let json = serde_json::to_string(&original).unwrap();
    let parsed: TrendConfig = serde_json::from_str(&json).unwrap();

    assert_eq!(original, parsed);
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
