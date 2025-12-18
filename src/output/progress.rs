use std::io::IsTerminal;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use indicatif::{ProgressBar, ProgressStyle};

/// Progress bar for file scanning operations.
///
/// The progress bar is automatically disabled in quiet mode or when stdout is not a TTY.
#[derive(Clone)]
pub struct ScanProgress {
    progress_bar: ProgressBar,
    counter: Arc<AtomicU64>,
}

impl ScanProgress {
    /// Creates a new progress bar for scanning files.
    ///
    /// # Arguments
    /// * `total` - Total number of files to scan
    /// * `quiet` - If true, progress bar is disabled
    ///
    /// The progress bar outputs to stderr to avoid interfering with stdout output.
    ///
    /// # Panics
    ///
    /// This function will panic if the progress bar template is invalid.
    /// The template is a compile-time constant, so this should never happen.
    #[must_use]
    pub fn new(total: u64, quiet: bool) -> Self {
        let progress_bar = if quiet || !std::io::stderr().is_terminal() {
            ProgressBar::hidden()
        } else {
            let pb = ProgressBar::new(total);
            pb.set_style(
                ProgressStyle::default_bar()
                    .template("{spinner:.green} Scanning [{bar:40.cyan/blue}] {pos}/{len} files ({percent}%)")
                    // SAFETY: Template is a static string with valid format specifiers
                    .expect("valid template")
                    .progress_chars("█▓░"),
            );
            pb
        };

        Self {
            progress_bar,
            counter: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Increments the progress counter by 1.
    ///
    /// Thread-safe for use with rayon parallel iterators.
    pub fn inc(&self) {
        let count = self.counter.fetch_add(1, Ordering::Relaxed) + 1;
        self.progress_bar.set_position(count);
    }

    /// Finishes the progress bar and clears it from the terminal.
    pub fn finish(&self) {
        self.progress_bar.finish_and_clear();
    }
}

#[cfg(test)]
#[path = "progress_tests.rs"]
mod tests;
