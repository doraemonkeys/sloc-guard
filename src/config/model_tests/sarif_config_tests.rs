use super::*;
use crate::output::SarifLevel;

#[test]
fn sarif_config_default_floor_is_error() {
    // The whole point of the feature: by default only real violations (error)
    // reach SARIF, keeping approaching-limit advisories out of the Security tab.
    let config = SarifConfig::default();
    assert_eq!(config.min_level, SarifLevel::Error);
}

#[test]
fn config_default_sarif_min_level_is_error() {
    let config = Config::default();
    assert_eq!(config.sarif.min_level, SarifLevel::Error);
}

#[test]
fn config_without_sarif_section_defaults_to_error_floor() {
    let toml_str = r#"
        version = "2"

        [content]
        max_lines = 500
    "#;

    let config: Config = toml::from_str(toml_str).unwrap();
    assert_eq!(config.sarif.min_level, SarifLevel::Error);
}

#[test]
fn config_sarif_min_level_opt_in_to_warning() {
    let toml_str = r#"
        version = "2"

        [sarif]
        min_level = "warning"
    "#;

    let config: Config = toml::from_str(toml_str).unwrap();
    assert_eq!(config.sarif.min_level, SarifLevel::Warning);
}

#[test]
fn config_sarif_min_level_note_emits_everything() {
    let toml_str = r#"
        version = "2"

        [sarif]
        min_level = "note"
    "#;

    let config: Config = toml::from_str(toml_str).unwrap();
    assert_eq!(config.sarif.min_level, SarifLevel::Note);
}

#[test]
fn config_sarif_empty_section_uses_default_floor() {
    // Declaring [sarif] without min_level still falls back to the error default.
    let toml_str = r#"
        version = "2"

        [sarif]
    "#;

    let config: Config = toml::from_str(toml_str).unwrap();
    assert_eq!(config.sarif.min_level, SarifLevel::Error);
}

#[test]
fn config_sarif_min_level_rejects_unknown_value() {
    let toml_str = r#"
        version = "2"

        [sarif]
        min_level = "bogus"
    "#;

    assert!(toml::from_str::<Config>(toml_str).is_err());
}
