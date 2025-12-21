use std::path::Path;
use tempfile::TempDir;

use super::*;

#[test]
fn detect_state_dir_outside_git_returns_fallback() {
    // Create a temporary directory that is NOT a git repo
    let temp_dir = TempDir::new().unwrap();
    let result = detect_state_dir(temp_dir.path());
    assert_eq!(result, temp_dir.path().join(".sloc-guard"));
}

#[test]
fn detect_state_dir_in_git_repo_returns_git_path() {
    // Create a temporary git repo (just needs .git directory)
    let temp_dir = TempDir::new().unwrap();
    let git_dir = temp_dir.path().join(".git");
    fs::create_dir(&git_dir).unwrap();

    let result = detect_state_dir(temp_dir.path());
    assert_eq!(result, git_dir.join("sloc-guard"));
}

#[test]
fn cache_path_construction() {
    let temp_dir = TempDir::new().unwrap();
    let result = cache_path(temp_dir.path());
    // Should be fallback since not a git repo
    assert_eq!(
        result,
        temp_dir.path().join(".sloc-guard").join("cache.json")
    );
}

#[test]
fn history_path_construction() {
    let temp_dir = TempDir::new().unwrap();
    let result = history_path(temp_dir.path());
    // Should be fallback since not a git repo
    assert_eq!(
        result,
        temp_dir.path().join(".sloc-guard").join("history.json")
    );
}

#[test]
fn cache_path_in_git_repo() {
    let temp_dir = TempDir::new().unwrap();
    let git_dir = temp_dir.path().join(".git");
    fs::create_dir(&git_dir).unwrap();

    let result = cache_path(temp_dir.path());
    assert_eq!(result, git_dir.join("sloc-guard").join("cache.json"));
}

#[test]
fn history_path_in_git_repo() {
    let temp_dir = TempDir::new().unwrap();
    let git_dir = temp_dir.path().join(".git");
    fs::create_dir(&git_dir).unwrap();

    let result = history_path(temp_dir.path());
    assert_eq!(result, git_dir.join("sloc-guard").join("history.json"));
}

#[test]
fn ensure_parent_dir_creates_nested_directory() {
    let temp_dir = TempDir::new().unwrap();
    let nested_path = temp_dir
        .path()
        .join("a")
        .join("b")
        .join("c")
        .join("file.json");

    ensure_parent_dir(&nested_path).unwrap();

    assert!(nested_path.parent().unwrap().exists());
}

#[test]
fn ensure_parent_dir_succeeds_when_exists() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("existing_dir").join("file.json");
    fs::create_dir(temp_dir.path().join("existing_dir")).unwrap();

    // Should succeed without error
    ensure_parent_dir(&file_path).unwrap();
}

#[test]
fn ensure_parent_dir_handles_root_path() {
    // Path with no parent should succeed
    let result = ensure_parent_dir(Path::new("file.json"));
    assert!(result.is_ok());
}

#[test]
fn discover_project_root_finds_git_directory() {
    let temp_dir = TempDir::new().unwrap();
    let git_dir = temp_dir.path().join(".git");
    fs::create_dir(&git_dir).unwrap();

    // Create a subdirectory
    let sub_dir = temp_dir.path().join("src").join("lib");
    fs::create_dir_all(&sub_dir).unwrap();

    let result = discover_project_root(&sub_dir);
    assert_eq!(result, fs::canonicalize(temp_dir.path()).unwrap());
}

#[test]
fn discover_project_root_finds_config_file() {
    let temp_dir = TempDir::new().unwrap();
    let config_file = temp_dir.path().join(".sloc-guard.toml");
    fs::write(&config_file, "").unwrap();

    // Create a subdirectory
    let sub_dir = temp_dir.path().join("src");
    fs::create_dir(&sub_dir).unwrap();

    let result = discover_project_root(&sub_dir);
    assert_eq!(result, fs::canonicalize(temp_dir.path()).unwrap());
}

#[test]
fn discover_project_root_prefers_git_over_config() {
    let temp_dir = TempDir::new().unwrap();
    let git_dir = temp_dir.path().join(".git");
    fs::create_dir(&git_dir).unwrap();
    let config_file = temp_dir.path().join(".sloc-guard.toml");
    fs::write(&config_file, "").unwrap();

    let sub_dir = temp_dir.path().join("src");
    fs::create_dir(&sub_dir).unwrap();

    // Both markers exist, should find .git first
    let result = discover_project_root(&sub_dir);
    assert_eq!(result, fs::canonicalize(temp_dir.path()).unwrap());
}

#[test]
fn discover_project_root_stops_at_first_marker() {
    // Test that discovery stops at the first marker found, not the outermost
    let temp_dir = TempDir::new().unwrap();

    // Create outer project with .git
    let git_dir = temp_dir.path().join(".git");
    fs::create_dir(&git_dir).unwrap();

    // Create inner project with only .sloc-guard.toml (no .git)
    let inner_project = temp_dir.path().join("packages").join("inner");
    fs::create_dir_all(&inner_project).unwrap();
    fs::write(inner_project.join(".sloc-guard.toml"), "").unwrap();

    // Create a subdirectory in inner project
    let sub_dir = inner_project.join("src");
    fs::create_dir(&sub_dir).unwrap();

    let result = discover_project_root(&sub_dir);
    // Should stop at inner project (config marker), not outer project (git marker)
    assert_eq!(result, fs::canonicalize(&inner_project).unwrap());
}

#[test]
fn baseline_path_construction() {
    let temp_dir = TempDir::new().unwrap();
    let result = baseline_path(temp_dir.path());
    assert_eq!(result, temp_dir.path().join(".sloc-guard-baseline.json"));
}
