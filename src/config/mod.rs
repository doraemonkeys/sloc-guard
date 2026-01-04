mod expires;
mod extends;
mod filesystem;
mod loader;
pub(crate) mod merge;
mod model;
pub mod presets;
mod remote;
mod validation;

pub use expires::{ExpiredRule, ExpiredRuleType, collect_expired_rules};
pub use extends::SourcedConfig;
pub use filesystem::{FileSystem, RealFileSystem};
pub use loader::{ConfigLoader, FileConfigLoader, LoadResult, LoadResultWithSources};
pub use merge::RESET_MARKER;
pub use model::{
    BaselineConfig, CONFIG_VERSION, CheckConfig, Config, ContentConfig, ContentRule,
    CustomLanguageConfig, DEFAULT_MAX_LINES, RatchetMode, ScannerConfig, SiblingRequire,
    SiblingRule, SiblingSeverity, StatsConfig, StatsReportConfig, StructureConfig, StructureRule,
    TrendConfig, UNLIMITED,
};
pub use remote::{
    FetchPolicy, clear_cache as clear_remote_cache, fetch_remote_config, is_remote_url,
};
pub(crate) use validation::validate_config_semantics;

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
