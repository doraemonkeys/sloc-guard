mod expires;
mod loader;
mod model;
pub mod presets;
mod remote;

pub use expires::{ExpiredRule, ExpiredRuleType, collect_expired_rules};
pub use loader::{ConfigLoader, FileConfigLoader, FileSystem, RealFileSystem};
pub use model::{
    CONFIG_VERSION, CONFIG_VERSION_V1, Config, ContentConfig, ContentRule, CustomLanguageConfig,
    DefaultConfig, ExcludeConfig, FileOverride, LanguageRule, RuleConfig, ScannerConfig,
    StructureConfig, StructureRule, UNLIMITED,
};
pub use remote::{
    clear_cache as clear_remote_cache, fetch_remote_config, fetch_remote_config_offline,
    is_remote_url,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_default_values() {
        let config = Config::default();
        assert_eq!(config.default.max_lines, 500);
        assert!(!config.default.extensions.is_empty());
    }

    #[test]
    fn config_merge_with_override() {
        let mut config = Config::default();
        let file_override = FileOverride {
            path: "src/legacy.rs".to_string(),
            max_lines: 800,
            reason: Some("Legacy code".to_string()),
        };
        config.overrides.push(file_override);

        assert_eq!(config.overrides.len(), 1);
        assert_eq!(config.overrides[0].max_lines, 800);
    }
}
