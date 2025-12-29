use super::*;

#[test]
fn stats_config_default_all_empty() {
    let config = StatsConfig::default();
    assert!(config.report.exclude.is_empty());
    assert!(config.report.top_count.is_none());
    assert!(config.report.breakdown_by.is_none());
    assert!(config.report.trend_since.is_none());
}

#[test]
fn config_has_default_stats_config() {
    let config = Config::default();
    assert!(config.stats.report.exclude.is_empty());
    assert!(config.stats.report.top_count.is_none());
}

#[test]
fn config_deserialize_stats_report_section() {
    let toml_str = r#"
        [stats.report]
        exclude = ["trend", "breakdown"]
        top_count = 20
        breakdown_by = "dir"
        trend_since = "7d"
    "#;

    let config: Config = toml::from_str(toml_str).unwrap();
    assert_eq!(config.stats.report.exclude, vec!["trend", "breakdown"]);
    assert_eq!(config.stats.report.top_count, Some(20));
    assert_eq!(config.stats.report.breakdown_by, Some("dir".to_string()));
    assert_eq!(config.stats.report.trend_since, Some("7d".to_string()));
}

#[test]
fn config_deserialize_stats_report_partial() {
    let toml_str = r"
        [stats.report]
        top_count = 15
    ";

    let config: Config = toml::from_str(toml_str).unwrap();
    assert!(config.stats.report.exclude.is_empty());
    assert_eq!(config.stats.report.top_count, Some(15));
    assert!(config.stats.report.breakdown_by.is_none());
    assert!(config.stats.report.trend_since.is_none());
}

#[test]
fn config_deserialize_stats_report_exclude_only() {
    let toml_str = r#"
        [stats.report]
        exclude = ["files"]
    "#;

    let config: Config = toml::from_str(toml_str).unwrap();
    assert_eq!(config.stats.report.exclude, vec!["files"]);
}

#[test]
fn config_deserialize_stats_with_other_sections() {
    let toml_str = r#"
        version = "2"

        [content]
        max_lines = 500

        [trend]
        max_entries = 100

        [stats.report]
        top_count = 25
        breakdown_by = "lang"
    "#;

    let config: Config = toml::from_str(toml_str).unwrap();
    // Verify stats config parsed correctly
    assert_eq!(config.stats.report.top_count, Some(25));
    assert_eq!(config.stats.report.breakdown_by, Some("lang".to_string()));
    // Verify other sections are unaffected
    assert_eq!(config.content.max_lines, 500);
    assert_eq!(config.trend.max_entries, Some(100));
}

#[test]
fn stats_report_config_roundtrip_serialization() {
    let original = StatsReportConfig {
        exclude: vec!["trend".to_string()],
        top_count: Some(10),
        breakdown_by: Some("dir".to_string()),
        trend_since: Some("30d".to_string()),
    };

    let json = serde_json::to_string(&original).unwrap();
    let parsed: StatsReportConfig = serde_json::from_str(&json).unwrap();

    assert_eq!(original, parsed);
}

#[test]
fn config_serialize_includes_stats() {
    let mut config = Config::default();
    config.stats.report.top_count = Some(15);
    config.stats.report.exclude = vec!["trend".to_string()];

    let serialized = toml::to_string(&config).unwrap();
    assert!(serialized.contains("[stats.report]"));
    assert!(serialized.contains("top_count = 15"));
    assert!(serialized.contains("exclude"));
}
