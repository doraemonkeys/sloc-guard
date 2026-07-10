use std::collections::HashMap;
use std::path::{Path, PathBuf};

use walkdir::WalkDir;

use super::{DirectoryPruner, FileFilter, FileScanner};
use super::{ScanResult, StructureScanConfig};
use crate::checker::{DirStats, StructureViolation};
use crate::error::Result;
use crate::project::{ProjectPaths, normalize_for_matching};

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
    ) -> Result<ScanResult> {
        if self.use_gitignore {
            self.scan_with_structure_gitignore(root, structure_config)
        } else {
            self.scan_with_structure_walkdir(root, structure_config)
        }
    }

    fn structure_path_context(&self, root: &Path) -> (ProjectPaths, ProjectPaths, bool) {
        let configured = self.filter.project_paths();
        if configured.is_rooted() {
            (configured.clone(), configured.clone(), false)
        } else {
            // Library callers that do not provide a project context historically match paths in
            // the namespace passed to `scan_with_structure`. Preserve that namespace for structure
            // rules while adding a scan-root-relative namespace specifically for scanner excludes.
            (
                configured.clone(),
                ProjectPaths::rooted(root.to_path_buf()),
                true,
            )
        }
    }

    fn combined_directory_pruner(
        &self,
        structure_config: Option<&StructureScanConfig>,
        project_paths: &ProjectPaths,
        match_identity_path: bool,
    ) -> Result<DirectoryPruner> {
        let filter_pruner = self.filter.directory_pruner();
        let config_pruner = structure_config
            .map(|config| {
                config.scanner_exclude_directory_pruner(project_paths.clone(), match_identity_path)
            })
            .transpose()?;

        Ok(std::sync::Arc::new(move |path| {
            filter_pruner(path)
                || config_pruner
                    .as_ref()
                    .is_some_and(|config_pruner| config_pruner(path))
        }))
    }

    fn scan_with_structure_walkdir(
        &self,
        root: &Path,
        structure_config: Option<&StructureScanConfig>,
    ) -> Result<ScanResult> {
        let (project_paths, scanner_project_paths, match_identity_path) =
            self.structure_path_context(root);
        let directory_pruner = self.combined_directory_pruner(
            structure_config,
            &scanner_project_paths,
            match_identity_path,
        )?;
        let mut state = StructureScanState::new(
            structure_config,
            project_paths,
            scanner_project_paths,
            match_identity_path,
        );
        // Use filter_entry to skip excluded directories entirely (prunes subtree)
        let walker = WalkDir::new(root).into_iter().filter_entry(move |entry| {
            !entry.file_type().is_dir() || !directory_pruner(entry.path())
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

        Ok(state.finalize())
    }

    fn scan_with_structure_gitignore(
        &self,
        root: &Path,
        structure_config: Option<&StructureScanConfig>,
    ) -> Result<ScanResult> {
        use ignore::WalkBuilder;

        let (project_paths, scanner_project_paths, match_identity_path) =
            self.structure_path_context(root);
        let directory_pruner = self.combined_directory_pruner(
            structure_config,
            &scanner_project_paths,
            match_identity_path,
        )?;
        let mut state = StructureScanState::new(
            structure_config,
            project_paths,
            scanner_project_paths,
            match_identity_path,
        );
        let walker = WalkBuilder::new(root)
            .git_ignore(true)
            .git_global(true)
            .git_exclude(true)
            .require_git(false)
            .hidden(false)
            .parents(true)
            .filter_entry(move |entry| {
                !entry
                    .file_type()
                    .is_some_and(|file_type| file_type.is_dir())
                    || !directory_pruner(entry.path())
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

        Ok(state.finalize())
    }
}

impl<F: FileFilter + Send + Sync> FileScanner for DirectoryScanner<F> {
    fn project_paths(&self) -> &ProjectPaths {
        self.filter.project_paths()
    }

    fn scan(&self, root: &Path) -> Result<Vec<PathBuf>> {
        Ok(self.scan_impl(root))
    }

    fn scan_with_structure(
        &self,
        root: &Path,
        structure_config: Option<&StructureScanConfig>,
    ) -> Result<ScanResult> {
        self.scan_with_structure_impl(root, structure_config)
    }
}

/// Helper state for structure-aware scanning.
/// Extracts common logic from walkdir and ignore-based scanning.
struct StructureScanState<'a> {
    result: ScanResult,
    dir_entries: HashMap<PathBuf, DirStats>,
    structure_config: Option<&'a StructureScanConfig>,
    project_paths: ProjectPaths,
    scanner_project_paths: ProjectPaths,
    match_identity_path: bool,
}

impl<'a> StructureScanState<'a> {
    fn new(
        structure_config: Option<&'a StructureScanConfig>,
        project_paths: ProjectPaths,
        scanner_project_paths: ProjectPaths,
        match_identity_path: bool,
    ) -> Self {
        Self {
            result: ScanResult::default(),
            dir_entries: HashMap::new(),
            structure_config,
            project_paths,
            scanner_project_paths,
            match_identity_path,
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
        if filter.is_scanner_excluded(path)
            || self.structure_config.is_some_and(|cfg| {
                cfg.is_scanner_excluded_with_namespaces(
                    path,
                    false,
                    &self.scanner_project_paths,
                    self.match_identity_path,
                )
            })
        {
            return;
        }

        // Check count_exclude - don't count but continue
        let is_count_excluded = self
            .structure_config
            .is_some_and(|cfg| cfg.is_count_excluded_with_project_paths(path, &self.project_paths));

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
        let logical_file = self.project_paths.logical(abs_path);
        let logical_parent = self.project_paths.logical(parent);

        // Find matching per-rule first (needed for override checks)
        let matching_rule = cfg.find_matching_allowlist_rule_logical(&logical_parent);

        // 1. Check global level patterns
        if cfg.has_global_file_allowlist() {
            // Allow mode: file must match global allowlist
            if !cfg.file_matches_global_allow(&logical_file) {
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
                .is_some_and(|rule| rule.has_allowlist() && rule.file_matches(&logical_file));

            if !overridden_by_rule
                && let Some(matched) = cfg.file_matches_global_deny(&logical_file)
            {
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
        if let Some(matched) = rule.file_matches_deny(&logical_file) {
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
            if !rule.file_matches(&logical_file) {
                self.result
                    .allowlist_violations
                    .push(StructureViolation::disallowed_file(
                        path.to_path_buf(),
                        rule.scope.clone(),
                    ));
                return; // Disallowed files don't need further checks
            }
        }

        // Check naming convention (only for allowed files)
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
        // Rule scopes and directory allow/deny patterns are configuration-root-relative.
        // Derive the parent after rebasing so a scan rooted at `.` from a nested cwd does not
        // accidentally treat the scan root as its own child.
        let logical_path = self.project_paths.logical(path);
        let is_configuration_root =
            normalize_for_matching(&self.scanner_project_paths.logical(path))
                .as_os_str()
                .is_empty();
        let matching_rule = if is_configuration_root {
            None
        } else {
            self.structure_config.and_then(|cfg| {
                logical_path
                    .parent()
                    .and_then(|parent| cfg.find_matching_allowlist_rule_logical(parent))
            })
        };

        // Check global level directory patterns
        if !is_configuration_root && let Some(cfg) = self.structure_config {
            if cfg.has_global_dir_allowlist() {
                // Allow mode: directory must match global allowlist
                if !cfg.dir_matches_global_allow(&logical_path) {
                    self.result.allowlist_violations.push(
                        StructureViolation::disallowed_directory(
                            path.to_path_buf(),
                            "global".to_string(),
                        ),
                    );
                }
            } else {
                // Check if a per-rule allow would override global deny
                let overridden_by_rule = matching_rule.is_some_and(|rule| {
                    rule.has_dir_allowlist() && rule.dir_matches(&logical_path)
                });

                if !overridden_by_rule {
                    // Deny mode: check directory-only deny patterns (patterns ending with `/`)
                    if let Some(pattern) = cfg.dir_matches_global_deny(&logical_path) {
                        self.result.allowlist_violations.push(
                            StructureViolation::denied_directory(
                                path.to_path_buf(),
                                "global".to_string(),
                                pattern,
                            ),
                        );
                    }

                    // Check deny_dirs (basename-only matching from structure.deny_dirs)
                    if let Some(pattern) = cfg.dir_matches_global_deny_basename(&logical_path) {
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
                if !rule.dir_matches(&logical_path) {
                    self.result.allowlist_violations.push(
                        StructureViolation::disallowed_directory(
                            path.to_path_buf(),
                            rule.scope.clone(),
                        ),
                    );
                }
            } else if let Some(pattern) = rule.dir_matches_deny(&logical_path) {
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
            .is_some_and(|cfg| cfg.is_count_excluded_with_project_paths(path, &self.project_paths));

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
