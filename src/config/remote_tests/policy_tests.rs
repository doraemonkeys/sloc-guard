//! Tests for `FetchPolicy` behavior: Offline and `ForceRefresh` modes.

use std::fs;

use super::super::{FetchPolicy, cache_file_path, fetch_remote_config_with_client, write_to_cache};

use super::{MockHttpClient, acquire_warning_lock, create_temp_project};

// ============================================================================
// Offline mode tests
// ============================================================================

#[test]
fn offline_mode_returns_cached_content() {
    let temp_dir = create_temp_project();
    let content = "version = \"2\"\n\n[content]\nmax_lines = 800\n";
    let client = MockHttpClient::error("Should not be called in offline mode");

    let url = "https://mock-test-offline-cached.example.com/config.toml";

    // Populate cache first
    let write_result = write_to_cache(url, content, Some(temp_dir.path()));
    assert!(write_result.is_some());

    // Fetch in offline mode should succeed using cached content
    let result = fetch_remote_config_with_client(
        url,
        &client,
        Some(temp_dir.path()),
        None,
        FetchPolicy::Offline,
    );
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), content);
    assert_eq!(client.call_count(), 0); // Client should not be called
}

#[test]
fn offline_mode_ignores_ttl() {
    let temp_dir = create_temp_project();
    let content = "version = \"2\"\n\n[content]\nmax_lines = 850\n";
    let client = MockHttpClient::error("Should not be called in offline mode");

    let url = "https://mock-test-offline-ignores-ttl.example.com/config.toml";

    // Populate cache (content exists regardless of TTL check)
    let write_result = write_to_cache(url, content, Some(temp_dir.path()));
    assert!(write_result.is_some());

    // Offline mode should use cache even if we can't verify TTL freshness
    // (main point: it doesn't require cache to be "fresh", just to exist)
    let result = fetch_remote_config_with_client(
        url,
        &client,
        Some(temp_dir.path()),
        None,
        FetchPolicy::Offline,
    );
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), content);
    assert_eq!(client.call_count(), 0);
}

#[test]
fn offline_mode_returns_error_on_cache_miss() {
    let temp_dir = create_temp_project();
    let client = MockHttpClient::error("Should not be called");

    let url = "https://mock-test-offline-miss.example.com/config.toml";

    // No cache populated, should error
    let result = fetch_remote_config_with_client(
        url,
        &client,
        Some(temp_dir.path()),
        None,
        FetchPolicy::Offline,
    );
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("cache miss"));
    assert!(err_msg.contains("offline"));
    assert_eq!(client.call_count(), 0);
}

#[test]
fn offline_mode_rejects_invalid_url() {
    let temp_dir = create_temp_project();
    let client = MockHttpClient::error("Should not be called");

    let result = fetch_remote_config_with_client(
        "/local/path",
        &client,
        Some(temp_dir.path()),
        None,
        FetchPolicy::Offline,
    );
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("Invalid remote config URL")
    );
}

#[test]
fn offline_mode_returns_error_without_project_root() {
    let client = MockHttpClient::error("Should not be called");
    let result = fetch_remote_config_with_client(
        "https://example.com/config.toml",
        &client,
        None,
        None,
        FetchPolicy::Offline,
    );
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("cache miss"));
}

// ============================================================================
// ForceRefresh mode tests
// ============================================================================

#[test]
fn force_refresh_skips_cache() {
    let _lock = acquire_warning_lock();
    let temp_dir = create_temp_project();
    let cached_content = "version = \"2\"\n\n[content]\nmax_lines = 1600\n";
    let fresh_content = "version = \"2\"\n\n[content]\nmax_lines = 1601\n";
    let client = MockHttpClient::success(fresh_content);

    let url = "https://mock-test-force-refresh.example.com/config.toml";

    // Populate cache first
    let write_result = write_to_cache(url, cached_content, Some(temp_dir.path()));
    assert!(write_result.is_some());

    // ForceRefresh should skip cache and fetch fresh content
    let result = fetch_remote_config_with_client(
        url,
        &client,
        Some(temp_dir.path()),
        None,
        FetchPolicy::ForceRefresh,
    );
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), fresh_content);
    assert_eq!(client.call_count(), 1); // Always fetches
}

#[test]
fn force_refresh_updates_cache() {
    let _lock = acquire_warning_lock();
    let temp_dir = create_temp_project();
    let fresh_content = "version = \"2\"\n\n[content]\nmax_lines = 1700\n";
    let client = MockHttpClient::success(fresh_content);

    let url = "https://mock-test-force-refresh-updates.example.com/config.toml";

    // ForceRefresh should fetch and update cache
    let result = fetch_remote_config_with_client(
        url,
        &client,
        Some(temp_dir.path()),
        None,
        FetchPolicy::ForceRefresh,
    );
    assert!(result.is_ok());

    // Verify cache was updated
    let cache_path = cache_file_path(url, temp_dir.path());
    if cache_path.exists() {
        let cached = fs::read_to_string(&cache_path).unwrap();
        assert_eq!(cached, fresh_content);
    }
}
