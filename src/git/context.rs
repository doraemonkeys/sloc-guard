//! Git context helpers for capturing repository state.

use std::path::Path;

/// Git repository context at a point in time.
///
/// Contains the commit hash (short form) and optionally the branch name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitContext {
    /// Short commit hash (e.g., "a1b2c3d")
    pub commit: String,
    /// Branch name if on a branch (None for detached HEAD)
    pub branch: Option<String>,
}

impl GitContext {
    /// Short hash length (7 characters, Git's default for short SHA)
    const SHORT_HASH_LEN: usize = 7;

    /// Get current git context for a repository at the given path.
    ///
    /// Returns `None` if:
    /// - Path is not in a git repository
    /// - Repository has no commits
    /// - Any git operation fails
    #[must_use]
    pub fn from_path(path: &Path) -> Option<Self> {
        let repo = gix::discover(path).ok()?;
        let head_commit = repo.head_commit().ok()?;
        let commit_id = head_commit.id();
        let short_hash = commit_id.to_string();
        let commit = short_hash
            .get(..Self::SHORT_HASH_LEN)
            .unwrap_or(&short_hash)
            .to_string();

        // Get current branch name (None if detached HEAD)
        let branch = repo
            .head_name()
            .ok()
            .flatten()
            .map(|name| name.shorten().to_string());

        Some(Self { commit, branch })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn git_context_from_current_repo() {
        // This test runs from within the sloc-guard repo
        let path = PathBuf::from(".");
        let context = GitContext::from_path(&path);

        // Should succeed since we're in a git repo
        assert!(context.is_some());
        let ctx = context.unwrap();

        // Should have a commit hash of expected length
        assert!(!ctx.commit.is_empty());
        assert!(ctx.commit.len() <= 7);

        // Branch may or may not be set (depends on CI vs local)
        // Just verify it doesn't panic
    }

    #[test]
    fn git_context_from_non_repo() {
        // Use a path that definitely isn't a git repo
        let context = GitContext::from_path(Path::new("/"));
        assert!(context.is_none());
    }

    #[test]
    fn git_context_fields() {
        let ctx = GitContext {
            commit: "a1b2c3d".to_string(),
            branch: Some("main".to_string()),
        };

        assert_eq!(ctx.commit, "a1b2c3d");
        assert_eq!(ctx.branch, Some("main".to_string()));
    }

    #[test]
    fn git_context_detached_head() {
        let ctx = GitContext {
            commit: "a1b2c3d".to_string(),
            branch: None,
        };

        assert_eq!(ctx.commit, "a1b2c3d");
        assert!(ctx.branch.is_none());
    }
}
