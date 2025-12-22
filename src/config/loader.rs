use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::error::{Result, SlocGuardError};

use super::Config;
use super::model::{CONFIG_VERSION, CONFIG_VERSION_V1, ContentOverride, ContentRule, LanguageRule};
use super::presets;
use super::remote::{fetch_remote_config, fetch_remote_config_offline, is_remote_url};

/// Trait for loading configuration from various sources.
pub trait ConfigLoader {
    /// Load configuration from the default location.
    ///
    /// # Errors
    /// Returns an error if the config file cannot be read or parsed.
    fn load(&self) -> Result<Config>;

    /// Load configuration from a specific path.
    ///
    /// # Errors
    /// Returns an error if the file cannot be read or parsed.
    fn load_from_path(&self, path: &Path) -> Result<Config>;

    /// Load configuration without resolving extends.
    ///
    /// # Errors
    /// Returns an error if the config file cannot be read or parsed.
    fn load_without_extends(&self) -> Result<Config>;

    /// Load configuration from a specific path without resolving extends.
    ///
    /// # Errors
    /// Returns an error if the file cannot be read or parsed.
    fn load_from_path_without_extends(&self, path: &Path) -> Result<Config>;
}

const LOCAL_CONFIG_NAME: &str = ".sloc-guard.toml";
const USER_CONFIG_DIR: &str = "sloc-guard";
const USER_CONFIG_NAME: &str = "config.toml";

/// Validate config version. Returns an error if version is unsupported.
/// Returns `true` if version is missing or V1 (requires migration).
fn validate_config_version(config: &Config) -> Result<bool> {
    // Reject deprecated path_rules - must use content.rules instead
    if !config.path_rules.is_empty() {
        return Err(SlocGuardError::Config(
            "Deprecated: [[path_rules]] is no longer supported. \
             Please migrate to [[content.rules]] format. Example:\n\n\
             # Old format (no longer supported):\n\
             # [[path_rules]]\n\
             # pattern = \"src/generated/**\"\n\
             # max_lines = 1000\n\n\
             # New format:\n\
             [[content.rules]]\n\
             pattern = \"src/generated/**\"\n\
             max_lines = 1000"
                .to_string(),
        ));
    }

    match &config.version {
        None => Ok(true),                              // Missing version - needs migration
        Some(v) if v == CONFIG_VERSION => Ok(false),   // V2 - no migration
        Some(v) if v == CONFIG_VERSION_V1 => Ok(true), // V1 - needs migration
        Some(v) => Err(SlocGuardError::Config(format!(
            "Unsupported config version '{v}'. Supported versions: '{CONFIG_VERSION_V1}', '{CONFIG_VERSION}'. \
             Please check for updates or adjust your config."
        ))),
    }
}

/// Migrate V1 config fields to V2 structure.
/// This function populates `scanner`/`content` fields from legacy fields.
fn migrate_v1_to_v2(config: &mut Config) {
    // Migrate default.gitignore -> scanner.gitignore
    config.scanner.gitignore = config.default.gitignore;

    // Migrate exclude.patterns -> scanner.exclude
    if !config.exclude.patterns.is_empty() {
        config.scanner.exclude = config.exclude.patterns.clone();
    }

    // Migrate default.* -> content.*
    config.content.extensions = config.default.extensions.clone();
    config.content.max_lines = config.default.max_lines;
    config.content.warn_threshold = config.default.warn_threshold;
    config.content.skip_comments = config.default.skip_comments;
    config.content.skip_blank = config.default.skip_blank;
    config.content.strict = config.default.strict;

    // Note: path_rules migration removed - V1 path_rules are now rejected with an error.
    // Users must manually migrate to [[content.rules]] format.

    // Migrate overrides -> content.overrides
    for ovr in &config.overrides {
        config.content.overrides.push(ContentOverride {
            path: ovr.path.clone(),
            max_lines: ovr.max_lines,
            reason: ovr
                .reason
                .clone()
                .unwrap_or_else(|| "Legacy override (migrated from v1)".to_string()),
        });
    }

    // Migrate rules (extension-based) -> content.languages
    // V1 format: [rules.python] with extensions = ["py"]
    // V2 format: [content.languages.py] - key is the actual file extension
    for rule in config.rules.values() {
        for ext in &rule.extensions {
            config.content.languages.insert(
                ext.clone(),
                LanguageRule {
                    max_lines: rule.max_lines,
                    warn_threshold: rule.warn_threshold,
                    skip_comments: rule.skip_comments,
                    skip_blank: rule.skip_blank,
                },
            );
        }
    }
}

/// Expand `[content.languages.X]` entries into `[[content.rules]]` with pattern `**/*.X`.
/// Expanded rules are inserted at HEAD of `content.rules` so explicit rules override them.
fn expand_language_rules(config: &mut Config) {
    if config.content.languages.is_empty() {
        return;
    }

    // Collect expanded rules (sorted by extension for deterministic order)
    let mut extensions: Vec<_> = config.content.languages.keys().cloned().collect();
    extensions.sort();

    let expanded_rules: Vec<ContentRule> = extensions
        .into_iter()
        .map(|ext| {
            let lang_rule = config
                .content
                .languages
                .get(&ext)
                .expect("key exists: iterating over collected keys");
            ContentRule {
                pattern: format!("**/*.{ext}"),
                max_lines: lang_rule.max_lines.unwrap_or(config.content.max_lines),
                warn_threshold: lang_rule.warn_threshold,
                skip_comments: lang_rule.skip_comments,
                skip_blank: lang_rule.skip_blank,
            }
        })
        .collect();

    // Insert at HEAD (so explicit [[content.rules]] override language rules)
    config.content.rules.splice(0..0, expanded_rules);

    // Clear languages to avoid double processing in ThresholdChecker
    config.content.languages.clear();
}

/// Trait for filesystem operations (for testability).
pub trait FileSystem {
    /// Read file contents as a string.
    ///
    /// # Errors
    /// Returns an error if the file cannot be read.
    fn read_to_string(&self, path: &Path) -> std::io::Result<String>;

    /// Check if a path exists.
    fn exists(&self, path: &Path) -> bool;

    /// Get the current working directory.
    ///
    /// # Errors
    /// Returns an error if the current directory cannot be determined.
    fn current_dir(&self) -> std::io::Result<PathBuf>;

    /// Get the user's home directory.
    fn home_dir(&self) -> Option<PathBuf>;
}

/// Real filesystem implementation.
#[derive(Debug, Default, Clone, Copy)]
pub struct RealFileSystem;

impl FileSystem for RealFileSystem {
    fn read_to_string(&self, path: &Path) -> std::io::Result<String> {
        std::fs::read_to_string(path)
    }

    fn exists(&self, path: &Path) -> bool {
        path.exists()
    }

    fn current_dir(&self) -> std::io::Result<PathBuf> {
        std::env::current_dir()
    }

    fn home_dir(&self) -> Option<PathBuf> {
        dirs_home()
    }
}

fn dirs_home() -> Option<PathBuf> {
    #[cfg(windows)]
    {
        std::env::var_os("USERPROFILE").map(PathBuf::from)
    }
    #[cfg(not(windows))]
    {
        std::env::var_os("HOME").map(PathBuf::from)
    }
}

/// Loads configuration from the filesystem.
///
/// Search order:
/// 1. `.sloc-guard.toml` in current directory
/// 2. `$HOME/.config/sloc-guard/config.toml`
/// 3. Returns `Config::default()` if no config found
#[derive(Debug)]
pub struct FileConfigLoader<F: FileSystem = RealFileSystem> {
    fs: F,
    offline: bool,
    project_root: Option<PathBuf>,
}

impl Default for FileConfigLoader<RealFileSystem> {
    fn default() -> Self {
        Self::new()
    }
}

impl FileConfigLoader<RealFileSystem> {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            fs: RealFileSystem,
            offline: false,
            project_root: None,
        }
    }

    /// Create a loader with offline mode and project root options.
    #[must_use]
    pub const fn with_options(offline: bool, project_root: Option<PathBuf>) -> Self {
        Self {
            fs: RealFileSystem,
            offline,
            project_root,
        }
    }
}

impl<F: FileSystem> FileConfigLoader<F> {
    #[must_use]
    pub const fn with_fs(fs: F) -> Self {
        Self {
            fs,
            offline: false,
            project_root: None,
        }
    }

    fn local_config_path(&self) -> Option<PathBuf> {
        self.fs
            .current_dir()
            .ok()
            .map(|dir| dir.join(LOCAL_CONFIG_NAME))
    }

    fn user_config_path(&self) -> Option<PathBuf> {
        self.fs.home_dir().map(|home| {
            home.join(".config")
                .join(USER_CONFIG_DIR)
                .join(USER_CONFIG_NAME)
        })
    }

    fn parse_config(content: &str) -> Result<Config> {
        let mut config: Config = toml::from_str(content).map_err(SlocGuardError::from)?;
        let needs_migration = validate_config_version(&config)?;

        // Migrate V1 config to V2 structure if needed
        if needs_migration {
            migrate_v1_to_v2(&mut config);
        }

        // Expand [content.languages.X] to [[content.rules]] with pattern **/*.X
        expand_language_rules(&mut config);

        Ok(config)
    }

    fn load_with_extends(&self, path: &Path, visited: &mut HashSet<String>) -> Result<toml::Value> {
        let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        let key = canonical.to_string_lossy().to_string();

        if !visited.insert(key) {
            return Err(SlocGuardError::Config(format!(
                "Circular extends detected: {}",
                canonical.display()
            )));
        }

        let content = self
            .fs
            .read_to_string(path)
            .map_err(|source| SlocGuardError::FileRead {
                path: path.to_path_buf(),
                source,
            })?;

        self.process_config_content(&content, Some(path), visited)
    }

    fn load_remote_with_extends(
        &self,
        url: &str,
        expected_hash: Option<&str>,
        visited: &mut HashSet<String>,
    ) -> Result<toml::Value> {
        if !visited.insert(url.to_string()) {
            return Err(SlocGuardError::Config(format!(
                "Circular extends detected: {url}"
            )));
        }

        let content = if self.offline {
            fetch_remote_config_offline(url, self.project_root.as_deref(), expected_hash)?
        } else {
            fetch_remote_config(url, self.project_root.as_deref(), expected_hash)?
        };
        self.process_config_content(&content, None, visited)
    }

    fn process_config_content(
        &self,
        content: &str,
        base_path: Option<&Path>,
        visited: &mut HashSet<String>,
    ) -> Result<toml::Value> {
        let mut config_value: toml::Value =
            toml::from_str(content).map_err(SlocGuardError::from)?;

        let extends_value = config_value
            .get("extends")
            .and_then(toml::Value::as_str)
            .map(String::from);

        let extends_sha256 = config_value
            .get("extends_sha256")
            .and_then(toml::Value::as_str)
            .map(String::from);

        if let Some(extends) = extends_value {
            let base_value = if let Some(preset_name) = extends.strip_prefix("preset:") {
                presets::load_preset(preset_name)?
            } else if is_remote_url(&extends) {
                self.load_remote_with_extends(&extends, extends_sha256.as_deref(), visited)?
            } else {
                let extends_path = Path::new(&extends);
                let resolved_path = if extends_path.is_absolute() {
                    extends_path.to_path_buf()
                } else if let Some(base) = base_path {
                    base.parent()
                        .unwrap_or_else(|| Path::new("."))
                        .join(extends_path)
                } else {
                    return Err(SlocGuardError::Config(format!(
                        "Cannot resolve relative path '{extends}' from remote config"
                    )));
                };
                self.load_with_extends(&resolved_path, visited)?
            };
            config_value = merge_toml_values(base_value, config_value);
        }

        if let Some(table) = config_value.as_table_mut() {
            table.remove("extends");
            table.remove("extends_sha256");
        }

        Ok(config_value)
    }
}

/// Merge two TOML values. Child values take precedence.
/// Tables are merged recursively. Arrays are replaced, not appended.
fn merge_toml_values(base: toml::Value, child: toml::Value) -> toml::Value {
    match (base, child) {
        (toml::Value::Table(mut base_table), toml::Value::Table(child_table)) => {
            for (key, child_val) in child_table {
                match base_table.remove(&key) {
                    Some(base_val) => {
                        base_table.insert(key, merge_toml_values(base_val, child_val));
                    }
                    None => {
                        base_table.insert(key, child_val);
                    }
                }
            }
            toml::Value::Table(base_table)
        }
        (_, child) => child,
    }
}

impl<F: FileSystem> ConfigLoader for FileConfigLoader<F> {
    fn load(&self) -> Result<Config> {
        if let Some(local_path) = self.local_config_path()
            && self.fs.exists(&local_path)
        {
            return self.load_from_path(&local_path);
        }

        if let Some(user_path) = self.user_config_path()
            && self.fs.exists(&user_path)
        {
            return self.load_from_path(&user_path);
        }

        Ok(Config::default())
    }

    fn load_from_path(&self, path: &Path) -> Result<Config> {
        let mut visited = HashSet::new();
        let merged_value = self.load_with_extends(path, &mut visited)?;
        let config_str =
            toml::to_string(&merged_value).map_err(|e| SlocGuardError::Config(e.to_string()))?;
        Self::parse_config(&config_str)
    }

    fn load_without_extends(&self) -> Result<Config> {
        if let Some(local_path) = self.local_config_path()
            && self.fs.exists(&local_path)
        {
            return self.load_from_path_without_extends(&local_path);
        }

        if let Some(user_path) = self.user_config_path()
            && self.fs.exists(&user_path)
        {
            return self.load_from_path_without_extends(&user_path);
        }

        Ok(Config::default())
    }

    fn load_from_path_without_extends(&self, path: &Path) -> Result<Config> {
        let content = self
            .fs
            .read_to_string(path)
            .map_err(|source| SlocGuardError::FileRead {
                path: path.to_path_buf(),
                source,
            })?;
        Self::parse_config(&content)
    }
}

#[cfg(test)]
#[path = "loader_tests/mod.rs"]
mod tests;
