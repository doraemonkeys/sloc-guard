use std::path::Path;

/// Check if a path matches an override path using suffix matching.
///
/// This function normalizes path separators (handles both `/` and `\\`)
/// and performs a suffix match - the override path components must match
/// the trailing components of the actual path.
///
/// # Examples
///
/// - Override `"src/lib.rs"` matches `"project/src/lib.rs"`
/// - Override `"lib.rs"` matches `"src/lib.rs"` and `"other/lib.rs"`
/// - Override `"src/lib.rs"` does NOT match `"other/src/lib.rs"` (full suffix must match)
#[must_use]
pub fn path_matches_override(actual_path: &Path, override_path: &str) -> bool {
    let override_components: Vec<&str> = override_path
        .split(['/', '\\'])
        .filter(|s| !s.is_empty())
        .collect();

    let path_components: Vec<_> = actual_path.components().collect();

    if override_components.is_empty() || override_components.len() > path_components.len() {
        return false;
    }

    path_components
        .iter()
        .rev()
        .zip(override_components.iter().rev())
        .all(|(path_comp, override_comp)| {
            path_comp.as_os_str().to_string_lossy() == *override_comp
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_exact_filename_match() {
        let path = PathBuf::from("src/lib.rs");
        assert!(path_matches_override(&path, "lib.rs"));
    }

    #[test]
    fn test_partial_path_match() {
        let path = PathBuf::from("project/src/lib.rs");
        assert!(path_matches_override(&path, "src/lib.rs"));
    }

    #[test]
    fn test_full_path_match() {
        let path = PathBuf::from("project/src/lib.rs");
        assert!(path_matches_override(&path, "project/src/lib.rs"));
    }

    #[test]
    fn test_no_match_different_directory() {
        let path = PathBuf::from("other/src/lib.rs");
        // "project/src/lib.rs" should not match because prefix differs
        assert!(!path_matches_override(&path, "project/src/lib.rs"));
    }

    #[test]
    fn test_no_match_override_longer_than_path() {
        let path = PathBuf::from("lib.rs");
        assert!(!path_matches_override(&path, "src/lib.rs"));
    }

    #[test]
    fn test_empty_override_no_match() {
        let path = PathBuf::from("src/lib.rs");
        assert!(!path_matches_override(&path, ""));
    }

    #[test]
    fn test_windows_separator_in_override() {
        let path = PathBuf::from("project/src/lib.rs");
        assert!(path_matches_override(&path, "src\\lib.rs"));
    }

    #[test]
    fn test_mixed_separators() {
        let path = PathBuf::from("project/src/components/Button.tsx");
        assert!(path_matches_override(&path, "src/components\\Button.tsx"));
    }

    #[test]
    fn test_directory_path() {
        let path = PathBuf::from("project/src/components");
        assert!(path_matches_override(&path, "src/components"));
    }

    #[test]
    fn test_trailing_separator_filtered() {
        let path = PathBuf::from("src/components");
        // Trailing separator should be filtered out
        assert!(path_matches_override(&path, "src/components/"));
    }
}
