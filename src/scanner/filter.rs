use std::collections::HashSet;
use std::path::Path;
use std::sync::Arc;

use globset::{GlobSet, GlobSetBuilder};

use crate::error::{Result, SlocGuardError};
use crate::project::{
    ProjectPaths, UNROOTED_PROJECT_PATHS, compile_logical_path_glob, normalize_for_matching,
    normalize_pattern_for_matching,
};

/// Owned directory predicate used to prune an entire scanner subtree.
pub type DirectoryPruner = Arc<dyn Fn(&Path) -> bool + Send + Sync>;

pub trait FileFilter {
    fn should_include(&self, path: &Path) -> bool;

    /// Whether a file is excluded from the scan rather than merely filtered by type.
    fn is_scanner_excluded(&self, _path: &Path) -> bool {
        false
    }

    /// Owned predicate for directories whose entire subtree is excluded.
    ///
    /// The owned callback supports walkers that require a `'static` filter closure.
    fn directory_pruner(&self) -> DirectoryPruner {
        Arc::new(|_| false)
    }

    /// Path namespace used by this filter's configured globs.
    fn project_paths(&self) -> &ProjectPaths {
        &UNROOTED_PROJECT_PATHS
    }
}

pub struct GlobFilter {
    extensions: HashSet<String>,
    exclude_patterns: GlobSet,
    exclude_directories: DirectoryExclusions,
    project_paths: ProjectPaths,
}

/// Safely prunable directory prefixes derived from terminal `/**` patterns.
///
/// A directory can only be skipped when the configured pattern proves that every
/// descendant is excluded. Keeping this derivation in one place prevents file
/// globs such as `vendor/[!x]*` from being mistaken for whole-subtree excludes.
#[derive(Clone, Debug)]
pub(super) struct DirectoryExclusions {
    prefixes: Vec<String>,
    matcher: GlobSet,
}

impl DirectoryExclusions {
    pub(super) fn from_patterns(patterns: &[String]) -> Result<Self> {
        let prefixes = patterns
            .iter()
            .filter_map(|pattern| {
                pattern
                    .strip_suffix("/**")
                    .filter(|prefix| !prefix.is_empty())
                    .map(String::from)
            })
            .collect();
        Self::from_prefixes(prefixes)
    }

    pub(super) fn from_prefixes(prefixes: Vec<String>) -> Result<Self> {
        let mut builder = GlobSetBuilder::new();
        for prefix in &prefixes {
            let glob = compile_logical_path_glob(prefix)?;
            builder.add(glob);
        }
        let matcher = builder
            .build()
            .map_err(|e| SlocGuardError::InvalidPattern {
                pattern: "combined directory patterns".to_string(),
                source: e,
            })?;

        Ok(Self { prefixes, matcher })
    }

    pub(super) fn prefixes(&self) -> &[String] {
        &self.prefixes
    }

    pub(super) fn is_match(&self, path: &Path) -> bool {
        self.matcher.is_match(path)
    }
}

pub(super) fn normalize_scanner_exclude_patterns(patterns: &[String]) -> Vec<String> {
    patterns
        .iter()
        .map(|pattern| normalize_pattern_for_matching(pattern))
        .collect()
}

impl GlobFilter {
    /// Create a new filter with the given extensions and exclude patterns.
    ///
    /// # Errors
    /// Returns an error if any exclude pattern is invalid.
    pub fn new(extensions: Vec<String>, exclude_patterns: &[String]) -> Result<Self> {
        Self::with_project_paths(extensions, exclude_patterns, ProjectPaths::unrooted())
    }

    /// Create a filter whose path patterns are evaluated relative to the configuration root.
    ///
    /// # Errors
    /// Returns an error if any exclude pattern is invalid.
    pub fn with_project_paths(
        extensions: Vec<String>,
        exclude_patterns: &[String],
        project_paths: ProjectPaths,
    ) -> Result<Self> {
        let exclude_patterns = normalize_scanner_exclude_patterns(exclude_patterns);
        let exclude_directories = DirectoryExclusions::from_patterns(&exclude_patterns)?;
        let mut builder = GlobSetBuilder::new();
        for pattern in &exclude_patterns {
            let glob = compile_logical_path_glob(pattern)?;
            builder.add(glob);
        }
        let exclude_patterns = builder
            .build()
            .map_err(|e| SlocGuardError::InvalidPattern {
                pattern: "combined patterns".to_string(),
                source: e,
            })?;

        Ok(Self {
            extensions: extensions.into_iter().collect(),
            exclude_patterns,
            exclude_directories,
            project_paths,
        })
    }

    fn has_valid_extension(&self, path: &Path) -> bool {
        if self.extensions.is_empty() {
            return true;
        }

        path.extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|ext| self.extensions.contains(ext))
    }

    fn is_excluded(&self, path: &Path) -> bool {
        let logical = normalize_for_matching(&self.project_paths.logical(path));
        self.exclude_patterns.is_match(logical)
    }
}

impl FileFilter for GlobFilter {
    fn should_include(&self, path: &Path) -> bool {
        self.has_valid_extension(path) && !self.is_excluded(path)
    }

    fn is_scanner_excluded(&self, path: &Path) -> bool {
        self.is_excluded(path)
    }

    fn directory_pruner(&self) -> DirectoryPruner {
        let exclude_directories = self.exclude_directories.clone();
        let project_paths = self.project_paths.clone();
        Arc::new(move |path| {
            let logical = normalize_for_matching(&project_paths.logical(path));
            exclude_directories.is_match(&logical)
        })
    }

    fn project_paths(&self) -> &ProjectPaths {
        &self.project_paths
    }
}

#[cfg(test)]
#[path = "filter_tests.rs"]
mod tests;
