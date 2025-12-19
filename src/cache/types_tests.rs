use super::*;
use crate::counter::LineStats;
use tempfile::NamedTempFile;

#[test]
fn test_cache_entry_new() {
    let stats = LineStats {
        total: 100,
        code: 80,
        comment: 15,
        blank: 5,
        ignored: 0,
    };
    let entry = CacheEntry::new("abc123".to_string(), &stats, 1000, 512);

    assert_eq!(entry.hash, "abc123");
    assert_eq!(entry.stats.total, 100);
    assert_eq!(entry.stats.code, 80);
    assert_eq!(entry.stats.comment, 15);
    assert_eq!(entry.stats.blank, 5);
    assert_eq!(entry.mtime, 1000);
    assert_eq!(entry.size, 512);
}

#[test]
fn test_cached_line_stats_from() {
    let stats = LineStats {
        total: 50,
        code: 40,
        comment: 5,
        blank: 5,
        ignored: 0,
    };
    let cached = CachedLineStats::from(&stats);

    assert_eq!(cached.total, 50);
    assert_eq!(cached.code, 40);
    assert_eq!(cached.comment, 5);
    assert_eq!(cached.blank, 5);
}

#[test]
fn test_line_stats_from_cached() {
    let cached = CachedLineStats {
        total: 50,
        code: 40,
        comment: 5,
        blank: 5,
        ignored: 0,
    };
    let stats = LineStats::from(&cached);

    assert_eq!(stats.total, 50);
    assert_eq!(stats.code, 40);
    assert_eq!(stats.comment, 5);
    assert_eq!(stats.blank, 5);
}

#[test]
fn test_cache_new() {
    let cache = Cache::new("config_hash_123".to_string());

    assert_eq!(cache.version(), 3);
    assert_eq!(cache.config_hash(), "config_hash_123");
    assert!(cache.is_empty());
}

#[test]
fn test_cache_default() {
    let cache = Cache::default();

    assert_eq!(cache.version(), 3);
    assert_eq!(cache.config_hash(), "");
    assert!(cache.is_empty());
}

#[test]
fn test_cache_set_and_get() {
    let mut cache = Cache::new("hash".to_string());
    let stats = LineStats {
        total: 100,
        code: 80,
        comment: 15,
        blank: 5,
        ignored: 0,
    };

    cache.set("src/main.rs", "file_hash".to_string(), &stats, 1000, 512);

    assert_eq!(cache.len(), 1);
    assert!(!cache.is_empty());

    let entry = cache.get("src/main.rs").unwrap();
    assert_eq!(entry.hash, "file_hash");
    assert_eq!(entry.stats.code, 80);
}

#[test]
fn test_cache_get_if_valid() {
    let mut cache = Cache::new("hash".to_string());
    let stats = LineStats {
        total: 100,
        code: 80,
        comment: 15,
        blank: 5,
        ignored: 0,
    };

    cache.set("src/main.rs", "file_hash".to_string(), &stats, 1000, 512);

    // Valid hash
    let entry = cache.get_if_valid("src/main.rs", "file_hash");
    assert!(entry.is_some());

    // Invalid hash
    let entry = cache.get_if_valid("src/main.rs", "different_hash");
    assert!(entry.is_none());

    // Non-existent file
    let entry = cache.get_if_valid("src/other.rs", "file_hash");
    assert!(entry.is_none());
}

#[test]
fn test_cache_remove() {
    let mut cache = Cache::new("hash".to_string());
    let stats = LineStats::default();

    cache.set("src/main.rs", "hash1".to_string(), &stats, 1000, 512);
    assert_eq!(cache.len(), 1);

    let removed = cache.remove("src/main.rs");
    assert!(removed.is_some());
    assert_eq!(cache.len(), 0);

    let removed = cache.remove("nonexistent");
    assert!(removed.is_none());
}

#[test]
fn test_cache_is_valid() {
    let cache = Cache::new("config_hash".to_string());

    assert!(cache.is_valid("config_hash"));
    assert!(!cache.is_valid("different_hash"));
}

#[test]
fn test_cache_save_and_load() {
    let mut cache = Cache::new("config_hash_abc".to_string());
    let stats = LineStats {
        total: 100,
        code: 80,
        comment: 15,
        blank: 5,
        ignored: 0,
    };
    cache.set(
        "src/main.rs",
        "file_hash_xyz".to_string(),
        &stats,
        2000,
        1024,
    );

    let temp_file = NamedTempFile::new().unwrap();
    let path = temp_file.path().to_path_buf();

    // Save
    cache.save(&path).unwrap();

    // Load
    let loaded = Cache::load(&path).unwrap();

    assert_eq!(loaded.version(), cache.version());
    assert_eq!(loaded.config_hash(), cache.config_hash());
    assert_eq!(loaded.len(), cache.len());

    let entry = loaded.get("src/main.rs").unwrap();
    assert_eq!(entry.hash, "file_hash_xyz");
    assert_eq!(entry.stats.code, 80);
}

#[test]
fn test_cache_load_nonexistent() {
    let result = Cache::load(std::path::Path::new("/nonexistent/cache.json"));
    assert!(result.is_err());
}

#[test]
fn test_cache_load_invalid_json() {
    let temp_file = NamedTempFile::new().unwrap();
    std::fs::write(temp_file.path(), b"invalid json").unwrap();

    let result = Cache::load(temp_file.path());
    assert!(result.is_err());
}

#[test]
fn test_compute_config_hash() {
    let config1 = Config::default();
    let config2 = Config::default();

    let hash1 = compute_config_hash(&config1);
    let hash2 = compute_config_hash(&config2);

    // Same config produces same hash
    assert_eq!(hash1, hash2);
    // Hash is non-empty
    assert!(!hash1.is_empty());
    // Hash is hex-encoded SHA-256 (64 chars)
    assert_eq!(hash1.len(), 64);
}

#[test]
fn test_compute_config_hash_different_configs() {
    let config1 = Config::default();
    let mut config2 = Config::default();
    config2.content.max_lines = 1000;

    let hash1 = compute_config_hash(&config1);
    let hash2 = compute_config_hash(&config2);

    assert_ne!(hash1, hash2);
}

#[test]
fn test_cache_files() {
    let mut cache = Cache::new("hash".to_string());
    let stats = LineStats::default();

    cache.set("file1.rs", "h1".to_string(), &stats, 1000, 100);
    cache.set("file2.rs", "h2".to_string(), &stats, 2000, 200);

    let files = cache.files();
    assert_eq!(files.len(), 2);
    assert!(files.contains_key("file1.rs"));
    assert!(files.contains_key("file2.rs"));
}

#[test]
fn test_cache_entry_metadata_matches() {
    let stats = LineStats::default();
    let entry = CacheEntry::new("hash".to_string(), &stats, 1000, 512);

    assert!(entry.metadata_matches(1000, 512));
    assert!(!entry.metadata_matches(1001, 512));
    assert!(!entry.metadata_matches(1000, 513));
    assert!(!entry.metadata_matches(0, 0));
}

#[test]
fn test_cache_get_if_metadata_matches() {
    let mut cache = Cache::new("hash".to_string());
    let stats = LineStats::default();

    cache.set("src/main.rs", "file_hash".to_string(), &stats, 1000, 512);

    // Matching metadata
    let entry = cache.get_if_metadata_matches("src/main.rs", 1000, 512);
    assert!(entry.is_some());
    assert_eq!(entry.unwrap().hash, "file_hash");

    // Different mtime
    let entry = cache.get_if_metadata_matches("src/main.rs", 1001, 512);
    assert!(entry.is_none());

    // Different size
    let entry = cache.get_if_metadata_matches("src/main.rs", 1000, 513);
    assert!(entry.is_none());

    // Non-existent file
    let entry = cache.get_if_metadata_matches("src/other.rs", 1000, 512);
    assert!(entry.is_none());
}
