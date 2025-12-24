use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use gix::object::tree::EntryKind;

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

    /// Collect all blob paths from a tree (used when adding/removing entire subtrees).
    fn collect_all_blob_paths(tree: &gix::Tree<'_>, prefix: &Path) -> Result<Vec<PathBuf>> {
        let mut paths = Vec::new();
        Self::collect_all_blob_paths_recursive(tree, prefix, &mut paths)?;
        Ok(paths)
    }

    fn collect_all_blob_paths_recursive(
        tree: &gix::Tree<'_>,
        prefix: &Path,
        paths: &mut Vec<PathBuf>,
    ) -> Result<()> {
        for entry in tree.iter() {
            let entry = entry
                .map_err(|e| SlocGuardError::Git(format!("Failed to read tree entry: {e}")))?;
            let name = std::str::from_utf8(entry.filename())
                .map_err(|e| SlocGuardError::Git(format!("Invalid filename encoding: {e}")))?;
            let path = prefix.join(name);

            match entry.mode().kind() {
                EntryKind::Blob | EntryKind::BlobExecutable => {
                    paths.push(path);
                }
                EntryKind::Tree => {
                    let subtree = entry.object().map_err(|e| {
                        SlocGuardError::Git(format!("Failed to get subtree object: {e}"))
                    })?;
                    Self::collect_all_blob_paths_recursive(&subtree.into_tree(), &path, paths)?;
                }
                // Submodules (Commit) and symbolic links (Link) are intentionally skipped.
                // We only track regular file changes, not submodule pointer updates.
                EntryKind::Commit | EntryKind::Link => {}
            }
        }
        Ok(())
    }

    /// Build a map of HEAD tree entries for efficient O(1) lookups.
    fn build_head_path_map(
        tree: &gix::Tree<'_>,
        prefix: &Path,
    ) -> Result<HashMap<PathBuf, gix::ObjectId>> {
        let mut map = HashMap::new();
        Self::build_head_path_map_recursive(tree, prefix, &mut map)?;
        Ok(map)
    }

    fn build_head_path_map_recursive(
        tree: &gix::Tree<'_>,
        prefix: &Path,
        map: &mut HashMap<PathBuf, gix::ObjectId>,
    ) -> Result<()> {
        for entry in tree.iter() {
            let entry = entry
                .map_err(|e| SlocGuardError::Git(format!("Failed to read tree entry: {e}")))?;
            let name = std::str::from_utf8(entry.filename())
                .map_err(|e| SlocGuardError::Git(format!("Invalid filename encoding: {e}")))?;
            let path = prefix.join(name);

            match entry.mode().kind() {
                EntryKind::Blob | EntryKind::BlobExecutable => {
                    map.insert(path, entry.oid().into());
                }
                EntryKind::Tree => {
                    let subtree = entry.object().map_err(|e| {
                        SlocGuardError::Git(format!("Failed to get subtree object: {e}"))
                    })?;
                    Self::build_head_path_map_recursive(&subtree.into_tree(), &path, map)?;
                }
                // Submodules (Commit) and symbolic links (Link) are intentionally skipped.
                // We only track regular file changes, not submodule pointer updates.
                EntryKind::Commit | EntryKind::Link => {}
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
        // Use HashMap for O(1) lookup instead of HashSet with O(n) search
        let head_paths: HashMap<PathBuf, gix::ObjectId> = match repo.head_commit() {
            Ok(commit) => {
                let head_tree = commit
                    .tree()
                    .map_err(|e| SlocGuardError::Git(format!("Failed to get HEAD tree: {e}")))?;
                Self::build_head_path_map(&head_tree, Path::new(""))?
            }
            Err(_) => HashMap::new(),
        };

        let mut staged_files = HashSet::new();
        for entry in index.entries() {
            let path_str = String::from_utf8_lossy(entry.path(&index)).to_string();
            let path = PathBuf::from(&path_str);

            // O(1) lookup instead of O(n) search
            let is_staged = head_paths
                .get(&path)
                .is_none_or(|head_oid| *head_oid != entry.id);

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
    /// Uses optimized tree comparison that short-circuits when subtree OIDs match,
    /// avoiding unnecessary traversal of identical subtrees. This is significantly
    /// faster for large repositories where only a small portion of files changed.
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

        // Use optimized comparison that skips identical subtrees
        let mut changed_files = HashSet::new();
        let mut deleted_paths = Vec::new();
        Self::compare_trees_recursive(
            &base_tree,
            &target_tree,
            Path::new(""),
            &mut changed_files,
            &mut deleted_paths,
        )?;

        // Prepend workdir to all changed paths
        let changed_files: HashSet<PathBuf> = changed_files
            .into_iter()
            .map(|p| self.workdir.join(p))
            .collect();

        // For deleted files, only include them if they still exist locally
        let mut result = changed_files;
        for path in deleted_paths {
            let full_path = self.workdir.join(&path);
            if full_path.exists() {
                result.insert(full_path);
            }
        }

        Ok(result)
    }

    /// Compare two trees recursively, short-circuiting when subtree OIDs match.
    ///
    /// When a subtree has the same OID in both trees, we skip traversing it entirely
    /// since identical OIDs guarantee identical contents. This dramatically reduces
    /// the number of git objects we need to load for large repositories.
    ///
    /// # Parameters
    /// - `deleted_candidates`: Paths that were deleted in git but may still exist locally
    ///   (e.g., uncommitted changes). These require existence checks after traversal.
    fn compare_trees_recursive(
        base_tree: &gix::Tree<'_>,
        target_tree: &gix::Tree<'_>,
        prefix: &Path,
        changed: &mut HashSet<PathBuf>,
        deleted_candidates: &mut Vec<PathBuf>,
    ) -> Result<()> {
        // Build entry maps for efficient lookup by filename
        let base_entries = Self::build_entry_map(base_tree)?;
        let target_entries = Self::build_entry_map(target_tree)?;

        // Process entries in target tree (additions and modifications)
        for (name, target_entry) in &target_entries {
            let path = prefix.join(name);

            if let Some(base_entry) = base_entries.get(name) {
                // Entry exists in both trees - check if OIDs differ
                if base_entry.oid != target_entry.oid {
                    Self::process_changed_entry(
                        base_entry,
                        target_entry,
                        &path,
                        changed,
                        deleted_candidates,
                    )?;
                }
                // If OIDs are equal, entire subtree is identical - skip it
            } else {
                // Entry only in target - it's new, add all its blobs
                Self::process_added_entry(target_entry, &path, changed)?;
            }
        }

        // Process entries only in base tree (deletions)
        for (name, base_entry) in &base_entries {
            if !target_entries.contains_key(name) {
                let path = prefix.join(name);
                Self::process_deleted_entry(base_entry, &path, deleted_candidates)?;
            }
        }

        Ok(())
    }

    /// Process an entry that changed between base and target trees.
    fn process_changed_entry(
        base_entry: &TreeEntry<'_, '_>,
        target_entry: &TreeEntry<'_, '_>,
        path: &Path,
        changed: &mut HashSet<PathBuf>,
        deleted_candidates: &mut Vec<PathBuf>,
    ) -> Result<()> {
        match (base_entry.kind, target_entry.kind) {
            // Both are blobs - file content changed
            (
                EntryKind::Blob | EntryKind::BlobExecutable,
                EntryKind::Blob | EntryKind::BlobExecutable,
            ) => {
                changed.insert(path.to_path_buf());
            }
            // Both are trees - recurse to find specific changes
            (EntryKind::Tree, EntryKind::Tree) => {
                let base_subtree = base_entry
                    .entry
                    .object()
                    .map_err(|e| SlocGuardError::Git(format!("Failed to get base subtree: {e}")))?;
                let target_subtree = target_entry.entry.object().map_err(|e| {
                    SlocGuardError::Git(format!("Failed to get target subtree: {e}"))
                })?;
                Self::compare_trees_recursive(
                    &base_subtree.into_tree(),
                    &target_subtree.into_tree(),
                    path,
                    changed,
                    deleted_candidates,
                )?;
            }
            // Type changed: tree -> blob (directory became file)
            (EntryKind::Tree, EntryKind::Blob | EntryKind::BlobExecutable) => {
                // All files in the old directory are "deleted"
                Self::process_deleted_entry(base_entry, path, deleted_candidates)?;
                // The new file is added
                changed.insert(path.to_path_buf());
            }
            // Type changed: blob -> tree (file became directory)
            (EntryKind::Blob | EntryKind::BlobExecutable, EntryKind::Tree) => {
                // Old file is "deleted"
                deleted_candidates.push(path.to_path_buf());
                // All files in the new directory are added
                Self::process_added_entry(target_entry, path, changed)?;
            }
            // Submodules (Commit) and symbolic links (Link) are intentionally skipped.
            // We only track regular file changes, not submodule pointer updates.
            (EntryKind::Commit | EntryKind::Link, _) | (_, EntryKind::Commit | EntryKind::Link) => {
            }
        }
        Ok(())
    }

    /// Process an entry that only exists in the target tree (addition).
    fn process_added_entry(
        entry: &TreeEntry<'_, '_>,
        path: &Path,
        changed: &mut HashSet<PathBuf>,
    ) -> Result<()> {
        match entry.kind {
            EntryKind::Blob | EntryKind::BlobExecutable => {
                changed.insert(path.to_path_buf());
            }
            EntryKind::Tree => {
                let subtree = entry.object().map_err(|e| {
                    SlocGuardError::Git(format!("Failed to get subtree object: {e}"))
                })?;
                let paths = Self::collect_all_blob_paths(&subtree.into_tree(), path)?;
                changed.extend(paths);
            }
            // Submodules (Commit) and symbolic links (Link) are intentionally skipped.
            // We only track regular file changes, not submodule pointer updates.
            EntryKind::Commit | EntryKind::Link => {}
        }
        Ok(())
    }

    /// Process an entry that only exists in the base tree (deletion).
    fn process_deleted_entry(
        entry: &TreeEntry<'_, '_>,
        path: &Path,
        deleted_candidates: &mut Vec<PathBuf>,
    ) -> Result<()> {
        match entry.kind {
            EntryKind::Blob | EntryKind::BlobExecutable => {
                deleted_candidates.push(path.to_path_buf());
            }
            EntryKind::Tree => {
                let subtree = entry.object().map_err(|e| {
                    SlocGuardError::Git(format!("Failed to get subtree object: {e}"))
                })?;
                let paths = Self::collect_all_blob_paths(&subtree.into_tree(), path)?;
                deleted_candidates.extend(paths);
            }
            // Submodules (Commit) and symbolic links (Link) are intentionally skipped.
            // We only track regular file changes, not submodule pointer updates.
            EntryKind::Commit | EntryKind::Link => {}
        }
        Ok(())
    }

    /// Build a map of tree entries by filename for efficient lookup.
    fn build_entry_map<'repo, 'a>(
        tree: &'a gix::Tree<'repo>,
    ) -> Result<HashMap<String, TreeEntry<'repo, 'a>>> {
        let mut map = HashMap::new();
        for entry in tree.iter() {
            let entry = entry
                .map_err(|e| SlocGuardError::Git(format!("Failed to read tree entry: {e}")))?;
            let name = std::str::from_utf8(entry.filename())
                .map_err(|e| SlocGuardError::Git(format!("Invalid filename encoding: {e}")))?
                .to_string();
            let kind = entry.mode().kind();
            let oid = entry.oid().into();
            map.insert(name, TreeEntry { entry, kind, oid });
        }
        Ok(map)
    }
}

/// Helper struct to hold tree entry data for efficient comparison.
///
/// Caches the entry's kind and OID to avoid repeated lookups during
/// tree comparison. Holds a reference to the underlying `EntryRef`
/// for deferred object loading when we need to recurse into subtrees.
struct TreeEntry<'repo, 'a> {
    entry: gix::object::tree::EntryRef<'repo, 'a>,
    kind: EntryKind,
    oid: gix::ObjectId,
}

impl TreeEntry<'_, '_> {
    fn object(&self) -> std::result::Result<gix::Object<'_>, gix::object::find::existing::Error> {
        self.entry.object()
    }
}

impl ChangedFiles for GitDiff {
    fn get_changed_files(&self, base_ref: &str) -> Result<HashSet<PathBuf>> {
        // Default behavior: compare base_ref to HEAD
        self.get_changed_files_range(base_ref, "HEAD")
    }
}
