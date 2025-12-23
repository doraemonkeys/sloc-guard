use std::path::Path;

use globset::{Glob, GlobSet, GlobSetBuilder};

use crate::SlocGuardError;
use crate::error::Result;

use super::AllowlistRule;

/// Configuration parameters for test helper constructor.
///
/// Use with `StructureScanConfig::new()` in tests. Implements `Default` for easy
/// struct update syntax: `TestConfigParams { field: value, ..Default::default() }`.
#[cfg(test)]
#[derive(Debug, Default)]
pub struct TestConfigParams {
    pub count_exclude_patterns: Vec<String>,
    pub scanner_exclude_patterns: Vec<String>,
    pub allowlist_rules: Vec<AllowlistRule>,
    pub global_allow_extensions: Vec<String>,
    pub global_allow_files: Vec<String>,
    pub global_allow_dirs: Vec<String>,
    pub global_deny_extensions: Vec<String>,
    pub global_deny_patterns: Vec<String>,
    pub global_deny_files: Vec<String>,
    pub global_deny_dirs: Vec<String>,
}

/// Configuration for structure-aware scanning.
#[derive(Debug, Clone, Default)]
pub struct StructureScanConfig {
    /// Patterns to exclude from file/dir counting (`structure.count_exclude`).
    pub count_exclude: GlobSet,
    /// Scanner exclude patterns (scanner.exclude) - skip entirely.
    pub scanner_exclude: GlobSet,
    /// Directory names extracted from scanner.exclude patterns ending with "/**".
    pub scanner_exclude_dir_names: Vec<String>,
    /// Allowlist rules from structure.rules with `allow_extensions`/`allow_patterns`.
    pub allowlist_rules: Vec<AllowlistRule>,
    /// Global allow extensions (e.g., ".rs", ".py") - allowlist mode.
    pub global_allow_extensions: Vec<String>,
    /// Global allow patterns (compiled) for files - allowlist mode.
    pub global_allow_files: GlobSet,
    /// Original file-name-only allow pattern strings (for error messages).
    pub global_allow_file_strs: Vec<String>,
    /// Global allow directory-name-only patterns (compiled) - allowlist mode.
    pub global_allow_dirs: GlobSet,
    /// Original directory-name-only allow pattern strings (for error messages).
    pub global_allow_dir_strs: Vec<String>,
    /// Global deny extensions (e.g., ".exe", ".dll") that apply everywhere.
    pub global_deny_extensions: Vec<String>,
    /// Global deny patterns (compiled) that apply to files everywhere.
    pub global_deny_patterns: GlobSet,
    /// Original file deny pattern strings (for error messages).
    pub global_deny_pattern_strs: Vec<String>,
    /// Global deny file-name-only patterns (compiled) that apply to files everywhere.
    pub global_deny_files: GlobSet,
    /// Original file-name-only deny pattern strings (for error messages).
    pub global_deny_file_strs: Vec<String>,
    /// Global deny directory-name-only patterns (compiled via `deny_dirs` config).
    pub global_deny_dir_basenames: GlobSet,
    /// Original directory-name-only deny pattern strings (for error messages).
    pub global_deny_dir_basename_strs: Vec<String>,
    /// Directory-only deny patterns (patterns ending with `/`, e.g., "`**/node_modules/`").
    pub global_deny_dir_patterns: GlobSet,
    /// Original directory-only deny pattern strings (for error messages).
    pub global_deny_dir_pattern_strs: Vec<String>,
}

/// Builder for `StructureScanConfig`.
#[derive(Debug, Default)]
pub struct StructureScanConfigBuilder {
    count_exclude_patterns: Vec<String>,
    scanner_exclude_patterns: Vec<String>,
    allowlist_rules: Vec<AllowlistRule>,
    global_allow_extensions: Vec<String>,
    global_allow_files: Vec<String>,
    global_allow_dirs: Vec<String>,
    global_deny_extensions: Vec<String>,
    global_deny_patterns: Vec<String>,
    global_deny_files: Vec<String>,
    global_deny_dirs: Vec<String>,
}

impl StructureScanConfigBuilder {
    /// Create a new builder with default values.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set patterns to exclude from file/dir counting.
    #[must_use]
    pub fn count_exclude(mut self, patterns: Vec<String>) -> Self {
        self.count_exclude_patterns = patterns;
        self
    }

    /// Set scanner exclude patterns (skip entirely).
    #[must_use]
    pub fn scanner_exclude(mut self, patterns: Vec<String>) -> Self {
        self.scanner_exclude_patterns = patterns;
        self
    }

    /// Set allowlist rules from structure.rules.
    #[must_use]
    pub fn allowlist_rules(mut self, rules: Vec<AllowlistRule>) -> Self {
        self.allowlist_rules = rules;
        self
    }

    /// Set global allow extensions (e.g., ".rs", ".py").
    #[must_use]
    pub fn global_allow_extensions(mut self, extensions: Vec<String>) -> Self {
        self.global_allow_extensions = extensions;
        self
    }

    /// Set global allow file patterns (filename-only matching).
    #[must_use]
    pub fn global_allow_files(mut self, patterns: Vec<String>) -> Self {
        self.global_allow_files = patterns;
        self
    }

    /// Set global allow directory patterns (dirname-only matching).
    #[must_use]
    pub fn global_allow_dirs(mut self, patterns: Vec<String>) -> Self {
        self.global_allow_dirs = patterns;
        self
    }

    /// Set global deny extensions (e.g., ".exe", ".dll").
    #[must_use]
    pub fn global_deny_extensions(mut self, extensions: Vec<String>) -> Self {
        self.global_deny_extensions = extensions;
        self
    }

    /// Set global deny patterns for files.
    #[must_use]
    pub fn global_deny_patterns(mut self, patterns: Vec<String>) -> Self {
        self.global_deny_patterns = patterns;
        self
    }

    /// Set global deny file patterns (filename-only matching).
    #[must_use]
    pub fn global_deny_files(mut self, patterns: Vec<String>) -> Self {
        self.global_deny_files = patterns;
        self
    }

    /// Set global deny directory patterns (dirname-only matching).
    #[must_use]
    pub fn global_deny_dirs(mut self, patterns: Vec<String>) -> Self {
        self.global_deny_dirs = patterns;
        self
    }

    /// Build the `StructureScanConfig`.
    ///
    /// # Errors
    /// Returns an error if any pattern is invalid.
    pub fn build(self) -> Result<StructureScanConfig> {
        StructureScanConfig::from_builder(self)
    }
}

impl StructureScanConfig {
    /// Create a new builder.
    #[must_use]
    pub fn builder() -> StructureScanConfigBuilder {
        StructureScanConfigBuilder::new()
    }

    /// Convenience constructor for tests using `TestConfigParams` struct.
    ///
    /// Production code should use the builder pattern via `StructureScanConfig::builder()`.
    ///
    /// # Errors
    /// Returns an error if any pattern is invalid.
    #[cfg(test)]
    pub fn new(params: TestConfigParams) -> Result<Self> {
        Self::builder()
            .count_exclude(params.count_exclude_patterns)
            .scanner_exclude(params.scanner_exclude_patterns)
            .allowlist_rules(params.allowlist_rules)
            .global_allow_extensions(params.global_allow_extensions)
            .global_allow_files(params.global_allow_files)
            .global_allow_dirs(params.global_allow_dirs)
            .global_deny_extensions(params.global_deny_extensions)
            .global_deny_patterns(params.global_deny_patterns)
            .global_deny_files(params.global_deny_files)
            .global_deny_dirs(params.global_deny_dirs)
            .build()
    }

    /// Build from a `StructureScanConfigBuilder`.
    fn from_builder(builder: StructureScanConfigBuilder) -> Result<Self> {
        let count_exclude = Self::build_glob_set(&builder.count_exclude_patterns)?;
        let scanner_exclude = Self::build_glob_set(&builder.scanner_exclude_patterns)?;
        let scanner_exclude_dir_names = Self::extract_dir_names(&builder.scanner_exclude_patterns);

        // Build global allow file patterns (filename-only matching)
        let global_allow_file_strs = builder.global_allow_files;
        let global_allow_files_compiled = Self::build_glob_set(&global_allow_file_strs)?;

        // Build global allow dir patterns (dirname-only matching)
        let global_allow_dir_strs = builder.global_allow_dirs;
        let global_allow_dirs_compiled = Self::build_glob_set(&global_allow_dir_strs)?;

        // Separate directory-only patterns (ending with `/`) from file patterns
        let (dir_patterns, file_patterns): (Vec<_>, Vec<_>) = builder
            .global_deny_patterns
            .iter()
            .partition(|p| p.ends_with('/'));

        // For directory patterns, strip the trailing `/` for glob matching
        let dir_pattern_strs: Vec<String> = dir_patterns.iter().map(|s| (*s).clone()).collect();
        let dir_patterns_for_glob: Vec<String> = dir_patterns
            .iter()
            .map(|p| p.trim_end_matches('/').to_string())
            .collect();

        // Convert file_patterns from Vec<&String> to Vec<String> for build_glob_set
        let file_pattern_strs: Vec<String> = file_patterns.iter().map(|s| (*s).clone()).collect();

        let global_deny_patterns_compiled = Self::build_glob_set(&file_pattern_strs)?;
        let global_deny_dir_patterns = Self::build_glob_set(&dir_patterns_for_glob)?;

        // Build global deny file patterns (filename-only matching)
        let global_deny_file_strs = builder.global_deny_files;
        let global_deny_files_compiled = Self::build_glob_set(&global_deny_file_strs)?;

        // Build global deny dir patterns (dirname-only matching, from deny_dirs config)
        let global_deny_dir_basename_strs = builder.global_deny_dirs;
        let global_deny_dir_basenames = Self::build_glob_set(&global_deny_dir_basename_strs)?;

        Ok(Self {
            count_exclude,
            scanner_exclude,
            scanner_exclude_dir_names,
            allowlist_rules: builder.allowlist_rules,
            global_allow_extensions: builder.global_allow_extensions,
            global_allow_files: global_allow_files_compiled,
            global_allow_file_strs,
            global_allow_dirs: global_allow_dirs_compiled,
            global_allow_dir_strs,
            global_deny_extensions: builder.global_deny_extensions,
            global_deny_patterns: global_deny_patterns_compiled,
            global_deny_pattern_strs: file_pattern_strs,
            global_deny_files: global_deny_files_compiled,
            global_deny_file_strs,
            global_deny_dir_basenames,
            global_deny_dir_basename_strs,
            global_deny_dir_patterns,
            global_deny_dir_pattern_strs: dir_pattern_strs,
        })
    }

    fn build_glob_set(patterns: &[String]) -> Result<GlobSet> {
        let mut builder = GlobSetBuilder::new();
        for pattern in patterns {
            let glob = Glob::new(pattern).map_err(|e| SlocGuardError::InvalidPattern {
                pattern: pattern.clone(),
                source: e,
            })?;
            builder.add(glob);
        }
        builder.build().map_err(|e| SlocGuardError::InvalidPattern {
            pattern: "combined patterns".to_string(),
            source: e,
        })
    }

    /// Extract directory names from patterns ending with "/**".
    fn extract_dir_names(patterns: &[String]) -> Vec<String> {
        patterns
            .iter()
            .filter_map(|p| {
                let trimmed = p.trim_end_matches("/**").trim_end_matches("\\**");
                if trimmed.len() < p.len() {
                    let last_component = trimmed
                        .rsplit(['/', '\\'])
                        .next()
                        .filter(|s| !s.is_empty() && !s.contains('*'));
                    last_component.map(String::from)
                } else {
                    None
                }
            })
            .collect()
    }

    /// Check if a path should be excluded from scanning entirely.
    pub(crate) fn is_scanner_excluded(&self, path: &Path, is_dir: bool) -> bool {
        let file_name = path.file_name().unwrap_or_default();
        let file_name_str = file_name.to_string_lossy();

        if self.scanner_exclude.is_match(file_name) || self.scanner_exclude.is_match(path) {
            return true;
        }

        // For directories, check against extracted dir names
        if is_dir {
            for dir_name in &self.scanner_exclude_dir_names {
                if file_name_str == *dir_name {
                    return true;
                }
            }
        }

        false
    }

    /// Check if a path should be excluded from counting (but still traversed).
    pub(crate) fn is_count_excluded(&self, path: &Path) -> bool {
        let file_name = path.file_name().unwrap_or_default();
        self.count_exclude.is_match(file_name) || self.count_exclude.is_match(path)
    }

    /// Find the first allowlist rule matching a directory.
    #[must_use]
    pub fn find_matching_allowlist_rule(&self, dir: &Path) -> Option<&AllowlistRule> {
        self.allowlist_rules
            .iter()
            .find(|r| r.matches_directory(dir))
    }

    /// Check if global file allowlist mode is enabled.
    #[allow(clippy::missing_const_for_fn)] // HashSet::is_empty() is not const
    pub(crate) fn has_global_file_allowlist(&self) -> bool {
        !self.global_allow_extensions.is_empty() || !self.global_allow_file_strs.is_empty()
    }

    /// Check if global directory allowlist mode is enabled.
    pub(crate) const fn has_global_dir_allowlist(&self) -> bool {
        !self.global_allow_dir_strs.is_empty()
    }

    /// Check if a file matches global allowlist.
    /// Returns `true` if file is allowed, `false` if not allowed.
    pub(crate) fn file_matches_global_allow(&self, file_path: &Path) -> bool {
        // Check global allow extensions
        if !self.global_allow_extensions.is_empty()
            && let Some(ext) = file_path.extension()
        {
            let ext_with_dot = format!(".{}", ext.to_string_lossy());
            if self.global_allow_extensions.contains(&ext_with_dot) {
                return true;
            }
        }

        let file_name = file_path.file_name().unwrap_or_default();

        // Check global allow files (filename-only matching)
        if self.global_allow_files.is_match(file_name) {
            return true;
        }

        false
    }

    /// Check if a directory matches global allowlist.
    /// Returns `true` if directory is allowed, `false` if not allowed.
    pub(crate) fn dir_matches_global_allow(&self, dir_path: &Path) -> bool {
        let dir_name = dir_path.file_name().unwrap_or_default();
        self.global_allow_dirs.is_match(dir_name)
    }

    /// Check if a file matches global deny patterns.
    /// Returns `Some(matched_pattern_or_extension)` if denied, `None` otherwise.
    pub(crate) fn file_matches_global_deny(&self, file_path: &Path) -> Option<String> {
        // Check global deny extensions first
        if !self.global_deny_extensions.is_empty()
            && let Some(ext) = file_path.extension()
        {
            let ext_with_dot = format!(".{}", ext.to_string_lossy());
            if self.global_deny_extensions.contains(&ext_with_dot) {
                return Some(ext_with_dot);
            }
        }

        let file_name = file_path.file_name().unwrap_or_default();

        // Check global deny files (filename-only matching)
        if let Some(idx) = self.global_deny_files.matches(file_name).into_iter().next() {
            return self.global_deny_file_strs.get(idx).cloned();
        }

        // Check global deny patterns (filename and full path)
        if let Some(idx) = self
            .global_deny_patterns
            .matches(file_name)
            .into_iter()
            .next()
        {
            return self.global_deny_pattern_strs.get(idx).cloned();
        }

        if let Some(idx) = self
            .global_deny_patterns
            .matches(file_path)
            .into_iter()
            .next()
        {
            return self.global_deny_pattern_strs.get(idx).cloned();
        }

        None
    }

    /// Check if a directory matches global directory-only deny patterns (patterns ending with `/`).
    /// Returns `Some(original_pattern)` if denied, `None` otherwise.
    pub(crate) fn dir_matches_global_deny(&self, dir_path: &Path) -> Option<String> {
        let dir_name = dir_path.file_name().unwrap_or_default();

        // Check against compiled patterns (without trailing `/`)
        if let Some(idx) = self
            .global_deny_dir_patterns
            .matches(dir_name)
            .into_iter()
            .next()
        {
            return self.global_deny_dir_pattern_strs.get(idx).cloned();
        }

        if let Some(idx) = self
            .global_deny_dir_patterns
            .matches(dir_path)
            .into_iter()
            .next()
        {
            return self.global_deny_dir_pattern_strs.get(idx).cloned();
        }

        None
    }

    /// Check if a directory matches global `deny_dirs` patterns (basename-only matching).
    /// Returns `Some(matched_pattern)` if denied, `None` otherwise.
    pub(crate) fn dir_matches_global_deny_basename(&self, dir_path: &Path) -> Option<String> {
        let dir_name = dir_path.file_name().unwrap_or_default();

        if let Some(idx) = self
            .global_deny_dir_basenames
            .matches(dir_name)
            .into_iter()
            .next()
        {
            return self.global_deny_dir_basename_strs.get(idx).cloned();
        }

        None
    }
}
