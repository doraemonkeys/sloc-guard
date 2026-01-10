//! Path display utilities for consistent output formatting.
//!
//! This module provides utilities for displaying paths relative to the project root,
//! with consistent forward-slash separators across platforms.

use std::path::{Path, PathBuf};

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

/// Normalize a path for consistent glob pattern matching.
///
/// This function performs two normalizations:
/// 1. Strips leading `.` component (e.g., `./src/foo` or `.\src\foo` → `src/foo`)
/// 2. Normalizes backslashes to forward slashes for cross-platform glob matching
///
/// This ensures patterns like `src/**/*tests.rs` match regardless of whether
/// the path is specified as `src/lib.rs`, `./src/lib.rs`, or `.\src\lib.rs`.
///
/// # Edge Case: Root Directory
///
/// When the input is `.` or `./`, returns an empty `PathBuf`. This is intentional:
/// an empty path won't match file-targeting patterns like `src/**/*.rs`, which is
/// correct since the project root itself is not a file. For glob patterns that
/// should match everything (like `**`), `GlobSet` handles empty strings correctly.
#[must_use]
pub(crate) fn normalize_for_matching(path: &Path) -> PathBuf {
    let path_str = path.to_string_lossy();

    // Strip leading "./" (Unix) or ".\" (Windows)
    let stripped = path_str
        .strip_prefix("./")
        .or_else(|| path_str.strip_prefix(".\\"))
        .unwrap_or(&path_str);

    // Handle bare "." - return empty path (see doc comment for rationale)
    if stripped.is_empty() || stripped == "." {
        return PathBuf::new();
    }

    // Normalize backslashes to forward slashes for consistent glob matching on all platforms.
    if stripped.contains('\\') {
        PathBuf::from(stripped.replace('\\', "/"))
    } else {
        PathBuf::from(stripped)
    }
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

    // Tests for normalize_for_matching
    #[test]
    fn normalize_for_matching_strips_dot_slash() {
        assert_eq!(
            normalize_for_matching(Path::new("./src/lib.rs")),
            PathBuf::from("src/lib.rs")
        );
    }

    #[test]
    fn normalize_for_matching_strips_dot_backslash() {
        assert_eq!(
            normalize_for_matching(Path::new(".\\src\\lib.rs")),
            PathBuf::from("src/lib.rs")
        );
    }

    #[test]
    fn normalize_for_matching_normalizes_backslashes() {
        assert_eq!(
            normalize_for_matching(Path::new("src\\cache\\cache_tests.rs")),
            PathBuf::from("src/cache/cache_tests.rs")
        );
    }

    #[test]
    fn normalize_for_matching_preserves_plain_paths() {
        assert_eq!(
            normalize_for_matching(Path::new("src/lib.rs")),
            PathBuf::from("src/lib.rs")
        );
    }

    #[test]
    fn normalize_for_matching_handles_bare_dot() {
        assert_eq!(normalize_for_matching(Path::new(".")), PathBuf::new());
    }

    #[test]
    fn normalize_for_matching_handles_dot_slash_only() {
        assert_eq!(normalize_for_matching(Path::new("./")), PathBuf::new());
        assert_eq!(normalize_for_matching(Path::new(".\\")), PathBuf::new());
    }

    #[test]
    fn normalize_for_matching_preserves_parent_dir() {
        assert_eq!(
            normalize_for_matching(Path::new("../src")),
            PathBuf::from("../src")
        );
        assert_eq!(
            normalize_for_matching(Path::new("..\\src")),
            PathBuf::from("../src")
        );
    }

    #[test]
    fn normalize_for_matching_preserves_dot_in_filename() {
        assert_eq!(
            normalize_for_matching(Path::new("src/.gitignore")),
            PathBuf::from("src/.gitignore")
        );
        assert_eq!(
            normalize_for_matching(Path::new("src\\.eslintrc")),
            PathBuf::from("src/.eslintrc")
        );
    }

    #[test]
    fn normalize_for_matching_handles_mixed_separators() {
        // Path with Unix prefix "./" but Windows separators internally
        assert_eq!(
            normalize_for_matching(Path::new("./src\\lib")),
            PathBuf::from("src/lib")
        );
        // Path with Windows prefix ".\" but Unix separators internally
        assert_eq!(
            normalize_for_matching(Path::new(".\\src/lib")),
            PathBuf::from("src/lib")
        );
    }
}
