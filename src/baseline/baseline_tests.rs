use std::fs;

use tempfile::TempDir;

use super::*;

#[test]
fn new_creates_empty_baseline() {
    let baseline = Baseline::new();
    assert!(baseline.is_empty());
    assert_eq!(baseline.len(), 0);
    assert_eq!(baseline.version(), 1);
}

#[test]
fn set_and_get_entry() {
    let mut baseline = Baseline::new();
    let hash = "abc123".to_string();
    baseline.set("src/main.rs", 100, hash.clone());

    let entry = baseline.get("src/main.rs").unwrap();
    assert_eq!(entry.lines, 100);
    assert_eq!(entry.hash, hash);
}

#[test]
fn get_nonexistent_entry_returns_none() {
    let baseline = Baseline::new();
    assert!(baseline.get("nonexistent.rs").is_none());
}

#[test]
fn contains_returns_correct_result() {
    let mut baseline = Baseline::new();
    baseline.set("src/main.rs", 100, "hash".to_string());

    assert!(baseline.contains("src/main.rs"));
    assert!(!baseline.contains("src/lib.rs"));
}

#[test]
fn remove_entry() {
    let mut baseline = Baseline::new();
    baseline.set("src/main.rs", 100, "hash".to_string());

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

    baseline.set("src/main.rs", 100, "hash1".to_string());
    assert!(!baseline.is_empty());
    assert_eq!(baseline.len(), 1);

    baseline.set("src/lib.rs", 200, "hash2".to_string());
    assert_eq!(baseline.len(), 2);
}

#[test]
fn files_returns_all_entries() {
    let mut baseline = Baseline::new();
    baseline.set("src/main.rs", 100, "hash1".to_string());
    baseline.set("src/lib.rs", 200, "hash2".to_string());

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
    baseline.set("src/main.rs", 100, "abc123".to_string());
    baseline.set("src/lib.rs", 200, "def456".to_string());

    baseline.save(&path).unwrap();
    assert!(path.exists());

    let loaded = Baseline::load(&path).unwrap();
    assert_eq!(loaded.len(), 2);
    assert_eq!(loaded.get("src/main.rs").unwrap().lines, 100);
    assert_eq!(loaded.get("src/lib.rs").unwrap().lines, 200);
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
fn baseline_entry_new() {
    let entry = BaselineEntry::new(100, "hash123".to_string());
    assert_eq!(entry.lines, 100);
    assert_eq!(entry.hash, "hash123");
}

#[test]
fn default_creates_new_baseline() {
    let baseline = Baseline::default();
    assert!(baseline.is_empty());
    assert_eq!(baseline.version(), 1);
}

#[test]
fn set_updates_existing_entry() {
    let mut baseline = Baseline::new();
    baseline.set("src/main.rs", 100, "hash1".to_string());
    baseline.set("src/main.rs", 150, "hash2".to_string());

    let entry = baseline.get("src/main.rs").unwrap();
    assert_eq!(entry.lines, 150);
    assert_eq!(entry.hash, "hash2");
}

#[test]
fn saved_json_is_readable() {
    let temp = TempDir::new().unwrap();
    let path = temp.path().join("baseline.json");

    let mut baseline = Baseline::new();
    baseline.set("src/main.rs", 100, "abc123".to_string());
    baseline.save(&path).unwrap();

    let json = fs::read_to_string(&path).unwrap();
    assert!(json.contains("\"version\":"));
    assert!(json.contains("\"files\":"));
    assert!(json.contains("src/main.rs"));
}
