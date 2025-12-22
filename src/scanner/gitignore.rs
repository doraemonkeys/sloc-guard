use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;

use gix::bstr::{BStr, ByteSlice};
use gix::dir::walk::EmissionMode;

use super::{FileFilter, FileScanner};
use super::{ScanResult, StructureScanConfig};
use crate::checker::{DirStats, StructureViolation};
use crate::error::{Result, SlocGuardError};

/// A file scanner that respects .gitignore patterns using gix.
///
/// This scanner discovers the git repository containing the scan root and uses
/// gix's dirwalk to iterate over files while automatically excluding paths
/// matched by .gitignore patterns.
///
/// If the scan path is not within a git repository, it returns an error.
pub struct GitAwareScanner<F: FileFilter> {
    filter: F,
}

impl<F: FileFilter> GitAwareScanner<F> {
    #[must_use]
    pub const fn new(filter: F) -> Self {
        Self { filter }
    }

    fn scan_with_gix(&self, root: &Path) -> Result<Vec<PathBuf>> {
        let repo = gix::discover(root).map_err(|e| {
            SlocGuardError::GitRepoNotFound(format!("Failed to discover git repository: {e}"))
        })?;

        let workdir = repo
            .workdir()
            .ok_or_else(|| SlocGuardError::Git("Repository has no working directory".into()))?;

        // Compute relative path from workdir to root
        let root_abs = root.canonicalize().map_err(|e| {
            SlocGuardError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Cannot canonicalize path {}: {e}", root.display()),
            ))
        })?;
        let workdir_abs = workdir.canonicalize().map_err(|e| {
            SlocGuardError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Cannot canonicalize workdir {}: {e}", workdir.display()),
            ))
        })?;

        // Calculate the prefix for filtering entries within the scan root
        let prefix = if root_abs == workdir_abs {
            PathBuf::new()
        } else {
            root_abs
                .strip_prefix(&workdir_abs)
                .map_err(|_| {
                    SlocGuardError::Git(format!(
                        "Scan path {} is not within git workdir {}",
                        root_abs.display(),
                        workdir_abs.display()
                    ))
                })?
                .to_path_buf()
        };

        // Get index for tracked files
        let index = repo
            .index_or_empty()
            .map_err(|e| SlocGuardError::Git(format!("Failed to get git index: {e}")))?;

        // Walk the directory with gitignore support
        let should_interrupt = AtomicBool::new(false);
        let options = repo
            .dirwalk_options()
            .map_err(|e| SlocGuardError::Git(format!("Failed to create dirwalk options: {e}")))?
            .emit_tracked(true)
            .emit_untracked(EmissionMode::Matching);

        let mut delegate = Collector::new(&prefix, &self.filter);

        // Convert prefix to BStr for pattern matching
        // Normalize path separators to forward slashes for git pathspecs
        let prefix_string = prefix.to_string_lossy().replace('\\', "/");
        let prefix_str = gix::bstr::BString::from(prefix_string.as_bytes());
        let patterns: &[&BStr] = if prefix_str.is_empty() {
            &[]
        } else {
            &[prefix_str.as_bstr()]
        };

        repo.dirwalk(&index, patterns, &should_interrupt, options, &mut delegate)
            .map_err(|e| SlocGuardError::Git(format!("Dirwalk failed: {e}")))?;

        Ok(delegate
            .files
            .into_iter()
            .map(|p| workdir_abs.join(p))
            .collect())
    }

    fn scan_with_structure_gix(
        &self,
        root: &Path,
        structure_config: Option<&StructureScanConfig>,
    ) -> Result<ScanResult> {
        let repo = gix::discover(root).map_err(|e| {
            SlocGuardError::GitRepoNotFound(format!("Failed to discover git repository: {e}"))
        })?;

        let workdir = repo
            .workdir()
            .ok_or_else(|| SlocGuardError::Git("Repository has no working directory".into()))?;

        let root_abs = root.canonicalize().map_err(|e| {
            SlocGuardError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Cannot canonicalize path {}: {e}", root.display()),
            ))
        })?;
        let workdir_abs = workdir.canonicalize().map_err(|e| {
            SlocGuardError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Cannot canonicalize workdir {}: {e}", workdir.display()),
            ))
        })?;

        let prefix = if root_abs == workdir_abs {
            PathBuf::new()
        } else {
            root_abs
                .strip_prefix(&workdir_abs)
                .map_err(|_| {
                    SlocGuardError::Git(format!(
                        "Scan path {} is not within git workdir {}",
                        root_abs.display(),
                        workdir_abs.display()
                    ))
                })?
                .to_path_buf()
        };

        let index = repo
            .index_or_empty()
            .map_err(|e| SlocGuardError::Git(format!("Failed to get git index: {e}")))?;

        let should_interrupt = AtomicBool::new(false);
        let options = repo
            .dirwalk_options()
            .map_err(|e| SlocGuardError::Git(format!("Failed to create dirwalk options: {e}")))?
            .emit_tracked(true)
            .emit_untracked(EmissionMode::Matching);

        let mut delegate = StructureAwareCollector::new(&prefix, &self.filter, structure_config);

        // Convert prefix to BStr for pattern matching
        // Normalize path separators to forward slashes for git pathspecs
        let prefix_string = prefix.to_string_lossy().replace('\\', "/");
        let prefix_str = gix::bstr::BString::from(prefix_string.as_bytes());
        let patterns: &[&BStr] = if prefix_str.is_empty() {
            &[]
        } else {
            &[prefix_str.as_bstr()]
        };

        repo.dirwalk(&index, patterns, &should_interrupt, options, &mut delegate)
            .map_err(|e| SlocGuardError::Git(format!("Dirwalk failed: {e}")))?;

        // Convert relative paths to absolute paths
        let files = delegate
            .files
            .into_iter()
            .map(|p| workdir_abs.join(p))
            .collect();

        let dir_stats = delegate
            .dir_stats
            .into_iter()
            .map(|(k, v)| (workdir_abs.join(k), v))
            .collect();

        let allowlist_violations = delegate
            .allowlist_violations
            .into_iter()
            .map(|mut v| {
                v.path = workdir_abs.join(&v.path);
                v
            })
            .collect();

        Ok(ScanResult {
            files,
            dir_stats,
            allowlist_violations,
        })
    }
}

impl<F: FileFilter + Send + Sync> FileScanner for GitAwareScanner<F> {
    fn scan(&self, root: &Path) -> Result<Vec<PathBuf>> {
        self.scan_with_gix(root)
    }

    fn scan_with_structure(
        &self,
        root: &Path,
        structure_config: Option<&StructureScanConfig>,
    ) -> Result<ScanResult> {
        self.scan_with_structure_gix(root, structure_config)
    }
}

/// Delegate for collecting file paths during dirwalk.
struct Collector<'a, F: FileFilter> {
    prefix: &'a Path,
    filter: &'a F,
    files: Vec<PathBuf>,
}

impl<'a, F: FileFilter> Collector<'a, F> {
    const fn new(prefix: &'a Path, filter: &'a F) -> Self {
        Self {
            prefix,
            filter,
            files: Vec::new(),
        }
    }
}

impl<F: FileFilter> gix::dir::walk::Delegate for Collector<'_, F> {
    fn emit(
        &mut self,
        entry: gix::dir::EntryRef<'_>,
        _collapsed_directory_status: Option<gix::dir::entry::Status>,
    ) -> gix::dir::walk::Action {
        // Only process files (not directories)
        if entry.disk_kind == Some(gix::dir::entry::Kind::File) {
            let path = entry.rela_path.to_path_lossy();

            // Check if within our scan prefix
            let in_prefix = self.prefix.as_os_str().is_empty() || path.starts_with(self.prefix);

            if in_prefix && self.filter.should_include(path.as_ref()) {
                self.files.push(path.into_owned());
            }
        }
        gix::dir::walk::Action::Continue
    }
}

/// Delegate for collecting files and directory stats during dirwalk.
struct StructureAwareCollector<'a, F: FileFilter> {
    prefix: &'a Path,
    filter: &'a F,
    structure_config: Option<&'a StructureScanConfig>,
    files: Vec<PathBuf>,
    dir_stats: HashMap<PathBuf, DirStats>,
    allowlist_violations: Vec<StructureViolation>,
    /// Track directories we've already counted to avoid double-counting.
    /// gix dirwalk only emits files, so we infer directories from file paths.
    seen_dirs: std::collections::HashSet<PathBuf>,
}

impl<'a, F: FileFilter> StructureAwareCollector<'a, F> {
    fn new(
        prefix: &'a Path,
        filter: &'a F,
        structure_config: Option<&'a StructureScanConfig>,
    ) -> Self {
        Self {
            prefix,
            filter,
            structure_config,
            files: Vec::new(),
            dir_stats: HashMap::new(),
            allowlist_violations: Vec::new(),
            seen_dirs: std::collections::HashSet::new(),
        }
    }

    /// Register a directory and all its ancestors, counting subdirectories.
    /// gix dirwalk only emits files, so we must infer directory structure from file paths.
    fn register_directory_chain(&mut self, dir_path: &Path) {
        let mut current = dir_path.to_path_buf();

        while !current.as_os_str().is_empty() {
            let depth = self.get_depth(&current);

            // Check if this directory should be excluded from counting
            let is_count_excluded = self
                .structure_config
                .is_some_and(|cfg| cfg.is_count_excluded(&current));

            // Initialize this directory's stats if not seen
            self.dir_stats
                .entry(current.clone())
                .or_insert_with(|| DirStats {
                    depth,
                    ..Default::default()
                });

            // If this is a new directory, increment parent's dir_count
            if !is_count_excluded
                && self.seen_dirs.insert(current.clone())
                && depth > 0
                && let Some(parent) = current.parent()
            {
                let parent_depth = depth - 1;
                let parent_stats =
                    self.dir_stats
                        .entry(parent.to_path_buf())
                        .or_insert_with(|| DirStats {
                            depth: parent_depth,
                            ..Default::default()
                        });
                parent_stats.dir_count += 1;
            }

            // Move to parent
            match current.parent() {
                Some(p) if !p.as_os_str().is_empty() => current = p.to_path_buf(),
                _ => break,
            }
        }
    }

    fn get_depth(&self, path: &Path) -> usize {
        // Count components relative to prefix
        if self.prefix.as_os_str().is_empty() {
            path.components().count()
        } else if let Ok(relative) = path.strip_prefix(self.prefix) {
            relative.components().count()
        } else {
            0
        }
    }
}

impl<F: FileFilter> gix::dir::walk::Delegate for StructureAwareCollector<'_, F> {
    fn emit(
        &mut self,
        entry: gix::dir::EntryRef<'_>,
        _collapsed_directory_status: Option<gix::dir::entry::Status>,
    ) -> gix::dir::walk::Action {
        let path = entry.rela_path.to_path_lossy();
        let path_ref: &Path = path.as_ref();

        // Check if within our scan prefix
        let in_prefix = self.prefix.as_os_str().is_empty() || path_ref.starts_with(self.prefix);
        if !in_prefix {
            return gix::dir::walk::Action::Continue;
        }

        let depth = self.get_depth(path_ref);
        let is_file = entry.disk_kind == Some(gix::dir::entry::Kind::File);
        let is_dir = entry.disk_kind == Some(gix::dir::entry::Kind::Directory);

        // Check scanner_exclude
        if let Some(cfg) = self.structure_config
            && cfg.is_scanner_excluded(path_ref, is_dir)
        {
            return gix::dir::walk::Action::Continue;
        }

        // Check count_exclude
        let is_count_excluded = self
            .structure_config
            .is_some_and(|cfg| cfg.is_count_excluded(path_ref));

        if is_file {
            // Add to files list if filter allows
            if self.filter.should_include(path_ref) {
                self.files.push(path.clone().into_owned());
            }

            // Count for parent directory (if not excluded)
            if !is_count_excluded && let Some(parent) = path_ref.parent() {
                let parent_depth = if depth > 0 { depth - 1 } else { 0 };
                let parent_stats =
                    self.dir_stats
                        .entry(parent.to_path_buf())
                        .or_insert_with(|| DirStats {
                            depth: parent_depth,
                            ..Default::default()
                        });
                parent_stats.file_count += 1;

                // Register the directory chain (infer directories from file paths)
                // gix dirwalk only emits files, so we must count subdirectories this way
                self.register_directory_chain(parent);

                // Check deny patterns and allowlist violations
                if let Some(cfg) = self.structure_config {
                    // 1. Check global deny patterns first (applies to all files)
                    // Use relative path for pattern matching (patterns are relative like "src/**")
                    if let Some(matched) = cfg.file_matches_global_deny(path_ref) {
                        self.allowlist_violations
                            .push(StructureViolation::denied_file(
                                path.clone().into_owned(),
                                "global".to_string(),
                                matched,
                            ));
                    } else if let Some(rule) = cfg.find_matching_allowlist_rule(parent) {
                        // 2. Check per-rule deny patterns
                        if let Some(matched) = rule.file_matches_deny(path_ref) {
                            self.allowlist_violations
                                .push(StructureViolation::denied_file(
                                    path.clone().into_owned(),
                                    rule.pattern.clone(),
                                    matched,
                                ));
                        } else {
                            // 3. Check allowlist (extensions/patterns) - only if configured
                            if rule.has_allowlist() && !rule.file_matches(path_ref) {
                                self.allowlist_violations.push(
                                    StructureViolation::disallowed_file(
                                        path.clone().into_owned(),
                                        rule.pattern.clone(),
                                    ),
                                );
                            }

                            // 4. Check naming convention
                            if !rule.filename_matches_naming_pattern(path_ref)
                                && let Some(ref pattern_str) = rule.naming_pattern_str
                            {
                                self.allowlist_violations.push(
                                    StructureViolation::naming_convention(
                                        path.clone().into_owned(),
                                        rule.pattern.clone(),
                                        pattern_str.clone(),
                                    ),
                                );
                            }
                        }
                    }
                }
            }
        } else if is_dir {
            // Note: gix dirwalk typically doesn't emit directory entries,
            // but we handle it for completeness
            self.dir_stats
                .entry(path.clone().into_owned())
                .or_insert_with(|| DirStats {
                    depth,
                    ..Default::default()
                });

            // Check directory-only deny patterns (patterns ending with `/`)
            if let Some(cfg) = self.structure_config
                && let Some(pattern) = cfg.dir_matches_global_deny(path_ref)
            {
                self.allowlist_violations
                    .push(StructureViolation::denied_directory(
                        path.clone().into_owned(),
                        "global".to_string(),
                        pattern,
                    ));
            }

            // Register directory chain for this directory
            self.register_directory_chain(path_ref);
        }

        gix::dir::walk::Action::Continue
    }
}

#[cfg(test)]
#[path = "gitignore_tests.rs"]
mod tests;
