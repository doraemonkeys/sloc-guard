use std::path::Path;

use super::super::FileFilter;

pub struct AcceptAllFilter;

impl FileFilter for AcceptAllFilter {
    fn should_include(&self, _path: &Path) -> bool {
        true
    }
}

pub struct RustOnlyFilter;

impl FileFilter for RustOnlyFilter {
    fn should_include(&self, path: &Path) -> bool {
        path.extension().is_some_and(|ext| ext == "rs")
    }
}

pub fn init_git_repo(dir: &Path) {
    std::process::Command::new("git")
        .args(["init"])
        .current_dir(dir)
        .output()
        .expect("Failed to init git repo");

    std::process::Command::new("git")
        .args(["config", "user.email", "test@test.com"])
        .current_dir(dir)
        .output()
        .expect("Failed to set git email");

    std::process::Command::new("git")
        .args(["config", "user.name", "Test"])
        .current_dir(dir)
        .output()
        .expect("Failed to set git name");
}
