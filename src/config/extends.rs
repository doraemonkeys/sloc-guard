//! Extends chain resolution for config inheritance.
//!
//! Handles resolving the `extends` field in configs, supporting:
//! - Local file paths (absolute or relative)
//! - Remote URLs (http/https)
//! - Presets (preset:name)
//! - Circular reference detection
//! - Depth limiting

use std::path::Path;

use indexmap::IndexSet;

use crate::error::{ConfigSource, Result, SlocGuardError};

use super::filesystem::FileSystem;
use super::merge::{merge_toml_values, strip_reset_markers, validate_reset_positions};
use super::presets;
use super::remote::{FetchPolicy, fetch_remote_config, is_remote_url};

/// A configuration value paired with its source.
///
/// Used during extends resolution to track which config contributed which values,
/// enabling the `explain --sources` feature to show field origins.
#[derive(Debug, Clone)]
pub struct SourcedConfig {
    /// The source of this configuration (file, remote URL, or preset).
    pub source: ConfigSource,
    /// The raw TOML value before merging.
    pub value: toml::Value,
}

/// Maximum depth of extends chain to prevent stack overflow from deeply nested configs.
///
/// Depth starts at 0 (the initial config) and increments with each `extends` resolution.
/// A value of 10 allows depths 0..=10 inclusive, meaning a chain of up to 11 config files.
pub const MAX_EXTENDS_DEPTH: usize = 10;

/// Resolves extends chains for config inheritance.
///
/// This struct encapsulates all the logic for resolving `extends` fields in configs,
/// including local/remote/preset resolution, circular detection, and depth limiting.
pub struct ExtendsResolver<'a, F: FileSystem> {
    fs: &'a F,
    fetch_policy: FetchPolicy,
    project_root: Option<&'a Path>,
}

impl<'a, F: FileSystem> ExtendsResolver<'a, F> {
    /// Create a new extends resolver.
    pub const fn new(fs: &'a F, fetch_policy: FetchPolicy, project_root: Option<&'a Path>) -> Self {
        Self {
            fs,
            fetch_policy,
            project_root,
        }
    }

    /// Parse content to `toml::Value` with precise syntax error reporting.
    pub fn parse_value_with_location(
        content: &str,
        source: Option<ConfigSource>,
    ) -> Result<toml::Value> {
        toml::from_str(content).map_err(|e| SlocGuardError::syntax_from_toml(&e, content, source))
    }

    /// Load config with extends chain, optionally tracking sources.
    ///
    /// When `sources` is `Some`, records each config source in the chain for
    /// `explain --sources` output. When `None`, skips source tracking overhead.
    pub fn load_with_extends(
        &self,
        path: &Path,
        visited: &mut IndexSet<String>,
        sources: Option<&mut Vec<SourcedConfig>>,
        depth: usize,
    ) -> Result<(toml::Value, Option<String>)> {
        let content =
            self.fs
                .read_to_string(path)
                .map_err(|source| SlocGuardError::FileAccess {
                    path: path.to_path_buf(),
                    source,
                })?;

        self.load_with_extends_from_content(path, &content, visited, sources, depth)
    }

    /// Continue extends chain with pre-read content, optionally tracking sources.
    pub fn load_with_extends_from_content(
        &self,
        path: &Path,
        content: &str,
        visited: &mut IndexSet<String>,
        sources: Option<&mut Vec<SourcedConfig>>,
        depth: usize,
    ) -> Result<(toml::Value, Option<String>)> {
        let source = ConfigSource::file(path);
        let config_value = Self::parse_value_with_location(content, Some(source))?;
        self.load_with_extends_from_value(path, config_value, visited, sources, depth)
    }

    /// Continue extends chain with pre-parsed value, optionally tracking sources.
    pub fn load_with_extends_from_value(
        &self,
        path: &Path,
        config_value: toml::Value,
        visited: &mut IndexSet<String>,
        sources: Option<&mut Vec<SourcedConfig>>,
        depth: usize,
    ) -> Result<(toml::Value, Option<String>)> {
        if depth > MAX_EXTENDS_DEPTH {
            return Err(SlocGuardError::ExtendsTooDeep {
                depth,
                max: MAX_EXTENDS_DEPTH,
                chain: visited.iter().cloned().collect(),
            });
        }

        let canonical =
            self.fs
                .canonicalize(path)
                .map_err(|source| SlocGuardError::FileAccess {
                    path: path.to_path_buf(),
                    source,
                })?;
        let key = canonical.to_string_lossy().to_string();

        if !visited.insert(key.clone()) {
            let mut chain: Vec<String> = visited.iter().cloned().collect();
            chain.push(key);
            return Err(SlocGuardError::CircularExtends { chain });
        }

        self.process_config_value(config_value, Some(path), visited, sources, depth)
    }

    /// Load remote config with extends chain, optionally tracking sources.
    fn load_remote_with_extends(
        &self,
        url: &str,
        expected_hash: Option<&str>,
        visited: &mut IndexSet<String>,
        mut sources: Option<&mut Vec<SourcedConfig>>,
        depth: usize,
    ) -> Result<(toml::Value, Option<String>)> {
        if depth > MAX_EXTENDS_DEPTH {
            return Err(SlocGuardError::ExtendsTooDeep {
                depth,
                max: MAX_EXTENDS_DEPTH,
                chain: visited.iter().cloned().collect(),
            });
        }

        let url_key = url.to_string();
        if !visited.insert(url_key.clone()) {
            let mut chain: Vec<String> = visited.iter().cloned().collect();
            chain.push(url_key);
            return Err(SlocGuardError::CircularExtends { chain });
        }

        let content =
            fetch_remote_config(url, self.project_root, expected_hash, self.fetch_policy)?;
        let source = ConfigSource::remote(url);
        let config_value = Self::parse_value_with_location(&content, Some(source))?;

        // Process extends chain (base sources come first via recursion)
        // Clone only when tracking sources; move otherwise for efficiency
        if sources.is_some() {
            let (merged_value, preset_used) = self.process_config_value(
                config_value.clone(),
                None, // No base_path for remote configs
                visited,
                sources.as_deref_mut(),
                depth,
            )?;

            // Record remote source (after recursion, so base sources come first)
            // INVARIANT: sources was Some, unwrap is safe
            sources.as_mut().unwrap().push(SourcedConfig {
                source: ConfigSource::remote(url),
                value: config_value,
            });

            Ok((merged_value, preset_used))
        } else {
            self.process_config_value(
                config_value, // Move, no clone needed
                None,
                visited,
                None,
                depth,
            )
        }
    }

    /// Process pre-parsed config value, optionally tracking sources.
    ///
    /// This is the core method that handles extends resolution and optional source collection.
    /// When `sources` is `Some`, records each config in the inheritance chain.
    //
    // Note: We use `as_mut().map(|v| &mut **v)` instead of `as_deref_mut()` because we need
    // `Option<&mut Vec<T>>` (for `push()`), not `Option<&mut [T]>` (slice) which `as_deref_mut` returns.
    // The `useless_let_if_seq` lint is suppressed because the conditional mutation pattern
    // is clearer with explicit `let mut` for this complex extends resolution logic.
    #[allow(clippy::option_as_ref_deref, clippy::useless_let_if_seq)]
    fn process_config_value(
        &self,
        config_value: toml::Value,
        base_path: Option<&Path>,
        visited: &mut IndexSet<String>,
        mut sources: Option<&mut Vec<SourcedConfig>>,
        depth: usize,
    ) -> Result<(toml::Value, Option<String>)> {
        let extends_value = config_value
            .get("extends")
            .and_then(toml::Value::as_str)
            .map(String::from);

        let extends_sha256 = config_value
            .get("extends_sha256")
            .and_then(toml::Value::as_str)
            .map(String::from);

        let (mut merged_value, preset_used) = if let Some(extends) = extends_value {
            let (base_value, preset_used) = if let Some(preset_name) =
                extends.strip_prefix("preset:")
            {
                let preset_value = presets::load_preset(preset_name)?;
                // Record preset source if tracking
                if let Some(src_vec) = sources.as_mut() {
                    src_vec.push(SourcedConfig {
                        source: ConfigSource::preset(preset_name),
                        value: preset_value.clone(),
                    });
                }
                (preset_value, Some(preset_name.to_string()))
            } else if is_remote_url(&extends) {
                self.load_remote_with_extends(
                    &extends,
                    extends_sha256.as_deref(),
                    visited,
                    sources.as_deref_mut(),
                    depth + 1,
                )?
            } else {
                let extends_path = Path::new(&extends);
                let resolved_path = if extends_path.is_absolute() {
                    extends_path.to_path_buf()
                } else if let Some(base) = base_path {
                    base.parent()
                        .unwrap_or_else(|| Path::new("."))
                        .join(extends_path)
                } else {
                    return Err(SlocGuardError::ExtendsResolution {
                        path: extends,
                        base: "remote config".to_string(),
                    });
                };
                self.load_with_extends(&resolved_path, visited, sources.as_deref_mut(), depth + 1)?
            };
            // Record source before merge (clone only when tracking is needed)
            if let Some(src_vec) = sources.as_mut()
                && let Some(path) = base_path
            {
                src_vec.push(SourcedConfig {
                    source: ConfigSource::file(path),
                    value: config_value.clone(),
                });
            }
            (merge_toml_values(base_value, config_value), preset_used)
        } else {
            // Record source before return (clone only when tracking is needed)
            if let Some(src_vec) = sources.as_mut()
                && let Some(path) = base_path
            {
                src_vec.push(SourcedConfig {
                    source: ConfigSource::file(path),
                    value: config_value.clone(),
                });
            }
            (config_value, None)
        };

        if let Some(table) = merged_value.as_table_mut() {
            table.remove("extends");
            table.remove("extends_sha256");
        }

        validate_reset_positions(&merged_value, "")?;
        strip_reset_markers(&mut merged_value);

        Ok((merged_value, preset_used))
    }
}
