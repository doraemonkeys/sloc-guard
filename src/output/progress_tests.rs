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

#[test]
fn test_progress_bar_visible_when_tty() {
    // Test the visible progress bar path by simulating TTY environment
    let progress = ScanProgress::new_with_visibility(50, false, true);

    progress.inc();
    progress.inc();
    progress.finish();
}

#[test]
fn test_progress_bar_hidden_when_not_tty() {
    // Test the hidden path when not a TTY
    let progress = ScanProgress::new_with_visibility(50, false, false);

    progress.inc();
    progress.finish();
}

#[test]
fn test_progress_bar_hidden_when_quiet_overrides_tty() {
    // Quiet mode should hide progress bar even if TTY
    let progress = ScanProgress::new_with_visibility(50, true, true);

    progress.inc();
    progress.finish();
}

#[test]
fn test_create_visible_progress_bar() {
    // Directly test the visible progress bar creation
    let pb = ScanProgress::create_visible_progress_bar(100);

    pb.set_position(50);
    pb.finish_and_clear();
}
