mod cache_tests;
mod fetch_tests;
mod hash_verification_tests;
mod policy_tests;
mod url_tests;
mod warning_tests;

use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

use tempfile::TempDir;

use crate::error::{Result, SlocGuardError};

use super::HttpClient;

/// Create a temporary project root for testing cache.
/// Each test gets its own isolated directory via `TempDir`.
/// The directory is automatically cleaned up when `TempDir` is dropped.
pub fn create_temp_project() -> TempDir {
    tempfile::tempdir().expect("Failed to create temp directory")
}

/// Mock HTTP client for testing remote config fetching.
pub struct MockHttpClient {
    success_content: Option<String>,
    error_message: Option<String>,
    call_count: AtomicUsize,
}

impl MockHttpClient {
    pub fn success(content: &str) -> Self {
        Self {
            success_content: Some(content.to_string()),
            error_message: None,
            call_count: AtomicUsize::new(0),
        }
    }

    pub fn error(msg: &str) -> Self {
        Self {
            success_content: None,
            error_message: Some(msg.to_string()),
            call_count: AtomicUsize::new(0),
        }
    }

    pub fn call_count(&self) -> usize {
        self.call_count.load(Ordering::SeqCst)
    }
}

impl HttpClient for MockHttpClient {
    fn get(&self, _url: &str) -> Result<String> {
        self.call_count.fetch_add(1, Ordering::SeqCst);
        self.success_content.as_ref().map_or_else(
            || {
                let msg = self
                    .error_message
                    .as_ref()
                    .map_or("No response configured", String::as_str);
                Err(SlocGuardError::Config(msg.to_string()))
            },
            |content| Ok(content.clone()),
        )
    }
}

/// Global lock for warning flag tests to prevent test interference.
pub static WARNING_LOCK: Mutex<()> = Mutex::new(());

/// Acquire warning lock, recovering from poisoned state.
pub fn acquire_warning_lock() -> std::sync::MutexGuard<'static, ()> {
    WARNING_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}
