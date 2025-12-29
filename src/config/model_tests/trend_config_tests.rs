use super::*;

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
