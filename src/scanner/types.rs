use std::collections::HashMap;
use std::path::{Path, PathBuf};

use globset::{Glob, GlobSet, GlobSetBuilder};
use regex::Regex;

use crate::SlocGuardError;
use crate::checker::{DirStats, StructureViolation};
use crate::error::Result;

/// Result of unified directory scan with structure stats.
#[derive(Debug, Clone, Default)]
pub struct ScanResult {
    /// All file paths discovered during scanning.
    pub files: Vec<PathBuf>,
    /// Directory statistics: immediate children counts and depth.
    pub dir_stats: HashMap<PathBuf, DirStats>,
    /// Allowlist violations detected during scanning.
    pub allowlist_violations: Vec<StructureViolation>,
}

/// A compiled allowlist rule for checking allowed file types in a directory.
#[derive(Debug, Clone)]
pub struct AllowlistRule {
    /// Glob pattern matching directories where this rule applies.
    pub pattern: String,
    matcher: globset::GlobMatcher,
    /// Validated extensions (with leading dot, e.g., ".rs").
    pub allow_extensions: Vec<String>,
    /// Compiled patterns for allowlist matching.
    pub allow_patterns: GlobSet,
    /// Compiled regex for filename validation (optional).
    naming_pattern: Option<Regex>,
    /// Original regex string for error messages.
    pub naming_pattern_str: Option<String>,
}

impl AllowlistRule {
    /// Check if allowlist is configured (extensions or patterns).
    pub(crate) fn has_allowlist(&self) -> bool {
        !self.allow_extensions.is_empty() || !self.allow_patterns.is_empty()
    }

    /// Check if a file matches this allowlist (extensions OR patterns).
    pub(crate) fn file_matches(&self, file_path: &Path) -> bool {
        // Check extensions first (OR logic)
        if !self.allow_extensions.is_empty()
            && let Some(ext) = file_path.extension()
        {
            let ext_with_dot = format!(".{}", ext.to_string_lossy());
            if self.allow_extensions.contains(&ext_with_dot) {
                return true;
            }
        }

        // Check patterns (OR logic with extensions)
        let file_name = file_path.file_name().unwrap_or_default();
        if self.allow_patterns.is_match(file_name) || self.allow_patterns.is_match(file_path) {
            return true;
        }

        false
    }

    /// Check if a directory path matches this rule's pattern.
    #[must_use]
    pub fn matches_directory(&self, dir: &Path) -> bool {
        self.matcher.is_match(dir)
    }

    /// Check if a filename matches the naming convention pattern.
    /// Returns `true` if no pattern is set or if the filename matches.
    #[must_use]
    pub fn filename_matches_naming_pattern(&self, file_path: &Path) -> bool {
        let Some(ref regex) = self.naming_pattern else {
            return true; // No naming pattern = always valid
        };

        let file_name = file_path
            .file_name()
            .map(|n| n.to_string_lossy())
            .unwrap_or_default();

        regex.is_match(&file_name)
    }
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
}

impl StructureScanConfig {
    /// Build from config components.
    ///
    /// # Errors
    /// Returns an error if any pattern is invalid.
    pub fn new(
        count_exclude_patterns: &[String],
        scanner_exclude_patterns: &[String],
        allowlist_rules: Vec<AllowlistRule>,
    ) -> Result<Self> {
        let count_exclude = Self::build_glob_set(count_exclude_patterns)?;
        let scanner_exclude = Self::build_glob_set(scanner_exclude_patterns)?;
        let scanner_exclude_dir_names = Self::extract_dir_names(scanner_exclude_patterns);

        Ok(Self {
            count_exclude,
            scanner_exclude,
            scanner_exclude_dir_names,
            allowlist_rules,
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
}

/// Builder for creating `AllowlistRule` instances.
pub struct AllowlistRuleBuilder {
    pattern: String,
    allow_extensions: Vec<String>,
    allow_patterns: Vec<String>,
    naming_pattern: Option<String>,
}

impl AllowlistRuleBuilder {
    #[must_use]
    pub const fn new(pattern: String) -> Self {
        Self {
            pattern,
            allow_extensions: Vec::new(),
            allow_patterns: Vec::new(),
            naming_pattern: None,
        }
    }

    #[must_use]
    pub fn with_extensions(mut self, extensions: Vec<String>) -> Self {
        self.allow_extensions = extensions;
        self
    }

    #[must_use]
    pub fn with_patterns(mut self, patterns: Vec<String>) -> Self {
        self.allow_patterns = patterns;
        self
    }

    #[must_use]
    pub fn with_naming_pattern(mut self, pattern: Option<String>) -> Self {
        self.naming_pattern = pattern;
        self
    }

    /// Build the `AllowlistRule`.
    ///
    /// # Errors
    /// Returns an error if any pattern is invalid.
    pub fn build(self) -> Result<AllowlistRule> {
        let glob = Glob::new(&self.pattern).map_err(|e| SlocGuardError::InvalidPattern {
            pattern: self.pattern.clone(),
            source: e,
        })?;

        let mut pattern_builder = GlobSetBuilder::new();
        for p in &self.allow_patterns {
            let g = Glob::new(p).map_err(|e| SlocGuardError::InvalidPattern {
                pattern: p.clone(),
                source: e,
            })?;
            pattern_builder.add(g);
        }
        let allow_patterns =
            pattern_builder
                .build()
                .map_err(|e| SlocGuardError::InvalidPattern {
                    pattern: "allow_patterns".to_string(),
                    source: e,
                })?;

        // Compile naming pattern regex if provided
        let (naming_pattern, naming_pattern_str) = match self.naming_pattern {
            Some(pattern_str) => {
                let regex = Regex::new(&pattern_str).map_err(|e| {
                    SlocGuardError::Config(format!(
                        "Invalid naming pattern regex '{pattern_str}': {e}"
                    ))
                })?;
                (Some(regex), Some(pattern_str))
            }
            None => (None, None),
        };

        Ok(AllowlistRule {
            pattern: self.pattern,
            matcher: glob.compile_matcher(),
            allow_extensions: self.allow_extensions,
            allow_patterns,
            naming_pattern,
            naming_pattern_str,
        })
    }
}
