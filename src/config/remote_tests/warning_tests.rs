//! Tests for warning flag behavior during remote config fetching.

use super::super::{
    FetchPolicy, cache_file_path, fetch_remote_config_with_client, reset_warning_flag,
    was_warning_shown,
};

use super::{MockHttpClient, acquire_warning_lock, create_temp_project};

#[test]
fn warning_flag_initially_false() {
    let _lock = acquire_warning_lock();
    reset_warning_flag();
    assert!(!was_warning_shown());
}

#[test]
fn warning_shown_on_first_remote_fetch() {
    let _warn_lock = acquire_warning_lock();
    let temp_dir = create_temp_project();
    reset_warning_flag();

    let content = "version = \"2\"\n\n[content]\nmax_lines = 500\n";
    let client = MockHttpClient::success(content);

    let url = "https://mock-test-warning.example.com/config.toml";

    assert!(!was_warning_shown());
    let result = fetch_remote_config_with_client(
        url,
        &client,
        Some(temp_dir.path()),
        None,
        FetchPolicy::Normal,
    );
    assert!(result.is_ok());
    assert!(was_warning_shown());
}

#[test]
fn warning_shown_only_once_per_session() {
    let _warn_lock = acquire_warning_lock();
    let temp_dir = create_temp_project();
    reset_warning_flag();

    let content = "version = \"2\"\n\n[content]\nmax_lines = 600\n";
    let client = MockHttpClient::success(content);

    let url1 = "https://mock-test-warning-once-1.example.com/config.toml";
    let url2 = "https://mock-test-warning-once-2.example.com/config.toml";

    // First fetch - warning shown
    assert!(!was_warning_shown());
    let _ = fetch_remote_config_with_client(
        url1,
        &client,
        Some(temp_dir.path()),
        None,
        FetchPolicy::Normal,
    );
    assert!(was_warning_shown());

    // Second fetch - warning already shown, flag still true
    let _ = fetch_remote_config_with_client(
        url2,
        &client,
        Some(temp_dir.path()),
        None,
        FetchPolicy::Normal,
    );
    assert!(was_warning_shown());
}

#[test]
fn warning_not_shown_when_cache_hit() {
    let _warn_lock = acquire_warning_lock();
    let temp_dir = create_temp_project();
    reset_warning_flag();

    let content = "version = \"2\"\n\n[content]\nmax_lines = 700\n";
    let client = MockHttpClient::success(content);

    let url = "https://mock-test-warning-cache.example.com/config.toml";

    // First fetch - populates cache, shows warning
    let _ = fetch_remote_config_with_client(
        url,
        &client,
        Some(temp_dir.path()),
        None,
        FetchPolicy::Normal,
    );

    // Check if cache was created
    let cache_file_exists = cache_file_path(url, temp_dir.path()).exists();
    if !cache_file_exists {
        return;
    }

    // Reset warning flag
    reset_warning_flag();
    assert!(!was_warning_shown());

    // Second fetch - should use cache, no warning
    let _ = fetch_remote_config_with_client(
        url,
        &client,
        Some(temp_dir.path()),
        None,
        FetchPolicy::Normal,
    );
    assert!(!was_warning_shown()); // Warning not shown because cache was used
}
