use super::*;

#[test]
fn default_config_has_expected_values() {
    let config = DefaultConfig::default();
    assert_eq!(config.max_lines, 500);
    assert!(config.skip_comments);
    assert!(config.skip_blank);
    assert!(!config.strict);
}

#[test]
fn config_deserialize_from_toml() {
    let toml_str = r#"
        [default]
        max_lines = 300
        extensions = ["rs"]
    "#;

    let config: Config = toml::from_str(toml_str).unwrap();
    assert_eq!(config.default.max_lines, 300);
    assert_eq!(config.default.extensions, vec!["rs"]);
}

#[test]
fn config_deserialize_with_rules() {
    let toml_str = r#"
        [default]
        max_lines = 500

        [rules.rust]
        extensions = ["rs"]
        max_lines = 300
    "#;

    let config: Config = toml::from_str(toml_str).unwrap();
    assert!(config.rules.contains_key("rust"));
    assert_eq!(config.rules["rust"].max_lines, Some(300));
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
    let toml_str = r"
        [default]
        max_lines = 500
        strict = true
    ";

    let config: Config = toml::from_str(toml_str).unwrap();
    assert!(config.default.strict);
}

#[test]
fn config_deserialize_strict_mode_default_false() {
    let toml_str = r"
        [default]
        max_lines = 500
    ";

    let config: Config = toml::from_str(toml_str).unwrap();
    assert!(!config.default.strict);
}

#[test]
fn config_deserialize_rule_with_warn_threshold() {
    let toml_str = r#"
        [default]
        max_lines = 500

        [rules.rust]
        extensions = ["rs"]
        max_lines = 300
        warn_threshold = 0.85
    "#;

    let config: Config = toml::from_str(toml_str).unwrap();
    assert!(config.rules.contains_key("rust"));
    assert_eq!(config.rules["rust"].max_lines, Some(300));
    assert_eq!(config.rules["rust"].warn_threshold, Some(0.85));
}

#[test]
fn config_deserialize_rule_without_warn_threshold() {
    let toml_str = r#"
        [default]
        max_lines = 500

        [rules.rust]
        extensions = ["rs"]
        max_lines = 300
    "#;

    let config: Config = toml::from_str(toml_str).unwrap();
    assert!(config.rules.contains_key("rust"));
    assert_eq!(config.rules["rust"].warn_threshold, None);
}

#[test]
fn config_deserialize_custom_language() {
    let toml_str = r#"
        [default]
        max_lines = 500

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
        [default]
        max_lines = 500

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
        [default]
        max_lines = 500

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
        [default]
        max_lines = 500

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
        [default]
        max_lines = 500

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
    let toml_str = r"
        [default]
        max_lines = 500

        [structure]
        max_files = 50
        max_dirs = 10
        warn_threshold = 0.9
    ";

    let config: Config = toml::from_str(toml_str).unwrap();
    assert_eq!(config.structure.max_files, Some(50));
    assert_eq!(config.structure.max_dirs, Some(10));
    assert_eq!(config.structure.warn_threshold, Some(0.9));
}

#[test]
fn config_deserialize_structure_rule_warn_threshold() {
    let toml_str = r"
        [default]
        max_lines = 500

        [structure]
        max_files = 50
        warn_threshold = 0.9

        [[structure.rules]]
        scope = 'src/generated/**'
        max_files = 100
        warn_threshold = 0.8
    ";

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
        version = "1"

        [default]
        max_lines = 500
    "#;

    let config: Config = toml::from_str(toml_str).unwrap();
    assert_eq!(config.version, Some("1".to_string()));
}

#[test]
fn config_deserialize_without_version() {
    let toml_str = r"
        [default]
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
