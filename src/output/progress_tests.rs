use super::*;

#[test]
fn test_progress_bar_hidden_in_quiet_mode() {
    let progress = ScanProgress::new(100, true);
    progress.inc();
    progress.inc();
    progress.finish();
}

#[test]
fn test_progress_bar_increment() {
    let progress = ScanProgress::new(10, true);

    for _ in 0..10 {
        progress.inc();
    }

    progress.finish();
}

#[test]
fn test_progress_bar_clone() {
    let progress = ScanProgress::new(100, true);
    let cloned = progress.clone();

    progress.inc();
    cloned.inc();

    progress.finish();
}
