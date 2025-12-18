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
    assert!(config.ignore.is_empty());
    assert!(config.rules.is_empty());
}

#[test]
fn config_structure_default_empty() {
    let config = Config::default();
    assert!(config.structure.max_files.is_none());
    assert!(config.structure.max_dirs.is_none());
    assert!(config.structure.ignore.is_empty());
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
        ignore = ["*.md", ".gitkeep"]
    "#;

    let config: Config = toml::from_str(toml_str).unwrap();
    assert_eq!(config.structure.max_files, Some(10));
    assert_eq!(config.structure.max_dirs, Some(5));
    assert_eq!(config.structure.ignore, vec!["*.md", ".gitkeep"]);
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
        pattern = "src/generated/**"
        max_files = 50

        [[structure.rules]]
        pattern = "tests/**"
        max_files = 20
        max_dirs = 10
    "#;

    let config: Config = toml::from_str(toml_str).unwrap();
    assert_eq!(config.structure.rules.len(), 2);

    let rule0 = &config.structure.rules[0];
    assert_eq!(rule0.pattern, "src/generated/**");
    assert_eq!(rule0.max_files, Some(50));
    assert!(rule0.max_dirs.is_none());

    let rule1 = &config.structure.rules[1];
    assert_eq!(rule1.pattern, "tests/**");
    assert_eq!(rule1.max_files, Some(20));
    assert_eq!(rule1.max_dirs, Some(10));
}

#[test]
fn config_deserialize_structure_only_rules() {
    let toml_str = r#"
        [default]
        max_lines = 500

        [[structure.rules]]
        pattern = "vendor/**"
        max_files = 100
    "#;

    let config: Config = toml::from_str(toml_str).unwrap();
    assert!(config.structure.max_files.is_none());
    assert!(config.structure.max_dirs.is_none());
    assert_eq!(config.structure.rules.len(), 1);
    assert_eq!(config.structure.rules[0].pattern, "vendor/**");
}
