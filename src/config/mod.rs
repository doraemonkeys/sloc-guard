mod expires;
mod loader;
mod model;
pub mod presets;
mod remote;

pub use expires::{ExpiredRule, ExpiredRuleType, collect_expired_rules};
pub use loader::{ConfigLoader, FileConfigLoader, FileSystem, LoadResult, RealFileSystem};
pub use model::{
    BaselineConfig, CONFIG_VERSION, Config, ContentConfig, ContentRule, CustomLanguageConfig,
    DEFAULT_MAX_LINES, RatchetMode, ScannerConfig, SiblingRequire, SiblingRule, SiblingSeverity,
    StatsConfig, StatsReportConfig, StructureConfig, StructureRule, TrendConfig, UNLIMITED,
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
        assert_eq!(config.content.max_lines, DEFAULT_MAX_LINES);
        assert!(!config.content.extensions.is_empty());
    }

    #[test]
    fn config_has_v2_sections() {
        let config = Config::default();
        // Verify V2 structure exists
        assert!(config.scanner.gitignore);
        assert!(config.content.skip_comments);
        assert!(config.structure.rules.is_empty());
    }
}
