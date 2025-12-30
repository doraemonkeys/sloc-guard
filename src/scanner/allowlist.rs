use std::path::{Path, PathBuf};

use globset::{Glob, GlobSet, GlobSetBuilder};
use regex::Regex;

use crate::SlocGuardError;
use crate::error::Result;

/// Strip leading `.` component from a path for consistent scope matching.
///
/// Converts paths like `./src/foo` or `.\src\foo` (Windows) to `src/foo`,
/// ensuring scope patterns like `{src,src/**}` match regardless of scan root.
fn strip_dot_prefix(path: &Path) -> PathBuf {
    let path_str = path.to_string_lossy();

    // Strip leading "./" (Unix) or ".\" (Windows)
    if let Some(rest) = path_str
        .strip_prefix("./")
        .or_else(|| path_str.strip_prefix(".\\"))
    {
        return PathBuf::from(rest);
    }

    // Handle bare "."
    if path_str == "." {
        return PathBuf::new();
    }

    path.to_path_buf()
}

/// A compiled allowlist rule for checking allowed file types in a directory.
#[derive(Debug, Clone)]
pub struct AllowlistRule {
    /// Glob pattern defining the directory scope where this rule applies.
    pub scope: String,
    matcher: globset::GlobMatcher,
    /// Validated extensions (with leading dot, e.g., ".rs").
    pub allow_extensions: Vec<String>,
    /// Compiled patterns for allowlist matching.
    pub allow_patterns: GlobSet,
    /// Compiled file-name-only allow patterns.
    pub allow_files: GlobSet,
    /// Original file-name-only allow pattern strings (for error messages).
    pub allow_file_strs: Vec<String>,
    /// Compiled directory-name-only allow patterns.
    pub allow_dirs: GlobSet,
    /// Original directory-name-only allow pattern strings (for error messages).
    pub allow_dir_strs: Vec<String>,
    /// Deny list of file extensions (with leading dot, e.g., ".exe").
    pub deny_extensions: Vec<String>,
    /// Compiled patterns for deny matching.
    pub deny_patterns: GlobSet,
    /// Compiled file-name-only deny patterns.
    pub deny_files: GlobSet,
    /// Original file-name-only deny pattern strings (for error messages).
    pub deny_file_strs: Vec<String>,
    /// Compiled directory-name-only deny patterns.
    pub deny_dirs: GlobSet,
    /// Original directory-name-only deny pattern strings (for error messages).
    pub deny_dir_strs: Vec<String>,
    /// Compiled regex for filename validation (optional).
    naming_pattern: Option<Regex>,
    /// Original regex string for error messages.
    pub naming_pattern_str: Option<String>,
}

impl AllowlistRule {
    /// Check if file allowlist is configured (extensions, patterns, or files).
    pub(crate) fn has_allowlist(&self) -> bool {
        !self.allow_extensions.is_empty()
            || !self.allow_patterns.is_empty()
            || !self.allow_file_strs.is_empty()
    }

    /// Check if directory allowlist is configured.
    pub(crate) const fn has_dir_allowlist(&self) -> bool {
        !self.allow_dir_strs.is_empty()
    }

    /// Check if a file matches this denylist.
    /// Returns `Some(matched_pattern_or_extension)` if denied, `None` otherwise.
    pub(crate) fn file_matches_deny(&self, file_path: &Path) -> Option<String> {
        // Check deny extensions first
        if !self.deny_extensions.is_empty()
            && let Some(ext) = file_path.extension()
        {
            let ext_with_dot = format!(".{}", ext.to_string_lossy());
            if self.deny_extensions.contains(&ext_with_dot) {
                return Some(ext_with_dot);
            }
        }

        let file_name = file_path.file_name().unwrap_or_default();

        // Check deny_files (filename only)
        if let Some(idx) = self.deny_files.matches(file_name).into_iter().next() {
            return self.deny_file_strs.get(idx).cloned();
        }

        // Check deny patterns (filename and full path)
        if let Some(idx) = self.deny_patterns.matches(file_name).into_iter().next() {
            return Some(format!("pattern #{idx}"));
        }
        if let Some(idx) = self.deny_patterns.matches(file_path).into_iter().next() {
            return Some(format!("pattern #{idx}"));
        }

        None
    }

    /// Check if a directory matches this rule's `deny_dirs`.
    /// Returns `Some(matched_pattern)` if denied, `None` otherwise.
    pub(crate) fn dir_matches_deny(&self, dir_path: &Path) -> Option<String> {
        let dir_name = dir_path.file_name().unwrap_or_default();

        // Check deny_dirs (dirname only)
        if let Some(idx) = self.deny_dirs.matches(dir_name).into_iter().next() {
            return self.deny_dir_strs.get(idx).cloned();
        }

        None
    }

    /// Check if a file matches this allowlist (extensions OR patterns OR `allow_files`).
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

        let file_name = file_path.file_name().unwrap_or_default();

        // Check allow_files (filename only, OR logic)
        if self.allow_files.is_match(file_name) {
            return true;
        }

        // Check patterns (OR logic with extensions and files)
        if self.allow_patterns.is_match(file_name) || self.allow_patterns.is_match(file_path) {
            return true;
        }

        false
    }

    /// Check if a directory matches this allowlist (`allow_dirs`).
    pub(crate) fn dir_matches(&self, dir_path: &Path) -> bool {
        let dir_name = dir_path.file_name().unwrap_or_default();
        self.allow_dirs.is_match(dir_name)
    }

    /// Check if a directory path matches this rule's pattern.
    ///
    /// Normalizes paths by stripping leading `.` component (e.g., `./src` → `src`)
    /// to ensure consistent scope matching regardless of scan root.
    #[must_use]
    pub fn matches_directory(&self, dir: &Path) -> bool {
        let normalized = strip_dot_prefix(dir);
        self.matcher.is_match(normalized)
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

/// Builder for creating `AllowlistRule` instances.
pub struct AllowlistRuleBuilder {
    scope: String,
    allow_extensions: Vec<String>,
    allow_patterns: Vec<String>,
    allow_files: Vec<String>,
    allow_dirs: Vec<String>,
    deny_extensions: Vec<String>,
    deny_patterns: Vec<String>,
    deny_files: Vec<String>,
    deny_dirs: Vec<String>,
    naming_pattern: Option<String>,
}

impl AllowlistRuleBuilder {
    #[must_use]
    pub const fn new(scope: String) -> Self {
        Self {
            scope,
            allow_extensions: Vec::new(),
            allow_patterns: Vec::new(),
            allow_files: Vec::new(),
            allow_dirs: Vec::new(),
            deny_extensions: Vec::new(),
            deny_patterns: Vec::new(),
            deny_files: Vec::new(),
            deny_dirs: Vec::new(),
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
    pub fn with_allow_files(mut self, patterns: Vec<String>) -> Self {
        self.allow_files = patterns;
        self
    }

    #[must_use]
    pub fn with_allow_dirs(mut self, patterns: Vec<String>) -> Self {
        self.allow_dirs = patterns;
        self
    }

    #[must_use]
    pub fn with_deny_extensions(mut self, extensions: Vec<String>) -> Self {
        self.deny_extensions = extensions;
        self
    }

    #[must_use]
    pub fn with_deny_patterns(mut self, patterns: Vec<String>) -> Self {
        self.deny_patterns = patterns;
        self
    }

    #[must_use]
    pub fn with_deny_files(mut self, patterns: Vec<String>) -> Self {
        self.deny_files = patterns;
        self
    }

    #[must_use]
    pub fn with_deny_dirs(mut self, patterns: Vec<String>) -> Self {
        self.deny_dirs = patterns;
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
        let glob = Glob::new(&self.scope).map_err(|e| SlocGuardError::InvalidPattern {
            pattern: self.scope.clone(),
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

        // Build allow file patterns GlobSet (filename-only matching)
        let mut allow_file_builder = GlobSetBuilder::new();
        for p in &self.allow_files {
            let g = Glob::new(p).map_err(|e| SlocGuardError::InvalidPattern {
                pattern: p.clone(),
                source: e,
            })?;
            allow_file_builder.add(g);
        }
        let allow_files =
            allow_file_builder
                .build()
                .map_err(|e| SlocGuardError::InvalidPattern {
                    pattern: "allow_files".to_string(),
                    source: e,
                })?;

        // Build allow dirs GlobSet (dirname-only matching)
        let mut allow_dir_builder = GlobSetBuilder::new();
        for p in &self.allow_dirs {
            let g = Glob::new(p).map_err(|e| SlocGuardError::InvalidPattern {
                pattern: p.clone(),
                source: e,
            })?;
            allow_dir_builder.add(g);
        }
        let allow_dirs = allow_dir_builder
            .build()
            .map_err(|e| SlocGuardError::InvalidPattern {
                pattern: "allow_dirs".to_string(),
                source: e,
            })?;

        // Build deny patterns GlobSet
        let mut deny_pattern_builder = GlobSetBuilder::new();
        for p in &self.deny_patterns {
            let g = Glob::new(p).map_err(|e| SlocGuardError::InvalidPattern {
                pattern: p.clone(),
                source: e,
            })?;
            deny_pattern_builder.add(g);
        }
        let deny_patterns =
            deny_pattern_builder
                .build()
                .map_err(|e| SlocGuardError::InvalidPattern {
                    pattern: "deny_patterns".to_string(),
                    source: e,
                })?;

        // Build deny file patterns GlobSet (filename-only matching)
        let mut deny_file_builder = GlobSetBuilder::new();
        for p in &self.deny_files {
            let g = Glob::new(p).map_err(|e| SlocGuardError::InvalidPattern {
                pattern: p.clone(),
                source: e,
            })?;
            deny_file_builder.add(g);
        }
        let deny_files = deny_file_builder
            .build()
            .map_err(|e| SlocGuardError::InvalidPattern {
                pattern: "deny_files".to_string(),
                source: e,
            })?;

        // Build deny dirs GlobSet (dirname-only matching)
        let mut deny_dir_builder = GlobSetBuilder::new();
        for p in &self.deny_dirs {
            let g = Glob::new(p).map_err(|e| SlocGuardError::InvalidPattern {
                pattern: p.clone(),
                source: e,
            })?;
            deny_dir_builder.add(g);
        }
        let deny_dirs = deny_dir_builder
            .build()
            .map_err(|e| SlocGuardError::InvalidPattern {
                pattern: "deny_dirs".to_string(),
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
            scope: self.scope,
            matcher: glob.compile_matcher(),
            allow_extensions: self.allow_extensions,
            allow_patterns,
            allow_files,
            allow_file_strs: self.allow_files,
            allow_dirs,
            allow_dir_strs: self.allow_dirs,
            deny_extensions: self.deny_extensions,
            deny_patterns,
            deny_files,
            deny_file_strs: self.deny_files,
            deny_dirs,
            deny_dir_strs: self.deny_dirs,
            naming_pattern,
            naming_pattern_str,
        })
    }
}

#[cfg(test)]
mod strip_dot_prefix_tests {
    use super::*;

    #[test]
    fn removes_leading_dot() {
        assert_eq!(strip_dot_prefix(Path::new("./src")), PathBuf::from("src"));
        assert_eq!(
            strip_dot_prefix(Path::new("./src/lib")),
            PathBuf::from("src/lib")
        );
    }

    #[test]
    fn handles_backslash_paths() {
        // On Windows, paths may use backslashes
        assert_eq!(strip_dot_prefix(Path::new(".\\src")), PathBuf::from("src"));
        assert_eq!(
            strip_dot_prefix(Path::new(".\\src\\lib")),
            PathBuf::from("src\\lib")
        );
    }

    #[test]
    fn no_op_for_paths_without_dot() {
        assert_eq!(strip_dot_prefix(Path::new("src")), PathBuf::from("src"));
        assert_eq!(
            strip_dot_prefix(Path::new("src/lib")),
            PathBuf::from("src/lib")
        );
    }

    #[test]
    fn preserves_dot_in_filename() {
        // Dot in filename should not be stripped
        assert_eq!(
            strip_dot_prefix(Path::new("src/.gitignore")),
            PathBuf::from("src/.gitignore")
        );
        assert_eq!(
            strip_dot_prefix(Path::new("./.gitignore")),
            PathBuf::from(".gitignore")
        );
    }

    #[test]
    fn handles_just_dot() {
        assert_eq!(strip_dot_prefix(Path::new(".")), PathBuf::from(""));
    }

    #[test]
    fn preserves_parent_dir_prefix() {
        // "../" is ParentDir, NOT CurDir - it should not be stripped
        assert_eq!(
            strip_dot_prefix(Path::new("../src")),
            PathBuf::from("../src")
        );
        assert_eq!(
            strip_dot_prefix(Path::new("..\\src")),
            PathBuf::from("..\\src")
        );
        assert_eq!(
            strip_dot_prefix(Path::new("../../lib")),
            PathBuf::from("../../lib")
        );
    }

    #[test]
    fn handles_mixed_separators() {
        // Path with Unix prefix "./" but Windows separators internally
        assert_eq!(
            strip_dot_prefix(Path::new("./src\\lib")),
            PathBuf::from("src\\lib")
        );
        // Path with Windows prefix ".\" but Unix separators internally
        assert_eq!(
            strip_dot_prefix(Path::new(".\\src/lib")),
            PathBuf::from("src/lib")
        );
    }
}
