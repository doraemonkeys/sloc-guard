use std::collections::HashMap;
use std::path::{Path, PathBuf};

use walkdir::WalkDir;

use super::{FileFilter, FileScanner};
use super::{ScanResult, StructureScanConfig};
use crate::checker::{DirStats, StructureViolation};
use crate::error::Result;

pub struct DirectoryScanner<F: FileFilter> {
    filter: F,
    use_gitignore: bool,
}

impl<F: FileFilter> DirectoryScanner<F> {
    #[must_use]
    pub const fn new(filter: F) -> Self {
        Self {
            filter,
            use_gitignore: false,
        }
    }

    #[must_use]
    pub const fn with_gitignore(filter: F, use_gitignore: bool) -> Self {
        Self {
            filter,
            use_gitignore,
        }
    }

    fn scan_impl(&self, root: &Path) -> Vec<PathBuf> {
        if self.use_gitignore {
            self.scan_with_gitignore(root)
        } else {
            self.scan_without_gitignore(root)
        }
    }

    fn scan_without_gitignore(&self, root: &Path) -> Vec<PathBuf> {
        WalkDir::new(root)
            .into_iter()
            .filter_map(std::result::Result::ok)
            .filter(|e| e.file_type().is_file() && self.filter.should_include(e.path()))
            .map(walkdir::DirEntry::into_path)
            .collect()
    }

    fn scan_with_gitignore(&self, root: &Path) -> Vec<PathBuf> {
        use ignore::WalkBuilder;

        WalkBuilder::new(root)
            .git_ignore(true)
            .git_global(true)
            .git_exclude(true)
            .require_git(false)
            .hidden(false)
            .parents(false)
            .build()
            .filter_map(std::result::Result::ok)
            .filter(|e| e.file_type().is_some_and(|ft| ft.is_file()))
            .filter(|e| self.filter.should_include(e.path()))
            .map(ignore::DirEntry::into_path)
            .collect()
    }

    fn scan_with_structure_impl(
        &self,
        root: &Path,
        structure_config: Option<&StructureScanConfig>,
    ) -> ScanResult {
        if self.use_gitignore {
            self.scan_with_structure_gitignore(root, structure_config)
        } else {
            self.scan_with_structure_walkdir(root, structure_config)
        }
    }

    fn scan_with_structure_walkdir(
        &self,
        root: &Path,
        structure_config: Option<&StructureScanConfig>,
    ) -> ScanResult {
        let mut state = StructureScanState::new(structure_config);
        let walker = WalkDir::new(root).into_iter();

        for entry in walker {
            let Ok(entry) = entry else {
                continue;
            };

            let path = entry.path();
            let depth = entry.depth();
            let file_type = entry.file_type();

            if file_type.is_file() {
                state.process_file(path, depth, &self.filter, path);
            } else if file_type.is_dir() {
                state.process_directory(path, depth);
            }
        }

        state.finalize()
    }

    fn scan_with_structure_gitignore(
        &self,
        root: &Path,
        structure_config: Option<&StructureScanConfig>,
    ) -> ScanResult {
        use ignore::WalkBuilder;

        let mut state = StructureScanState::new(structure_config);
        let walker = WalkBuilder::new(root)
            .git_ignore(true)
            .git_global(true)
            .git_exclude(true)
            .require_git(false)
            .hidden(false)
            .parents(false)
            .build();

        for entry in walker.filter_map(std::result::Result::ok) {
            let path = entry.path();
            let depth = entry.depth();
            let file_type = entry.file_type();

            let Some(ft) = file_type else {
                continue;
            };

            if ft.is_file() {
                state.process_file(path, depth, &self.filter, path);
            } else if ft.is_dir() {
                state.process_directory(path, depth);
            }
        }

        state.finalize()
    }
}

impl<F: FileFilter + Send + Sync> FileScanner for DirectoryScanner<F> {
    fn scan(&self, root: &Path) -> Result<Vec<PathBuf>> {
        Ok(self.scan_impl(root))
    }

    fn scan_with_structure(
        &self,
        root: &Path,
        structure_config: Option<&StructureScanConfig>,
    ) -> Result<ScanResult> {
        Ok(self.scan_with_structure_impl(root, structure_config))
    }
}

/// Helper state for structure-aware scanning.
/// Extracts common logic from walkdir and ignore-based scanning.
struct StructureScanState<'a> {
    result: ScanResult,
    dir_entries: HashMap<PathBuf, DirStats>,
    structure_config: Option<&'a StructureScanConfig>,
}

impl<'a> StructureScanState<'a> {
    fn new(structure_config: Option<&'a StructureScanConfig>) -> Self {
        Self {
            result: ScanResult::default(),
            dir_entries: HashMap::new(),
            structure_config,
        }
    }

    fn process_file(
        &mut self,
        path: &Path,
        depth: usize,
        filter: &impl FileFilter,
        abs_path: &Path,
    ) {
        // Check scanner_exclude - skip entry entirely
        if let Some(cfg) = self.structure_config
            && cfg.is_scanner_excluded(path, false)
        {
            return;
        }

        // Check count_exclude - don't count but continue
        let is_count_excluded = self
            .structure_config
            .is_some_and(|cfg| cfg.is_count_excluded(path));

        // Add to files list if filter allows
        if filter.should_include(path) {
            self.result.files.push(path.to_path_buf());
        }

        // Count for parent directory (if not excluded)
        if !is_count_excluded && let Some(parent) = path.parent() {
            let parent_stats = self
                .dir_entries
                .entry(parent.to_path_buf())
                .or_insert_with(|| DirStats {
                    depth: if depth > 0 { depth - 1 } else { 0 },
                    ..Default::default()
                });
            parent_stats.file_count += 1;

            self.check_allowlist_violations(path, parent, abs_path);
        }
    }

    fn check_allowlist_violations(&mut self, path: &Path, parent: &Path, abs_path: &Path) {
        let Some(cfg) = self.structure_config else {
            return;
        };

        // 1. Check global deny patterns first (applies to all files)
        if let Some(matched) = cfg.file_matches_global_deny(abs_path) {
            self.result
                .allowlist_violations
                .push(StructureViolation::denied_file(
                    path.to_path_buf(),
                    "global".to_string(),
                    matched,
                ));
            return; // Denied files don't need further checks
        }

        // 2. Check per-rule deny and allowlist patterns
        let Some(rule) = cfg.find_matching_allowlist_rule(parent) else {
            return;
        };

        // 2a. Check per-rule deny patterns
        if let Some(matched) = rule.file_matches_deny(abs_path) {
            self.result
                .allowlist_violations
                .push(StructureViolation::denied_file(
                    path.to_path_buf(),
                    rule.scope.clone(),
                    matched,
                ));
            return; // Denied files don't need further checks
        }

        // 2b. Check allowlist (extensions/patterns) - only if configured
        if rule.has_allowlist() && !rule.file_matches(abs_path) {
            self.result
                .allowlist_violations
                .push(StructureViolation::disallowed_file(
                    path.to_path_buf(),
                    rule.scope.clone(),
                ));
        }

        // 2c. Check naming convention
        if !rule.filename_matches_naming_pattern(abs_path)
            && let Some(ref pattern_str) = rule.naming_pattern_str
        {
            self.result
                .allowlist_violations
                .push(StructureViolation::naming_convention(
                    path.to_path_buf(),
                    rule.scope.clone(),
                    pattern_str.clone(),
                ));
        }
    }

    fn process_directory(&mut self, path: &Path, depth: usize) {
        // Check scanner_exclude - skip entry entirely
        if let Some(cfg) = self.structure_config
            && cfg.is_scanner_excluded(path, true)
        {
            return;
        }

        // Check directory-only deny patterns (patterns ending with `/`)
        if let Some(cfg) = self.structure_config
            && let Some(pattern) = cfg.dir_matches_global_deny(path)
        {
            self.result
                .allowlist_violations
                .push(StructureViolation::denied_directory(
                    path.to_path_buf(),
                    "global".to_string(),
                    pattern,
                ));
        }

        // Check deny_dirs (basename-only matching from structure.deny_dirs)
        if let Some(cfg) = self.structure_config
            && let Some(pattern) = cfg.dir_matches_global_deny_basename(path)
        {
            self.result
                .allowlist_violations
                .push(StructureViolation::denied_directory(
                    path.to_path_buf(),
                    "global".to_string(),
                    pattern,
                ));
        }

        // Check per-rule deny_dirs
        if let Some(cfg) = self.structure_config
            && let Some(parent) = path.parent()
            && let Some(rule) = cfg.find_matching_allowlist_rule(parent)
            && let Some(pattern) = rule.dir_matches_deny(path)
        {
            self.result
                .allowlist_violations
                .push(StructureViolation::denied_directory(
                    path.to_path_buf(),
                    rule.scope.clone(),
                    pattern,
                ));
        }

        // Check count_exclude
        let is_count_excluded = self
            .structure_config
            .is_some_and(|cfg| cfg.is_count_excluded(path));

        // Initialize this directory's stats
        self.dir_entries
            .entry(path.to_path_buf())
            .or_insert_with(|| DirStats {
                depth,
                ..Default::default()
            });

        // Count as subdirectory for parent (if not excluded and not root)
        if depth > 0
            && !is_count_excluded
            && let Some(parent) = path.parent()
        {
            let parent_stats = self
                .dir_entries
                .entry(parent.to_path_buf())
                .or_insert_with(|| DirStats {
                    depth: depth - 1,
                    ..Default::default()
                });
            parent_stats.dir_count += 1;
        }
    }

    fn finalize(mut self) -> ScanResult {
        self.result.dir_stats = self.dir_entries;
        self.result
    }
}
