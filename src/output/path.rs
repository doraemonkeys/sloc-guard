//! Path display utilities for consistent output formatting.
//!
//! This module provides utilities for displaying paths relative to the project root,
//! with consistent forward-slash separators across platforms.

use std::path::Path;

/// Format a path for display, making it relative to the project root if possible.
///
/// - If `project_root` is provided and `path` is a child of it, returns the relative path.
/// - Uses forward slashes as separators for consistent cross-platform output.
/// - If the path cannot be made relative, returns the path as-is with normalized separators.
/// - Returns `"."` for empty relative paths (e.g., when path equals `project_root`).
#[must_use]
pub fn display_path(path: &Path, project_root: Option<&Path>) -> String {
    let display_path = project_root.map_or_else(
        || path.to_path_buf(),
        |root| {
            path.strip_prefix(root)
                .map_or_else(|_| path.to_path_buf(), std::path::Path::to_path_buf)
        },
    );

    // Convert to string with forward slashes for consistent output
    let result = normalize_separators(&display_path.to_string_lossy());

    // Return "." for empty relative paths
    if result.is_empty() {
        ".".to_string()
    } else {
        result
    }
}

/// Normalize path separators to forward slashes.
#[must_use]
pub fn normalize_separators(path: &str) -> String {
    path.replace('\\', "/")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn display_path_with_project_root() {
        let project_root = PathBuf::from("/home/user/project");
        let file_path = PathBuf::from("/home/user/project/src/main.rs");

        let result = display_path(&file_path, Some(&project_root));
        assert_eq!(result, "src/main.rs");
    }

    #[test]
    fn display_path_without_project_root() {
        let file_path = PathBuf::from("/home/user/project/src/main.rs");

        let result = display_path(&file_path, None);
        // Should return the path as-is with normalized separators
        assert!(result.contains("src/main.rs"));
    }

    #[test]
    fn display_path_not_child_of_root() {
        let project_root = PathBuf::from("/home/user/project");
        let file_path = PathBuf::from("/home/other/file.rs");

        let result = display_path(&file_path, Some(&project_root));
        // Should return the path as-is since it's not under project_root
        assert!(result.contains("other/file.rs"));
    }

    #[test]
    fn display_path_same_as_root() {
        let project_root = PathBuf::from("/home/user/project");
        let file_path = PathBuf::from("/home/user/project");

        let result = display_path(&file_path, Some(&project_root));
        // Empty relative path becomes "."
        assert_eq!(result, ".");
    }

    #[test]
    fn normalize_separators_converts_backslashes() {
        assert_eq!(normalize_separators("src\\lib.rs"), "src/lib.rs");
        assert_eq!(
            normalize_separators("project\\src\\main.rs"),
            "project/src/main.rs"
        );
    }

    #[test]
    fn normalize_separators_preserves_forward_slashes() {
        assert_eq!(normalize_separators("src/lib.rs"), "src/lib.rs");
    }

    #[cfg(windows)]
    #[test]
    fn display_path_windows_paths() {
        let project_root = PathBuf::from(r"C:\Users\user\project");
        let file_path = PathBuf::from(r"C:\Users\user\project\src\main.rs");

        let result = display_path(&file_path, Some(&project_root));
        assert_eq!(result, "src/main.rs");
    }
}
