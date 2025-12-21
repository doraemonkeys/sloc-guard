use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::{Result, SlocGuardError};

/// Returns the set of files changed compared to a git reference.
pub trait ChangedFiles {
    /// Get files changed between the given reference and HEAD.
    ///
    /// # Errors
    /// Returns an error if the reference cannot be parsed or the repository cannot be accessed.
    fn get_changed_files(&self, base_ref: &str) -> Result<HashSet<PathBuf>>;
}

/// Git diff implementation using gix.
pub struct GitDiff {
    repo_path: PathBuf,
    workdir: PathBuf,
}

impl GitDiff {
    /// Create a new `GitDiff` for the repository containing the given path.
    ///
    /// # Errors
    /// Returns an error if no git repository is found.
    pub fn discover(path: &Path) -> Result<Self> {
        let repo = gix::discover(path)
            .map_err(|e| SlocGuardError::Git(format!("Failed to discover git repository: {e}")))?;
        let workdir = repo
            .workdir()
            .ok_or_else(|| SlocGuardError::Git("Repository has no working directory".into()))?
            .to_path_buf();
        Ok(Self {
            repo_path: repo.path().to_path_buf(),
            workdir,
        })
    }

    /// Get the working directory of the repository.
    #[must_use]
    pub fn workdir(&self) -> &Path {
        &self.workdir
    }

    fn open_repo(&self) -> Result<gix::Repository> {
        gix::open(&self.repo_path)
            .map_err(|e| SlocGuardError::Git(format!("Failed to open git repository: {e}")))
    }

    fn collect_tree_paths(
        tree: &gix::Tree<'_>,
        prefix: &Path,
    ) -> Result<HashSet<(PathBuf, gix::ObjectId)>> {
        let mut paths = HashSet::new();
        Self::collect_tree_paths_recursive(tree, prefix, &mut paths)?;
        Ok(paths)
    }

    fn collect_tree_paths_recursive(
        tree: &gix::Tree<'_>,
        prefix: &Path,
        paths: &mut HashSet<(PathBuf, gix::ObjectId)>,
    ) -> Result<()> {
        for entry in tree.iter() {
            let entry = entry
                .map_err(|e| SlocGuardError::Git(format!("Failed to read tree entry: {e}")))?;
            let name = std::str::from_utf8(entry.filename())
                .map_err(|e| SlocGuardError::Git(format!("Invalid filename encoding: {e}")))?;
            let path = prefix.join(name);

            match entry.mode().kind() {
                gix::object::tree::EntryKind::Blob
                | gix::object::tree::EntryKind::BlobExecutable => {
                    paths.insert((path, entry.oid().into()));
                }
                gix::object::tree::EntryKind::Tree => {
                    let subtree = entry.object().map_err(|e| {
                        SlocGuardError::Git(format!("Failed to get subtree object: {e}"))
                    })?;
                    let subtree = subtree.into_tree();
                    Self::collect_tree_paths_recursive(&subtree, &path, paths)?;
                }
                _ => {}
            }
        }
        Ok(())
    }

    /// Get files staged for commit (index differs from HEAD).
    ///
    /// # Errors
    /// Returns an error if the repository cannot be accessed.
    pub fn get_staged_files(&self) -> Result<HashSet<PathBuf>> {
        let repo = self.open_repo()?;

        // Get index (staging area)
        let index = repo
            .open_index()
            .map_err(|e| SlocGuardError::Git(format!("Failed to open git index: {e}")))?;

        // Get HEAD tree (if exists) - new repos have no commits yet
        let head_paths = match repo.head_commit() {
            Ok(commit) => {
                let head_tree = commit
                    .tree()
                    .map_err(|e| SlocGuardError::Git(format!("Failed to get HEAD tree: {e}")))?;
                Self::collect_tree_paths(&head_tree, Path::new(""))?
            }
            Err(_) => HashSet::new(),
        };

        let mut staged_files = HashSet::new();
        for entry in index.entries() {
            let path_str = String::from_utf8_lossy(entry.path(&index)).to_string();
            let path = PathBuf::from(&path_str);

            let is_staged = head_paths
                .iter()
                .find(|(p, _)| p == &path)
                .is_none_or(|(_, head_oid)| *head_oid != entry.id);

            if is_staged {
                staged_files.insert(self.workdir.join(path));
            }
        }

        Ok(staged_files)
    }
}

impl GitDiff {
    /// Get files changed between two git references.
    ///
    /// # Errors
    /// Returns an error if either reference cannot be parsed or the repository cannot be accessed.
    pub fn get_changed_files_range(
        &self,
        base_ref: &str,
        target_ref: &str,
    ) -> Result<HashSet<PathBuf>> {
        let repo = self.open_repo()?;

        // Parse the base reference
        let base_commit = repo
            .rev_parse_single(base_ref)
            .map_err(|e| {
                SlocGuardError::Git(format!("Failed to parse reference '{base_ref}': {e}"))
            })?
            .object()
            .map_err(|e| {
                SlocGuardError::Git(format!("Failed to get object for '{base_ref}': {e}"))
            })?
            .peel_to_commit()
            .map_err(|e| {
                SlocGuardError::Git(format!("Failed to peel to commit '{base_ref}': {e}"))
            })?;

        // Parse the target reference
        let target_commit = repo
            .rev_parse_single(target_ref)
            .map_err(|e| {
                SlocGuardError::Git(format!("Failed to parse reference '{target_ref}': {e}"))
            })?
            .object()
            .map_err(|e| {
                SlocGuardError::Git(format!("Failed to get object for '{target_ref}': {e}"))
            })?
            .peel_to_commit()
            .map_err(|e| {
                SlocGuardError::Git(format!("Failed to peel to commit '{target_ref}': {e}"))
            })?;

        // Get trees
        let base_tree = base_commit.tree().map_err(|e| {
            SlocGuardError::Git(format!("Failed to get tree for '{base_ref}': {e}"))
        })?;
        let target_tree = target_commit.tree().map_err(|e| {
            SlocGuardError::Git(format!("Failed to get tree for '{target_ref}': {e}"))
        })?;

        // Collect all file paths from both trees
        let base_paths = Self::collect_tree_paths(&base_tree, Path::new(""))?;
        let target_paths = Self::collect_tree_paths(&target_tree, Path::new(""))?;

        // Find changed files: files that exist in only one tree or have different OIDs
        let mut changed_files = HashSet::new();

        // Files in target but not in base, or changed
        for (path, oid) in &target_paths {
            let is_changed = base_paths
                .iter()
                .find(|(p, _)| p == path)
                .is_none_or(|(_, base_oid)| base_oid != oid);
            if is_changed {
                changed_files.insert(self.workdir.join(path));
            }
        }

        // Files deleted (in base but not in target) - include them as they might still exist locally
        for (path, _) in &base_paths {
            if !target_paths.iter().any(|(p, _)| p == path) {
                let full_path = self.workdir.join(path);
                if full_path.exists() {
                    changed_files.insert(full_path);
                }
            }
        }

        Ok(changed_files)
    }
}

impl ChangedFiles for GitDiff {
    fn get_changed_files(&self, base_ref: &str) -> Result<HashSet<PathBuf>> {
        // Default behavior: compare base_ref to HEAD
        self.get_changed_files_range(base_ref, "HEAD")
    }
}
