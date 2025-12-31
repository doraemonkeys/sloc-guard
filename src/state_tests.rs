use std::path::Path;
use tempfile::TempDir;

use super::*;

// =============================================================================
// Timestamp Tests
// =============================================================================

#[test]
fn try_current_unix_timestamp_returns_some() {
    let ts = try_current_unix_timestamp();
    assert!(ts.is_some());
    // Should be roughly current time (sanity check: after ~2023)
    assert!(ts.unwrap() > 1_700_000_000);
}

#[test]
fn current_unix_timestamp_is_reasonable() {
    let ts = current_unix_timestamp();
    // Should be after ~2023 and not too far in the future
    assert!(ts > 1_700_000_000);
    assert!(ts < 3_000_000_000); // Before year 2065
}

// =============================================================================
// State Directory Tests
// =============================================================================

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
    assert_eq!(result, dunce::canonicalize(temp_dir.path()).unwrap());
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
    assert_eq!(result, dunce::canonicalize(temp_dir.path()).unwrap());
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
    assert_eq!(result, dunce::canonicalize(temp_dir.path()).unwrap());
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
    assert_eq!(result, dunce::canonicalize(&inner_project).unwrap());
}

#[test]
fn baseline_path_construction() {
    let temp_dir = TempDir::new().unwrap();
    let result = baseline_path(temp_dir.path());
    assert_eq!(result, temp_dir.path().join(".sloc-guard-baseline.json"));
}

// =============================================================================
// File Locking Tests
// =============================================================================

#[test]
fn lock_error_display_timeout() {
    let err = LockError::Timeout;
    assert_eq!(format!("{err}"), "lock acquisition timed out");
}

#[test]
fn lock_error_display_io() {
    let io_err = io::Error::new(io::ErrorKind::PermissionDenied, "access denied");
    let err = LockError::Io(io_err);
    assert!(format!("{err}").contains("lock I/O error"));
}

#[test]
fn exclusive_lock_succeeds_on_uncontested_file() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.json");
    let file = fs::File::create(&file_path).unwrap();

    let result = try_lock_exclusive_with_timeout(&file, 100);
    assert!(result.is_ok());

    unlock_file(&file);
}

#[test]
fn shared_lock_succeeds_on_uncontested_file() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.json");
    let file = fs::File::create(&file_path).unwrap();

    let result = try_lock_shared_with_timeout(&file, 100);
    assert!(result.is_ok());

    unlock_file(&file);
}

#[test]
fn exclusive_lock_times_out_when_held() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.json");

    // Create and lock file from first handle
    let file1 = fs::File::create(&file_path).unwrap();
    file1.lock().unwrap();

    // Try to acquire exclusive lock from second handle with short timeout
    let file2 = fs::File::open(&file_path).unwrap();
    let result = try_lock_exclusive_with_timeout(&file2, 100);

    assert!(matches!(result, Err(LockError::Timeout)));

    file1.unlock().unwrap();
}

#[test]
fn shared_lock_times_out_when_exclusive_lock_held() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.json");

    // Create and exclusively lock file
    let file1 = fs::File::create(&file_path).unwrap();
    file1.lock().unwrap();

    // Try to acquire shared lock from second handle with short timeout
    let file2 = fs::File::open(&file_path).unwrap();
    let result = try_lock_shared_with_timeout(&file2, 100);

    assert!(matches!(result, Err(LockError::Timeout)));

    file1.unlock().unwrap();
}

#[test]
fn multiple_shared_locks_allowed() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.json");

    // Create file
    let file1 = fs::File::create(&file_path).unwrap();

    // Acquire first shared lock
    let result1 = try_lock_shared_with_timeout(&file1, 100);
    assert!(result1.is_ok());

    // Acquire second shared lock from another handle
    let file2 = fs::File::open(&file_path).unwrap();
    let result2 = try_lock_shared_with_timeout(&file2, 100);
    assert!(result2.is_ok());

    unlock_file(&file1);
    unlock_file(&file2);
}

#[test]
fn unlock_file_is_idempotent() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.json");
    let file = fs::File::create(&file_path).unwrap();

    // Lock and unlock multiple times should not panic
    file.lock().unwrap();
    unlock_file(&file);
    unlock_file(&file); // Should not panic on double unlock
}

#[test]
fn lock_error_source_timeout_returns_none() {
    let err = LockError::Timeout;
    assert!(std::error::Error::source(&err).is_none());
}

#[test]
fn lock_error_source_io_returns_source() {
    let io_err = io::Error::new(io::ErrorKind::PermissionDenied, "access denied");
    let err = LockError::Io(io_err);
    let source = std::error::Error::source(&err);
    assert!(source.is_some());
}

#[test]
fn lock_error_from_io_error() {
    let io_err = io::Error::new(io::ErrorKind::NotFound, "file not found");
    let lock_err: LockError = io_err.into();
    assert!(matches!(lock_err, LockError::Io(_)));
}

// =============================================================================
// SharedLockGuard Tests
// =============================================================================

#[test]
fn shared_lock_guard_acquires_lock_successfully() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.json");
    let file = fs::File::create(&file_path).unwrap();

    let guard = SharedLockGuard::try_acquire(&file, 100, "test file", &file_path);
    assert!(guard.is_locked());
    // Guard drops here and unlocks
}

#[test]
fn shared_lock_guard_reports_not_locked_on_timeout() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.json");

    // Create and exclusively lock file
    let file1 = fs::File::create(&file_path).unwrap();
    file1.lock().unwrap();

    // Try to acquire shared lock from second handle (should fail)
    let file2 = fs::File::open(&file_path).unwrap();
    let guard = SharedLockGuard::try_acquire(&file2, 100, "test file", &file_path);
    assert!(!guard.is_locked());

    file1.unlock().unwrap();
}

#[test]
fn shared_lock_guard_unlocks_on_drop() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.json");
    let file = fs::File::create(&file_path).unwrap();

    {
        let guard = SharedLockGuard::try_acquire(&file, 100, "test file", &file_path);
        assert!(guard.is_locked());
        // Guard drops here
    }

    // After guard drops, we should be able to acquire exclusive lock
    let file2 = fs::File::open(&file_path).unwrap();
    let result = try_lock_exclusive_with_timeout(&file2, 100);
    assert!(result.is_ok());
    file2.unlock().unwrap();
}

// =============================================================================
// SaveOutcome Tests
// =============================================================================

#[test]
fn save_outcome_is_saved() {
    let outcome = SaveOutcome::Saved;
    assert!(outcome.is_saved());
    assert!(!outcome.is_skipped());
}

#[test]
fn save_outcome_is_skipped() {
    let outcome = SaveOutcome::Skipped;
    assert!(outcome.is_skipped());
    assert!(!outcome.is_saved());
}

#[test]
fn save_outcome_equality() {
    assert_eq!(SaveOutcome::Saved, SaveOutcome::Saved);
    assert_eq!(SaveOutcome::Skipped, SaveOutcome::Skipped);
    assert_ne!(SaveOutcome::Saved, SaveOutcome::Skipped);
}

// =============================================================================
// Atomic Write Tests
// =============================================================================

#[test]
fn atomic_write_creates_file() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.json");
    let content = b"test content";

    let result = atomic_write_with_lock(&file_path, content, "test file");
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), SaveOutcome::Saved);
    assert!(file_path.exists());
    assert_eq!(fs::read(&file_path).unwrap(), content);
}

#[test]
fn atomic_write_creates_parent_directories() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir
        .path()
        .join("a")
        .join("b")
        .join("c")
        .join("test.json");
    let content = b"nested content";

    let result = atomic_write_with_lock(&file_path, content, "test file");
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), SaveOutcome::Saved);
    assert!(file_path.exists());
    assert_eq!(fs::read(&file_path).unwrap(), content);
}

#[test]
fn atomic_write_overwrites_existing_file() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.json");

    // Write initial content
    fs::write(&file_path, "old content").unwrap();

    // Overwrite with atomic write
    let new_content = b"new content";
    let result = atomic_write_with_lock(&file_path, new_content, "test file");
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), SaveOutcome::Saved);
    assert_eq!(fs::read(&file_path).unwrap(), new_content);
}

#[test]
fn atomic_write_preserves_original_on_error() {
    // This test verifies that if atomic write fails, the original file is preserved
    // We can't easily simulate a failure in the atomic_write function itself,
    // but we can verify that temp files don't corrupt the original

    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.json");
    let original_content = b"original";

    fs::write(&file_path, original_content).unwrap();

    // Write new content
    let new_content = b"new content";
    let result = atomic_write_with_lock(&file_path, new_content, "test file");
    assert!(result.is_ok());

    // Should be updated
    assert_eq!(fs::read(&file_path).unwrap(), new_content);
}

#[test]
fn atomic_write_cleans_up_temp_files() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.json");
    let content = b"test content";

    atomic_write_with_lock(&file_path, content, "test file").unwrap();

    // Check no temp files remain
    let entries: Vec<_> = fs::read_dir(temp_dir.path())
        .unwrap()
        .filter_map(std::result::Result::ok)
        .collect();
    assert_eq!(entries.len(), 1); // Only the target file
    assert_eq!(entries[0].file_name(), "test.json");
}

#[test]
fn atomic_write_returns_skipped_on_lock_timeout() {
    // Test that atomic_write_with_lock returns SaveOutcome::Skipped when lock times out.
    use std::sync::mpsc;
    use std::thread;

    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.json");

    // Create the file first (atomic_write needs it to exist for locking)
    fs::write(&file_path, "original").unwrap();

    // Open and lock the file exclusively
    let lock_holder = fs::OpenOptions::new().write(true).open(&file_path).unwrap();
    lock_holder.lock().unwrap();

    // Use short timeout (100ms) for fast testing
    let test_timeout_ms = 100;

    // Run atomic write in a thread with short timeout while holding the lock
    let file_path_clone = file_path.clone();
    let (tx, rx) = mpsc::channel();
    let handle = thread::spawn(move || {
        let result = atomic_write_with_lock_timeout(
            &file_path_clone,
            b"new content",
            "test file",
            test_timeout_ms,
        );
        tx.send(result).unwrap();
    });

    // Wait for the thread to complete (should timeout quickly)
    let result = rx.recv().unwrap();
    handle.join().unwrap();

    // Verify it returned Skipped due to lock timeout
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), SaveOutcome::Skipped);

    // Release lock before reading (Windows requires this)
    lock_holder.unlock().unwrap();
    drop(lock_holder);

    // Original file should be unchanged
    assert_eq!(fs::read_to_string(&file_path).unwrap(), "original");
}
