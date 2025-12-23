#![allow(dead_code)]

use std::fmt::Write;
use std::fs;
use std::path::Path;

use tempfile::TempDir;

/// Creates an `assert_cmd` Command for the sloc-guard binary.
#[macro_export]
macro_rules! sloc_guard {
    () => {
        assert_cmd::Command::new(assert_cmd::cargo::cargo_bin!("sloc-guard"))
    };
}

/// Creates a temporary directory with test fixtures for integration tests.
pub struct TestFixture {
    pub dir: TempDir,
}

impl TestFixture {
    /// Creates a new test fixture with an empty temp directory.
    pub fn new() -> Self {
        Self {
            dir: TempDir::new().expect("Failed to create temp directory"),
        }
    }

    /// Creates a file with the given content in the temp directory.
    pub fn create_file(&self, relative_path: &str, content: &str) {
        let path = self.dir.path().join(relative_path);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("Failed to create parent directories");
        }
        fs::write(&path, content).expect("Failed to write file");
    }

    /// Creates a directory in the temp directory.
    pub fn create_dir(&self, relative_path: &str) {
        let path = self.dir.path().join(relative_path);
        fs::create_dir_all(&path).expect("Failed to create directory");
    }

    /// Returns the path to the temp directory.
    pub fn path(&self) -> &Path {
        self.dir.path()
    }

    /// Creates a basic sloc-guard config file.
    pub fn create_config(&self, content: &str) {
        self.create_file(".sloc-guard.toml", content);
    }

    /// Creates a simple Rust file with the given number of code lines.
    pub fn create_rust_file(&self, relative_path: &str, code_lines: usize) {
        let mut content = String::new();
        for i in 0..code_lines {
            let _ = writeln!(content, "let var_{i} = {i};");
        }
        self.create_file(relative_path, &content);
    }

    /// Creates a Rust file with comments and blank lines.
    pub fn create_rust_file_with_comments(
        &self,
        relative_path: &str,
        code_lines: usize,
        comment_lines: usize,
        blank_lines: usize,
    ) {
        let mut content = String::new();

        for i in 0..comment_lines {
            let _ = writeln!(content, "// Comment line {i}");
        }

        for _ in 0..blank_lines {
            content.push('\n');
        }

        for i in 0..code_lines {
            let _ = writeln!(content, "let var_{i} = {i};");
        }

        self.create_file(relative_path, &content);
    }
}

impl Default for TestFixture {
    fn default() -> Self {
        Self::new()
    }
}

/// Basic V2 config for testing.
pub const BASIC_CONFIG_V2: &str = r#"
version = "2"

[scanner]
gitignore = false
exclude = []

[content]
extensions = ["rs"]
max_lines = 100
warn_threshold = 0.8
skip_comments = true
skip_blank = true

[structure]
max_files = 10
max_dirs = 5
warn_threshold = 0.9
"#;

/// Config with low limits to trigger failures.
pub const STRICT_CONFIG_V2: &str = r#"
version = "2"

[scanner]
gitignore = false
exclude = []

[content]
extensions = ["rs"]
max_lines = 10
warn_threshold = 0.5
skip_comments = true
skip_blank = true

[structure]
max_files = 2
max_dirs = 1
warn_threshold = 0.8
"#;

/// Config with content rules.
pub const CONFIG_WITH_RULES: &str = r#"
version = "2"

[scanner]
gitignore = false
exclude = []

[content]
extensions = ["rs", "py"]
max_lines = 100
warn_threshold = 0.8
skip_comments = true
skip_blank = true

[[content.rules]]
pattern = "tests/**"
max_lines = 500

[[content.rules]]
pattern = "src/generated/**"
max_lines = 1000

[structure]
max_files = 50
max_dirs = 10
"#;

/// Config with structure rules.
pub const CONFIG_WITH_STRUCTURE_RULES: &str = r#"
version = "2"

[scanner]
gitignore = false
exclude = []

[content]
extensions = ["rs"]
max_lines = 200
skip_comments = true
skip_blank = true

[structure]
max_files = 10
max_dirs = 5

[[structure.rules]]
scope = "src/components/*"
max_files = 3
max_dirs = 0

[[structure.rules]]
scope = "src/generated"
max_files = 100
max_dirs = -1
"#;
