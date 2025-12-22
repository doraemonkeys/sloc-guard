//! Structure checker tests organized by domain.

mod basic_tests;
mod depth_tests;
mod limit_tests;
mod override_tests;
mod rule_priority_tests;
mod sibling_tests;

use crate::checker::structure::*;
use crate::config::{StructureConfig, StructureOverride, StructureRule, UNLIMITED};

/// Create a default config with no limits set.
fn default_config() -> StructureConfig {
    StructureConfig::default()
}

/// Create a config with only `max_files` set.
fn config_with_file_limit(max_files: i64) -> StructureConfig {
    StructureConfig {
        max_files: Some(max_files),
        ..Default::default()
    }
}

/// Create a config with only `max_dirs` set.
fn config_with_dir_limit(max_dirs: i64) -> StructureConfig {
    StructureConfig {
        max_dirs: Some(max_dirs),
        ..Default::default()
    }
}

/// Create a config with only `max_depth` set.
fn config_with_depth_limit(max_depth: i64) -> StructureConfig {
    StructureConfig {
        max_depth: Some(max_depth),
        ..Default::default()
    }
}

/// Create a basic structure rule with the given scope and `max_files`.
fn make_rule(scope: &str, max_files: Option<i64>) -> StructureRule {
    StructureRule {
        scope: scope.to_string(),
        max_files,
        max_dirs: None,
        max_depth: None,
        warn_threshold: None,
        allow_extensions: vec![],
        allow_patterns: vec![],
        file_naming_pattern: None,
        relative_depth: false,
        file_pattern: None,
        require_sibling: None,
        deny_extensions: vec![],
        deny_patterns: vec![],

        deny_files: vec![],
        deny_dirs: vec![],
    }
}
