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
            .parents(true)
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
        // Use filter_entry to skip excluded directories entirely (prunes subtree)
        let walker = WalkDir::new(root).into_iter().filter_entry(|e| {
            if e.file_type().is_dir()
                && let Some(cfg) = structure_config
            {
                // Return false to skip this directory and all its children
                return !cfg.is_scanner_excluded(e.path(), true);
            }
            true
        });

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
        // Clone config: filter_entry closure must be 'static, but structure_config is a borrowed reference
        let config_for_filter = structure_config.cloned();
        let walker = WalkBuilder::new(root)
            .git_ignore(true)
            .git_global(true)
            .git_exclude(true)
            .require_git(false)
            .hidden(false)
            .parents(true)
            .filter_entry(move |e| {
                // Skip excluded directories entirely (prunes subtree)
                if e.file_type().is_some_and(|ft| ft.is_dir())
                    && let Some(ref cfg) = config_for_filter
                {
                    return !cfg.is_scanner_excluded(e.path(), true);
                }
                true
            })
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

        // Find matching per-rule first (needed for override checks)
        let matching_rule = cfg.find_matching_allowlist_rule(parent);

        // 1. Check global level patterns
        if cfg.has_global_file_allowlist() {
            // Allow mode: file must match global allowlist
            if !cfg.file_matches_global_allow(abs_path) {
                self.result
                    .allowlist_violations
                    .push(StructureViolation::disallowed_file(
                        path.to_path_buf(),
                        "global".to_string(),
                    ));
                return;
            }
        } else {
            // Deny mode: check global deny patterns
            // But first check if a per-rule allow would override global deny
            let overridden_by_rule = matching_rule
                .is_some_and(|rule| rule.has_allowlist() && rule.file_matches(abs_path));

            if !overridden_by_rule && let Some(matched) = cfg.file_matches_global_deny(abs_path) {
                self.result
                    .allowlist_violations
                    .push(StructureViolation::denied_file(
                        path.to_path_buf(),
                        "global".to_string(),
                        matched,
                    ));
                return; // Denied files don't need further checks
            }
        }

        // 2. Check per-rule patterns
        let Some(rule) = matching_rule else {
            return;
        };

        // Check per-rule deny patterns first (they take precedence over per-rule allow)
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

        // Then check if rule is in allow mode
        if rule.has_allowlist() {
            // Allow mode: file must match allowlist
            if !rule.file_matches(abs_path) {
                self.result
                    .allowlist_violations
                    .push(StructureViolation::disallowed_file(
                        path.to_path_buf(),
                        rule.scope.clone(),
                    ));
            }
        }

        // Check naming convention (applies regardless of mode)
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

        // Find matching per-rule for parent directory (needed for override checks)
        let matching_rule = self.structure_config.and_then(|cfg| {
            path.parent()
                .and_then(|p| cfg.find_matching_allowlist_rule(p))
        });

        // Check global level directory patterns
        if let Some(cfg) = self.structure_config {
            if cfg.has_global_dir_allowlist() {
                // Allow mode: directory must match global allowlist
                if !cfg.dir_matches_global_allow(path) {
                    self.result.allowlist_violations.push(
                        StructureViolation::disallowed_directory(
                            path.to_path_buf(),
                            "global".to_string(),
                        ),
                    );
                }
            } else {
                // Check if a per-rule allow would override global deny
                let overridden_by_rule = matching_rule
                    .is_some_and(|rule| rule.has_dir_allowlist() && rule.dir_matches(path));

                if !overridden_by_rule {
                    // Deny mode: check directory-only deny patterns (patterns ending with `/`)
                    if let Some(pattern) = cfg.dir_matches_global_deny(path) {
                        self.result.allowlist_violations.push(
                            StructureViolation::denied_directory(
                                path.to_path_buf(),
                                "global".to_string(),
                                pattern,
                            ),
                        );
                    }

                    // Check deny_dirs (basename-only matching from structure.deny_dirs)
                    if let Some(pattern) = cfg.dir_matches_global_deny_basename(path) {
                        self.result.allowlist_violations.push(
                            StructureViolation::denied_directory(
                                path.to_path_buf(),
                                "global".to_string(),
                                pattern,
                            ),
                        );
                    }
                }
            }
        }

        // Check per-rule directory patterns
        if let Some(rule) = matching_rule {
            if rule.has_dir_allowlist() {
                // Allow mode: directory must match allowlist
                if !rule.dir_matches(path) {
                    self.result.allowlist_violations.push(
                        StructureViolation::disallowed_directory(
                            path.to_path_buf(),
                            rule.scope.clone(),
                        ),
                    );
                }
            } else if let Some(pattern) = rule.dir_matches_deny(path) {
                // Deny mode: check per-rule deny_dirs
                self.result
                    .allowlist_violations
                    .push(StructureViolation::denied_directory(
                        path.to_path_buf(),
                        rule.scope.clone(),
                        pattern,
                    ));
            }
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
