use super::*;

#[test]
fn content_config_has_expected_defaults() {
    let config = ContentConfig::default();
    assert_eq!(config.max_lines, 600);
    assert!(config.skip_comments);
    assert!(config.skip_blank);
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
fn check_config_has_expected_defaults() {
    let config = CheckConfig::default();
    assert!(!config.warnings_as_errors);
    assert!(!config.fail_fast);
}

#[test]
fn config_deserialize_check_section() {
    let toml_str = r#"
        version = "2"

        [check]
        warnings_as_errors = true
        fail_fast = true
    "#;

    let config: Config = toml::from_str(toml_str).unwrap();
    assert!(config.check.warnings_as_errors);
    assert!(config.check.fail_fast);
}

#[test]
fn config_deserialize_check_section_partial() {
    let toml_str = r#"
        version = "2"

        [check]
        warnings_as_errors = true
    "#;

    let config: Config = toml::from_str(toml_str).unwrap();
    assert!(config.check.warnings_as_errors);
    assert!(!config.check.fail_fast);
}

#[test]
fn config_deserialize_check_section_default() {
    let toml_str = r"
        [content]
        max_lines = 500
    ";

    let config: Config = toml::from_str(toml_str).unwrap();
    assert!(!config.check.warnings_as_errors);
    assert!(!config.check.fail_fast);
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
