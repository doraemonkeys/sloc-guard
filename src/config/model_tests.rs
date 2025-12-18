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
