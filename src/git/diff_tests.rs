use std::path::Path;
use std::process::Command;

use tempfile::TempDir;

use super::*;

fn create_git_repo() -> TempDir {
    let dir = TempDir::new().unwrap();

    // Initialize git repo
    Command::new("git")
        .args(["init"])
        .current_dir(dir.path())
        .output()
        .expect("Failed to init git repo");

    // Configure git user
    Command::new("git")
        .args(["config", "user.email", "test@test.com"])
        .current_dir(dir.path())
        .output()
        .expect("Failed to config git user email");

    Command::new("git")
        .args(["config", "user.name", "Test User"])
        .current_dir(dir.path())
        .output()
        .expect("Failed to config git user name");

    dir
}

fn create_file(dir: &Path, name: &str, content: &str) {
    std::fs::write(dir.join(name), content).unwrap();
}

fn git_add_all(dir: &Path) {
    Command::new("git")
        .args(["add", "."])
        .current_dir(dir)
        .output()
        .expect("Failed to git add");
}

fn git_commit(dir: &Path, message: &str) {
    Command::new("git")
        .args(["commit", "-m", message])
        .current_dir(dir)
        .output()
        .expect("Failed to git commit");
}

#[test]
fn discover_finds_git_repo() {
    let dir = create_git_repo();
    let result = GitDiff::discover(dir.path());
    assert!(result.is_ok());
}

#[test]
fn discover_fails_for_non_git_directory() {
    // Create a temp dir that's not inside a git repo by using a path that
    // gix won't find a .git directory when traversing up.
    // On most systems, the root directory is not a git repo.
    // Since gix::discover searches parent directories, we need to test with
    // a directory that has no .git in its ancestry.
    //
    // Note: This test may pass or fail depending on where the test is run.
    // If the temp directory is inside a git repo, discover will succeed.
    // We test the error path by using a non-existent path instead.
    let result = GitDiff::discover(Path::new("/nonexistent/path/that/does/not/exist"));
    assert!(result.is_err());
}

#[test]
fn changed_files_detects_added_file() {
    let dir = create_git_repo();

    // Create initial file and commit
    create_file(dir.path(), "initial.rs", "fn main() {}");
    git_add_all(dir.path());
    git_commit(dir.path(), "Initial commit");

    // Create a new file and commit
    create_file(dir.path(), "new_file.rs", "fn new() {}");
    git_add_all(dir.path());
    git_commit(dir.path(), "Add new file");

    let git_diff = GitDiff::discover(dir.path()).unwrap();
    let changed = git_diff.get_changed_files("HEAD~1").unwrap();

    let new_file_path = dir.path().join("new_file.rs").canonicalize().unwrap();
    assert!(
        changed
            .iter()
            .any(|p| p.canonicalize().ok() == Some(new_file_path.clone()))
    );
}

#[test]
fn changed_files_detects_modified_file() {
    let dir = create_git_repo();

    // Create initial file and commit
    create_file(dir.path(), "main.rs", "fn main() {}");
    git_add_all(dir.path());
    git_commit(dir.path(), "Initial commit");

    // Modify the file and commit
    create_file(dir.path(), "main.rs", "fn main() { println!(\"hello\"); }");
    git_add_all(dir.path());
    git_commit(dir.path(), "Modify main.rs");

    let git_diff = GitDiff::discover(dir.path()).unwrap();
    let changed = git_diff.get_changed_files("HEAD~1").unwrap();

    let main_path = dir.path().join("main.rs").canonicalize().unwrap();
    assert!(
        changed
            .iter()
            .any(|p| p.canonicalize().ok() == Some(main_path.clone()))
    );
}

#[test]
fn changed_files_empty_when_no_changes() {
    let dir = create_git_repo();

    // Create initial file and commit
    create_file(dir.path(), "main.rs", "fn main() {}");
    git_add_all(dir.path());
    git_commit(dir.path(), "Initial commit");

    let git_diff = GitDiff::discover(dir.path()).unwrap();
    let changed = git_diff.get_changed_files("HEAD").unwrap();

    assert!(changed.is_empty());
}

#[test]
fn changed_files_invalid_reference_returns_error() {
    let dir = create_git_repo();

    // Create initial file and commit
    create_file(dir.path(), "main.rs", "fn main() {}");
    git_add_all(dir.path());
    git_commit(dir.path(), "Initial commit");

    let git_diff = GitDiff::discover(dir.path()).unwrap();
    let result = git_diff.get_changed_files("nonexistent-branch");

    assert!(result.is_err());
}

#[test]
fn workdir_returns_correct_path() {
    let dir = create_git_repo();
    let git_diff = GitDiff::discover(dir.path()).unwrap();

    let workdir = git_diff.workdir().canonicalize().unwrap();
    let expected = dir.path().canonicalize().unwrap();
    assert_eq!(workdir, expected);
}

#[test]
fn changed_files_handles_subdirectory() {
    let dir = create_git_repo();

    // Create initial file and commit
    std::fs::create_dir(dir.path().join("src")).unwrap();
    create_file(dir.path(), "src/main.rs", "fn main() {}");
    git_add_all(dir.path());
    git_commit(dir.path(), "Initial commit");

    // Add a new file in subdirectory
    create_file(dir.path(), "src/lib.rs", "pub fn lib() {}");
    git_add_all(dir.path());
    git_commit(dir.path(), "Add lib.rs");

    let git_diff = GitDiff::discover(dir.path()).unwrap();
    let changed = git_diff.get_changed_files("HEAD~1").unwrap();

    let lib_path = dir.path().join("src/lib.rs").canonicalize().unwrap();
    assert!(
        changed
            .iter()
            .any(|p| p.canonicalize().ok() == Some(lib_path.clone()))
    );
}

#[test]
fn staged_files_detects_staged_file() {
    let dir = create_git_repo();

    // Create initial file and commit
    create_file(dir.path(), "main.rs", "fn main() {}");
    git_add_all(dir.path());
    git_commit(dir.path(), "Initial commit");

    // Create a new file and stage it (but don't commit)
    create_file(dir.path(), "staged.rs", "fn staged() {}");
    git_add_all(dir.path());

    let git_diff = GitDiff::discover(dir.path()).unwrap();
    let staged = git_diff.get_staged_files().unwrap();

    let staged_path = dir.path().join("staged.rs").canonicalize().unwrap();
    assert!(
        staged
            .iter()
            .any(|p| p.canonicalize().ok() == Some(staged_path.clone()))
    );
}

#[test]
fn staged_files_empty_when_nothing_staged() {
    let dir = create_git_repo();

    // Create file, stage, and commit
    create_file(dir.path(), "main.rs", "fn main() {}");
    git_add_all(dir.path());
    git_commit(dir.path(), "Initial commit");

    let git_diff = GitDiff::discover(dir.path()).unwrap();
    let staged = git_diff.get_staged_files().unwrap();

    assert!(staged.is_empty());
}

#[test]
fn staged_files_ignores_unstaged_changes() {
    let dir = create_git_repo();

    // Create file, stage, and commit
    create_file(dir.path(), "main.rs", "fn main() {}");
    git_add_all(dir.path());
    git_commit(dir.path(), "Initial commit");

    // Modify file but don't stage it
    create_file(
        dir.path(),
        "main.rs",
        "fn main() { println!(\"modified\"); }",
    );

    let git_diff = GitDiff::discover(dir.path()).unwrap();
    let staged = git_diff.get_staged_files().unwrap();

    // Unstaged changes should not be included
    assert!(staged.is_empty());
}

// ============================================================================
// Range Comparison Tests (for --diff base..target syntax)
// ============================================================================

fn git_create_branch(dir: &Path, name: &str) {
    Command::new("git")
        .args(["branch", name])
        .current_dir(dir)
        .output()
        .expect("Failed to create branch");
}

fn git_checkout(dir: &Path, name: &str) {
    Command::new("git")
        .args(["checkout", name])
        .current_dir(dir)
        .output()
        .expect("Failed to checkout branch");
}

#[test]
fn changed_files_range_between_branches() {
    let dir = create_git_repo();

    // Create initial file and commit on main
    create_file(dir.path(), "initial.rs", "fn main() {}");
    git_add_all(dir.path());
    git_commit(dir.path(), "Initial commit");

    // Create a feature branch
    git_create_branch(dir.path(), "feature");
    git_checkout(dir.path(), "feature");

    // Add a new file on feature branch
    create_file(dir.path(), "feature.rs", "fn feature() {}");
    git_add_all(dir.path());
    git_commit(dir.path(), "Add feature file");

    // Stay on feature branch for the test (so feature.rs exists)
    let git_diff = GitDiff::discover(dir.path()).unwrap();

    // Compare master..feature should show feature.rs as changed
    let changed = git_diff
        .get_changed_files_range("master", "feature")
        .unwrap();

    // Check that feature.rs is in the changed files by looking at path endings
    let has_feature_rs = changed
        .iter()
        .any(|p| p.file_name().is_some_and(|name| name == "feature.rs"));
    assert!(has_feature_rs, "Expected feature.rs to be in changed files");
}

#[test]
fn changed_files_range_between_tags() {
    let dir = create_git_repo();

    // Create initial file and commit
    create_file(dir.path(), "main.rs", "fn main() {}");
    git_add_all(dir.path());
    git_commit(dir.path(), "Initial commit");

    // Create v1.0 tag
    Command::new("git")
        .args(["tag", "v1.0"])
        .current_dir(dir.path())
        .output()
        .expect("Failed to create tag");

    // Add another file
    create_file(dir.path(), "lib.rs", "pub fn lib() {}");
    git_add_all(dir.path());
    git_commit(dir.path(), "Add lib");

    // Create v2.0 tag
    Command::new("git")
        .args(["tag", "v2.0"])
        .current_dir(dir.path())
        .output()
        .expect("Failed to create tag");

    let git_diff = GitDiff::discover(dir.path()).unwrap();

    // Compare v1.0..v2.0 should show lib.rs as changed
    let changed = git_diff.get_changed_files_range("v1.0", "v2.0").unwrap();

    let lib_path = dir.path().join("lib.rs").canonicalize().unwrap();
    assert!(
        changed
            .iter()
            .any(|p| p.canonicalize().ok() == Some(lib_path.clone())),
        "Expected lib.rs to be in changed files between tags"
    );
}

#[test]
fn changed_files_range_same_ref_returns_empty() {
    let dir = create_git_repo();

    // Create file and commit
    create_file(dir.path(), "main.rs", "fn main() {}");
    git_add_all(dir.path());
    git_commit(dir.path(), "Initial commit");

    let git_diff = GitDiff::discover(dir.path()).unwrap();

    // Compare HEAD..HEAD should return empty
    let changed = git_diff.get_changed_files_range("HEAD", "HEAD").unwrap();
    assert!(changed.is_empty());
}

#[test]
fn changed_files_range_invalid_ref_returns_error() {
    let dir = create_git_repo();

    create_file(dir.path(), "main.rs", "fn main() {}");
    git_add_all(dir.path());
    git_commit(dir.path(), "Initial commit");

    let git_diff = GitDiff::discover(dir.path()).unwrap();

    // Invalid base ref
    let result = git_diff.get_changed_files_range("nonexistent", "HEAD");
    assert!(result.is_err());

    // Invalid target ref
    let result = git_diff.get_changed_files_range("HEAD", "nonexistent");
    assert!(result.is_err());
}

#[test]
fn changed_files_range_matches_trait_behavior() {
    let dir = create_git_repo();

    // Create initial file and commit
    create_file(dir.path(), "main.rs", "fn main() {}");
    git_add_all(dir.path());
    git_commit(dir.path(), "Initial commit");

    // Modify and commit
    create_file(dir.path(), "main.rs", "fn main() { println!(\"hello\"); }");
    git_add_all(dir.path());
    git_commit(dir.path(), "Modify main");

    let git_diff = GitDiff::discover(dir.path()).unwrap();

    // These should produce the same result
    let from_trait = git_diff.get_changed_files("HEAD~1").unwrap();
    let from_range = git_diff.get_changed_files_range("HEAD~1", "HEAD").unwrap();

    assert_eq!(
        from_trait, from_range,
        "Trait method and range method should produce same results"
    );
}

// ============================================================================
// Optimized Tree Comparison Tests
// ============================================================================

#[test]
fn changed_files_detects_deleted_file() {
    let dir = create_git_repo();

    // Create files and commit
    create_file(dir.path(), "keep.rs", "fn keep() {}");
    create_file(dir.path(), "delete_me.rs", "fn delete() {}");
    git_add_all(dir.path());
    git_commit(dir.path(), "Initial commit");

    // Delete one file and commit
    std::fs::remove_file(dir.path().join("delete_me.rs")).unwrap();
    git_add_all(dir.path());
    git_commit(dir.path(), "Delete file");

    // Restore the file locally (simulates uncommitted restoration)
    create_file(dir.path(), "delete_me.rs", "fn restored() {}");

    let git_diff = GitDiff::discover(dir.path()).unwrap();
    let changed = git_diff.get_changed_files("HEAD~1").unwrap();

    // The deleted file should be in changed set since it still exists locally
    let deleted_path = dir.path().join("delete_me.rs").canonicalize().unwrap();
    assert!(
        changed
            .iter()
            .any(|p| p.canonicalize().ok() == Some(deleted_path.clone())),
        "Deleted file that exists locally should be in changed set"
    );
}

#[test]
fn changed_files_skips_deleted_file_not_on_disk() {
    let dir = create_git_repo();

    // Create files and commit
    create_file(dir.path(), "keep.rs", "fn keep() {}");
    create_file(dir.path(), "delete_me.rs", "fn delete() {}");
    git_add_all(dir.path());
    git_commit(dir.path(), "Initial commit");

    // Delete one file and commit
    std::fs::remove_file(dir.path().join("delete_me.rs")).unwrap();
    git_add_all(dir.path());
    git_commit(dir.path(), "Delete file");

    // Don't restore the file - it's gone from disk

    let git_diff = GitDiff::discover(dir.path()).unwrap();
    let changed = git_diff.get_changed_files("HEAD~1").unwrap();

    // The deleted file should NOT be in changed set since it doesn't exist locally
    let has_deleted = changed
        .iter()
        .any(|p| p.file_name().is_some_and(|name| name == "delete_me.rs"));
    assert!(
        !has_deleted,
        "Deleted file that doesn't exist locally should not be in changed set"
    );
}

#[test]
fn changed_files_detects_file_to_directory_change() {
    let dir = create_git_repo();

    // Create a file and commit
    create_file(dir.path(), "module", "// single file module");
    git_add_all(dir.path());
    git_commit(dir.path(), "Initial commit with file");

    // Replace file with directory containing files
    std::fs::remove_file(dir.path().join("module")).unwrap();
    std::fs::create_dir(dir.path().join("module")).unwrap();
    create_file(dir.path(), "module/mod.rs", "mod sub;");
    create_file(dir.path(), "module/sub.rs", "pub fn sub() {}");
    git_add_all(dir.path());
    git_commit(dir.path(), "Convert file to directory");

    let git_diff = GitDiff::discover(dir.path()).unwrap();
    let changed = git_diff.get_changed_files("HEAD~1").unwrap();

    // Should detect the new files in the directory
    let has_mod = changed
        .iter()
        .any(|p| p.to_string_lossy().contains("module") && p.to_string_lossy().contains("mod.rs"));
    let has_sub = changed
        .iter()
        .any(|p| p.to_string_lossy().contains("module") && p.to_string_lossy().contains("sub.rs"));

    assert!(has_mod, "Expected module/mod.rs to be in changed files");
    assert!(has_sub, "Expected module/sub.rs to be in changed files");
}

#[test]
fn changed_files_detects_directory_to_file_change() {
    let dir = create_git_repo();

    // Create a directory with files and commit
    std::fs::create_dir(dir.path().join("module")).unwrap();
    create_file(dir.path(), "module/mod.rs", "mod sub;");
    create_file(dir.path(), "module/sub.rs", "pub fn sub() {}");
    git_add_all(dir.path());
    git_commit(dir.path(), "Initial commit with directory");

    // Replace directory with file
    std::fs::remove_dir_all(dir.path().join("module")).unwrap();
    create_file(dir.path(), "module", "// single file module");
    git_add_all(dir.path());
    git_commit(dir.path(), "Convert directory to file");

    let git_diff = GitDiff::discover(dir.path()).unwrap();
    let changed = git_diff.get_changed_files("HEAD~1").unwrap();

    // Should detect the new file
    let has_module = changed
        .iter()
        .any(|p| p.file_name().is_some_and(|name| name == "module"));
    assert!(has_module, "Expected module file to be in changed files");
}

#[test]
fn changed_files_handles_nested_directory_addition() {
    let dir = create_git_repo();

    // Create initial file and commit
    create_file(dir.path(), "main.rs", "fn main() {}");
    git_add_all(dir.path());
    git_commit(dir.path(), "Initial commit");

    // Add nested directory structure
    std::fs::create_dir_all(dir.path().join("src/deep/nested")).unwrap();
    create_file(dir.path(), "src/lib.rs", "mod deep;");
    create_file(dir.path(), "src/deep/mod.rs", "mod nested;");
    create_file(dir.path(), "src/deep/nested/mod.rs", "pub fn deep() {}");
    git_add_all(dir.path());
    git_commit(dir.path(), "Add nested directories");

    let git_diff = GitDiff::discover(dir.path()).unwrap();
    let changed = git_diff.get_changed_files("HEAD~1").unwrap();

    // Should detect all new files
    assert!(changed.len() >= 3, "Expected at least 3 new files");

    let has_nested = changed
        .iter()
        .any(|p| p.to_string_lossy().contains("nested"));
    assert!(has_nested, "Expected nested directory files to be detected");
}

#[test]
fn changed_files_handles_nested_directory_deletion() {
    let dir = create_git_repo();

    // Create nested directory structure and commit
    std::fs::create_dir_all(dir.path().join("src/deep/nested")).unwrap();
    create_file(dir.path(), "src/lib.rs", "mod deep;");
    create_file(dir.path(), "src/deep/mod.rs", "mod nested;");
    create_file(dir.path(), "src/deep/nested/mod.rs", "pub fn deep() {}");
    git_add_all(dir.path());
    git_commit(dir.path(), "Initial commit with nested dirs");

    // Delete nested directory and commit
    std::fs::remove_dir_all(dir.path().join("src/deep")).unwrap();
    std::fs::write(dir.path().join("src/lib.rs"), "// no more deep").unwrap();
    git_add_all(dir.path());
    git_commit(dir.path(), "Remove nested directories");

    let git_diff = GitDiff::discover(dir.path()).unwrap();
    let changed = git_diff.get_changed_files("HEAD~1").unwrap();

    // Should detect lib.rs as changed
    let has_lib = changed
        .iter()
        .any(|p| p.file_name().is_some_and(|name| name == "lib.rs"));
    assert!(has_lib, "Expected lib.rs to be in changed files");

    // Deleted files should not appear (they don't exist on disk)
    let has_nested = changed
        .iter()
        .any(|p| p.to_string_lossy().contains("nested"));
    assert!(
        !has_nested,
        "Deleted nested files should not appear (don't exist on disk)"
    );
}

#[test]
fn staged_files_in_new_repo_without_commits() {
    let dir = create_git_repo();

    // Stage a file without committing (no HEAD exists yet)
    create_file(dir.path(), "new.rs", "fn new() {}");
    git_add_all(dir.path());

    let git_diff = GitDiff::discover(dir.path()).unwrap();
    let staged = git_diff.get_staged_files().unwrap();

    // All staged files should be detected
    let new_path = dir.path().join("new.rs").canonicalize().unwrap();
    assert!(
        staged
            .iter()
            .any(|p| p.canonicalize().ok() == Some(new_path.clone())),
        "Staged file in repo without commits should be detected"
    );
}

#[test]
fn staged_files_detects_modified_staged_file() {
    let dir = create_git_repo();

    // Create file, commit
    create_file(dir.path(), "main.rs", "fn main() {}");
    git_add_all(dir.path());
    git_commit(dir.path(), "Initial commit");

    // Modify and stage (but don't commit)
    create_file(dir.path(), "main.rs", "fn main() { println!(\"staged\"); }");
    git_add_all(dir.path());

    let git_diff = GitDiff::discover(dir.path()).unwrap();
    let staged = git_diff.get_staged_files().unwrap();

    let main_path = dir.path().join("main.rs").canonicalize().unwrap();
    assert!(
        staged
            .iter()
            .any(|p| p.canonicalize().ok() == Some(main_path.clone())),
        "Modified staged file should be detected"
    );
}

#[test]
fn changed_files_skips_unchanged_subtrees() {
    let dir = create_git_repo();

    // Create multiple directories with files
    std::fs::create_dir_all(dir.path().join("unchanged/deep")).unwrap();
    std::fs::create_dir_all(dir.path().join("changed")).unwrap();
    create_file(dir.path(), "unchanged/a.rs", "fn a() {}");
    create_file(dir.path(), "unchanged/deep/b.rs", "fn b() {}");
    create_file(dir.path(), "changed/c.rs", "fn c() {}");
    git_add_all(dir.path());
    git_commit(dir.path(), "Initial commit");

    // Only modify file in 'changed' directory
    create_file(dir.path(), "changed/c.rs", "fn c() { /* modified */ }");
    git_add_all(dir.path());
    git_commit(dir.path(), "Modify only changed/c.rs");

    let git_diff = GitDiff::discover(dir.path()).unwrap();
    let changed = git_diff.get_changed_files("HEAD~1").unwrap();

    // Only the modified file should be in the result
    assert_eq!(changed.len(), 1, "Expected exactly 1 changed file");
    let has_c = changed
        .iter()
        .any(|p| p.file_name().is_some_and(|name| name == "c.rs"));
    assert!(has_c, "Expected c.rs to be in changed files");

    // Files in unchanged directory should not be included
    let has_unchanged = changed
        .iter()
        .any(|p| p.to_string_lossy().contains("unchanged"));
    assert!(!has_unchanged, "Unchanged subtree should be skipped");
}
