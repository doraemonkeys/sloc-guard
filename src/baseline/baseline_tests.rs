use std::fs::{self, File};
use std::io::Write;
use std::thread;
use std::time::Duration;

use tempfile::TempDir;

use super::*;

#[test]
fn new_creates_empty_baseline() {
    let baseline = Baseline::new();
    assert!(baseline.is_empty());
    assert_eq!(baseline.len(), 0);
    assert_eq!(baseline.version(), 2);
}

#[test]
fn set_content_and_get_entry() {
    let mut baseline = Baseline::new();
    let hash = "abc123".to_string();
    baseline.set_content("src/main.rs", 100, hash.clone());

    let entry = baseline.get("src/main.rs").unwrap();
    match entry {
        BaselineEntry::Content { lines, hash: h } => {
            assert_eq!(*lines, 100);
            assert_eq!(h, &hash);
        }
        BaselineEntry::Structure { .. } => panic!("Expected Content entry"),
    }
}

#[test]
fn set_structure_and_get_entry() {
    let mut baseline = Baseline::new();
    baseline.set_structure("src/components", StructureViolationType::Files, 25);

    let entry = baseline.get("src/components").unwrap();
    match entry {
        BaselineEntry::Structure {
            violation_type,
            count,
        } => {
            assert_eq!(*violation_type, StructureViolationType::Files);
            assert_eq!(*count, 25);
        }
        BaselineEntry::Content { .. } => panic!("Expected Structure entry"),
    }
}

#[test]
fn get_nonexistent_entry_returns_none() {
    let baseline = Baseline::new();
    assert!(baseline.get("nonexistent.rs").is_none());
}

#[test]
fn contains_returns_correct_result() {
    let mut baseline = Baseline::new();
    baseline.set_content("src/main.rs", 100, "hash".to_string());

    assert!(baseline.contains("src/main.rs"));
    assert!(!baseline.contains("src/lib.rs"));
}

#[test]
fn remove_entry() {
    let mut baseline = Baseline::new();
    baseline.set_content("src/main.rs", 100, "hash".to_string());

    let removed = baseline.remove("src/main.rs");
    assert!(removed.is_some());
    assert!(!baseline.contains("src/main.rs"));
}

#[test]
fn remove_nonexistent_returns_none() {
    let mut baseline = Baseline::new();
    assert!(baseline.remove("nonexistent.rs").is_none());
}

#[test]
fn len_and_is_empty() {
    let mut baseline = Baseline::new();
    assert!(baseline.is_empty());
    assert_eq!(baseline.len(), 0);

    baseline.set_content("src/main.rs", 100, "hash1".to_string());
    assert!(!baseline.is_empty());
    assert_eq!(baseline.len(), 1);

    baseline.set_content("src/lib.rs", 200, "hash2".to_string());
    assert_eq!(baseline.len(), 2);
}

#[test]
fn files_returns_all_entries() {
    let mut baseline = Baseline::new();
    baseline.set_content("src/main.rs", 100, "hash1".to_string());
    baseline.set_content("src/lib.rs", 200, "hash2".to_string());

    let files = baseline.files();
    assert_eq!(files.len(), 2);
    assert!(files.contains_key("src/main.rs"));
    assert!(files.contains_key("src/lib.rs"));
}

#[test]
fn save_and_load_baseline() {
    let temp = TempDir::new().unwrap();
    let path = temp.path().join(".sloc-guard-baseline.json");

    let mut baseline = Baseline::new();
    baseline.set_content("src/main.rs", 100, "abc123".to_string());
    baseline.set_content("src/lib.rs", 200, "def456".to_string());

    baseline.save(&path).unwrap();
    assert!(path.exists());

    let loaded = Baseline::load(&path).unwrap();
    assert_eq!(loaded.len(), 2);

    match loaded.get("src/main.rs").unwrap() {
        BaselineEntry::Content { lines, .. } => assert_eq!(*lines, 100),
        BaselineEntry::Structure { .. } => panic!("Expected Content entry"),
    }
    match loaded.get("src/lib.rs").unwrap() {
        BaselineEntry::Content { lines, .. } => assert_eq!(*lines, 200),
        BaselineEntry::Structure { .. } => panic!("Expected Content entry"),
    }
}

#[test]
fn load_nonexistent_file_returns_error() {
    let result = Baseline::load(std::path::Path::new("nonexistent.json"));
    assert!(result.is_err());
}

#[test]
fn compute_content_hash_produces_consistent_result() {
    let content = "fn main() {}";
    let hash1 = compute_content_hash(content);
    let hash2 = compute_content_hash(content);
    assert_eq!(hash1, hash2);
    assert_eq!(hash1.len(), 64);
}

#[test]
fn compute_content_hash_different_content_different_hash() {
    let hash1 = compute_content_hash("fn main() {}");
    let hash2 = compute_content_hash("fn main() { println!(\"hello\"); }");
    assert_ne!(hash1, hash2);
}

#[test]
fn compute_file_hash_works() {
    let temp = TempDir::new().unwrap();
    let path = temp.path().join("test.rs");
    fs::write(&path, "fn main() {}").unwrap();

    let hash = compute_file_hash(&path).unwrap();
    assert_eq!(hash.len(), 64);

    let content_hash = compute_content_hash("fn main() {}");
    assert_eq!(hash, content_hash);
}

#[test]
fn compute_file_hash_nonexistent_returns_error() {
    let result = compute_file_hash(std::path::Path::new("nonexistent.rs"));
    assert!(result.is_err());
}

#[test]
fn baseline_entry_content_constructor() {
    let entry = BaselineEntry::content(100, "hash123".to_string());
    assert!(entry.is_content());
    assert!(!entry.is_structure());
    match entry {
        BaselineEntry::Content { lines, hash } => {
            assert_eq!(lines, 100);
            assert_eq!(hash, "hash123");
        }
        BaselineEntry::Structure { .. } => panic!("Expected Content entry"),
    }
}

#[test]
fn baseline_entry_structure_constructor() {
    let entry = BaselineEntry::structure(StructureViolationType::Dirs, 10);
    assert!(entry.is_structure());
    assert!(!entry.is_content());
    match entry {
        BaselineEntry::Structure {
            violation_type,
            count,
        } => {
            assert_eq!(violation_type, StructureViolationType::Dirs);
            assert_eq!(count, 10);
        }
        BaselineEntry::Content { .. } => panic!("Expected Structure entry"),
    }
}

#[test]
fn default_creates_new_baseline() {
    let baseline = Baseline::default();
    assert!(baseline.is_empty());
    assert_eq!(baseline.version(), 2);
}

#[test]
fn set_content_updates_existing_entry() {
    let mut baseline = Baseline::new();
    baseline.set_content("src/main.rs", 100, "hash1".to_string());
    baseline.set_content("src/main.rs", 150, "hash2".to_string());

    let entry = baseline.get("src/main.rs").unwrap();
    match entry {
        BaselineEntry::Content { lines, hash } => {
            assert_eq!(*lines, 150);
            assert_eq!(hash, "hash2");
        }
        BaselineEntry::Structure { .. } => panic!("Expected Content entry"),
    }
}

#[test]
fn saved_json_is_readable() {
    let temp = TempDir::new().unwrap();
    let path = temp.path().join("baseline.json");

    let mut baseline = Baseline::new();
    baseline.set_content("src/main.rs", 100, "abc123".to_string());
    baseline.save(&path).unwrap();

    let json = fs::read_to_string(&path).unwrap();
    assert!(json.contains("\"version\":"));
    assert!(json.contains("\"files\":"));
    assert!(json.contains("src/main.rs"));
}

#[test]
fn mixed_content_and_structure_entries() {
    let temp = TempDir::new().unwrap();
    let path = temp.path().join("baseline.json");

    let mut baseline = Baseline::new();
    baseline.set_content("src/large_file.rs", 500, "hash1".to_string());
    baseline.set_structure("src/components", StructureViolationType::Files, 30);
    baseline.set_structure("src/utils", StructureViolationType::Dirs, 15);

    baseline.save(&path).unwrap();

    let loaded = Baseline::load(&path).unwrap();
    assert_eq!(loaded.len(), 3);

    assert!(loaded.get("src/large_file.rs").unwrap().is_content());
    assert!(loaded.get("src/components").unwrap().is_structure());
    assert!(loaded.get("src/utils").unwrap().is_structure());
}

#[test]
fn set_generic_entry() {
    let mut baseline = Baseline::new();

    let content_entry = BaselineEntry::content(100, "hash".to_string());
    baseline.set("src/file.rs", content_entry);

    let structure_entry = BaselineEntry::structure(StructureViolationType::Files, 20);
    baseline.set("src/dir", structure_entry);

    assert_eq!(baseline.len(), 2);
    assert!(baseline.get("src/file.rs").unwrap().is_content());
    assert!(baseline.get("src/dir").unwrap().is_structure());
}

// =============================================================================
// Lock Behavior Tests
// =============================================================================

#[test]
fn load_succeeds_with_shared_lock_held() {
    // Test that load works when file is already read-locked (shared lock should succeed)
    let temp = TempDir::new().unwrap();
    let path = temp.path().join("baseline.json");

    // Create baseline file
    let mut baseline = Baseline::new();
    baseline.set_content("src/main.rs", 100, "hash".to_string());
    let _ = baseline.save(&path);

    // Hold a shared lock on the file
    let file = File::open(&path).unwrap();
    file.try_lock_shared().unwrap();

    // Load should still succeed (shared locks are compatible)
    let loaded = Baseline::load(&path).unwrap();
    assert_eq!(loaded.len(), 1);

    file.unlock().unwrap();
}

#[test]
fn load_succeeds_with_warning_when_exclusive_lock_held() {
    // Test that load still works (with warning) when lock acquisition fails
    // because another process holds an exclusive lock
    use std::sync::mpsc;

    let temp = TempDir::new().unwrap();
    let path = temp.path().join("baseline.json");

    // Create baseline file
    let mut baseline = Baseline::new();
    baseline.set_content("src/main.rs", 100, "hash".to_string());
    let _ = baseline.save(&path);

    // Use channel to synchronize lock acquisition
    let (tx, rx) = mpsc::channel();

    // Hold an exclusive lock on the file in another thread
    let path_clone = path.clone();
    let handle = thread::spawn(move || {
        let file = File::open(&path_clone).unwrap();
        file.try_lock().unwrap(); // Exclusive lock
        tx.send(()).unwrap(); // Signal that lock is acquired
        // Hold the lock for a bit
        thread::sleep(Duration::from_millis(200));
        file.unlock().unwrap();
    });

    // Wait for the lock to be acquired (no race condition)
    rx.recv().unwrap();

    // Load should still succeed (continues without lock after timeout)
    // Note: On some platforms this might succeed if the lock times out quickly
    let loaded = Baseline::load(&path);
    assert!(loaded.is_ok());

    handle.join().unwrap();
}

#[test]
fn save_returns_skipped_when_lock_unavailable() {
    use crate::state::SaveOutcome;
    use std::sync::mpsc;

    // Test that save returns SaveOutcome::Skipped when lock cannot be acquired
    let temp = TempDir::new().unwrap();
    let path = temp.path().join("baseline.json");

    // Create initial baseline file
    {
        let mut file = File::create(&path).unwrap();
        file.write_all(b"{}").unwrap();
    }

    // Hold an exclusive lock on the file (blocking lock, guaranteed to acquire)
    let lock_file = File::open(&path).unwrap();
    lock_file.lock().unwrap();

    // Try to save from another baseline instance with a short timeout
    let mut baseline = Baseline::new();
    baseline.set_content("src/main.rs", 100, "hash".to_string());

    // Use a thread to avoid any timing issues with the lock
    let path_clone = path.clone();
    let (tx, rx) = mpsc::channel();
    let handle = thread::spawn(move || {
        // Use short timeout (100ms) for deterministic test behavior
        let result = baseline.save_with_timeout(&path_clone, 100);
        tx.send(result).unwrap();
    });

    // Wait for the thread to complete (should timeout quickly)
    let result = rx.recv().unwrap();
    handle.join().unwrap();

    // Verify it returned Skipped due to lock timeout
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), SaveOutcome::Skipped);

    // Release lock before cleanup (Windows requires this)
    lock_file.unlock().unwrap();
    drop(lock_file);

    // Original file should be unchanged
    assert_eq!(fs::read_to_string(&path).unwrap(), "{}");
}
