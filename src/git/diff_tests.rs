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
