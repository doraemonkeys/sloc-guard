use crate::error::{Result, SlocGuardError};
use toml::Value;

/// Available preset names.
pub const AVAILABLE_PRESETS: &[&str] = &[
    "rust-strict",
    "node-strict",
    "python-strict",
    "go-strict",
    "monorepo-base",
];

/// Load a built-in preset by name.
///
/// # Errors
/// Returns an error if the preset name is unknown.
pub fn load_preset(name: &str) -> Result<Value> {
    let content = match name {
        "rust-strict" => PRESET_RUST_STRICT,
        "node-strict" => PRESET_NODE_STRICT,
        "python-strict" => PRESET_PYTHON_STRICT,
        "go-strict" => PRESET_GO_STRICT,
        "monorepo-base" => PRESET_MONOREPO_BASE,
        _ => {
            return Err(SlocGuardError::Config(format!(
                "Unknown preset: '{}'. Available presets: {}",
                name,
                AVAILABLE_PRESETS.join(", ")
            )));
        }
    };

    toml::from_str(content)
        .map_err(|e| SlocGuardError::Config(format!("Failed to parse preset '{name}': {e}")))
}

const PRESET_RUST_STRICT: &str = r#"
version = "2"

[scanner]
exclude = [".git/**", "target/**", "vendor/**", "*.generated.rs", "benches/**"]

[content]
extensions = ["rs"]
max_lines = 600
warn_threshold = 0.85
skip_comments = true
skip_blank = true

# Test files often need more space for fixtures and setup
[[content.rules]]
pattern = "**/*_test.rs"
max_lines = 1000
reason = "Test files need more space for fixtures and assertions"

[[content.rules]]
pattern = "**/*_tests.rs"
max_lines = 1000
reason = "Test files need more space for fixtures and assertions"

[[content.rules]]
pattern = "**/tests/**/*.rs"
max_lines = 1000
reason = "Integration test files need more space"

[[content.rules]]
pattern = "**/benches/**/*.rs"
max_lines = 1500
reason = "Benchmark files may contain large datasets"

[[content.rules]]
pattern = "**/examples/**/*.rs"
max_lines = 800
reason = "Example files may be more verbose for clarity"

[structure]
max_files = 20
max_dirs = 10
warn_threshold = 0.9
deny_files = ["*.bak", "*.tmp", ".DS_Store", "Thumbs.db"]
deny_extensions = [".exe", ".dll", ".so", ".dylib"]

# Relax limits for test directories
[[structure.rules]]
scope = "tests/**"
max_files = 50
max_dirs = 15
reason = "Test directories often have more files"
"#;

const PRESET_NODE_STRICT: &str = r#"
version = "2"

[scanner]
exclude = [
    ".git/**", "node_modules/**", "dist/**", "build/**", ".next/**",
    "coverage/**", ".nuxt/**", ".output/**", ".cache/**", ".parcel-cache/**"
]

[content]
extensions = ["js", "jsx", "ts", "tsx", "mjs", "cjs", "vue", "svelte"]
max_lines = 600
warn_threshold = 0.85
skip_comments = true
skip_blank = true

# Test files need more space for fixtures and mocks
[[content.rules]]
pattern = "**/*.test.{js,jsx,ts,tsx}"
max_lines = 1000
reason = "Test files need more space for fixtures and assertions"

[[content.rules]]
pattern = "**/*.spec.{js,jsx,ts,tsx}"
max_lines = 1000
reason = "Test files need more space for fixtures and assertions"

[[content.rules]]
pattern = "**/__tests__/**/*.{js,jsx,ts,tsx}"
max_lines = 1000
reason = "Test files need more space for fixtures and assertions"

[[content.rules]]
pattern = "**/test/**/*.{js,jsx,ts,tsx}"
max_lines = 1000
reason = "Test files need more space for fixtures and assertions"

[[content.rules]]
pattern = "**/stories/**/*.{js,jsx,ts,tsx}"
max_lines = 800
reason = "Storybook stories may include multiple variants"

[[content.rules]]
pattern = "**/*.stories.{js,jsx,ts,tsx}"
max_lines = 800
reason = "Storybook stories may include multiple variants"

[[content.rules]]
pattern = "**/e2e/**/*.{js,ts}"
max_lines = 800
reason = "E2E tests may have longer flows"

[structure]
max_files = 25
max_dirs = 15
warn_threshold = 0.9
deny_files = ["*.bak", "*.tmp", ".DS_Store", "Thumbs.db", "npm-debug.log*", "yarn-error.log"]
deny_extensions = [".exe", ".dll"]

# Relax limits for test directories
[[structure.rules]]
scope = "__tests__/**"
max_files = 50
max_dirs = 20
reason = "Test directories often have more files"

[[structure.rules]]
scope = "test/**"
max_files = 50
max_dirs = 20
reason = "Test directories often have more files"

[[structure.rules]]
scope = "tests/**"
max_files = 50
max_dirs = 20
reason = "Test directories often have more files"

# Component directories may have many files
[[structure.rules]]
scope = "src/components/**"
max_files = 40
reason = "UI component directories may have many related files"
"#;

const PRESET_PYTHON_STRICT: &str = r#"
version = "2"

[scanner]
exclude = [
    ".git/**", "__pycache__/**", ".venv/**", "venv/**", "env/**",
    ".tox/**", "*.egg-info/**", ".pytest_cache/**", ".mypy_cache/**",
    ".ruff_cache/**", "htmlcov/**", ".coverage", "dist/**", "build/**"
]

[content]
extensions = ["py", "pyi"]
max_lines = 600
warn_threshold = 0.85
skip_comments = true
skip_blank = true

# Test files need more space for fixtures
[[content.rules]]
pattern = "**/test_*.py"
max_lines = 1000
reason = "Test files need more space for fixtures and assertions"

[[content.rules]]
pattern = "**/*_test.py"
max_lines = 1000
reason = "Test files need more space for fixtures and assertions"

[[content.rules]]
pattern = "**/tests/**/*.py"
max_lines = 1000
reason = "Test files need more space for fixtures and assertions"

[[content.rules]]
pattern = "**/conftest.py"
max_lines = 800
reason = "Conftest files contain shared fixtures"

[[content.rules]]
pattern = "**/migrations/**/*.py"
max_lines = 1500
reason = "Database migrations may be auto-generated and verbose"

[structure]
max_files = 20
max_dirs = 10
warn_threshold = 0.9
deny_files = ["*.bak", "*.tmp", ".DS_Store", "Thumbs.db", "*.pyc"]
deny_extensions = [".exe", ".dll", ".so"]
deny_dirs = ["__pycache__"]

# Relax limits for test directories
[[structure.rules]]
scope = "tests/**"
max_files = 50
max_dirs = 20
reason = "Test directories often have more files"

[[structure.rules]]
scope = "test/**"
max_files = 50
max_dirs = 20
reason = "Test directories often have more files"
"#;

const PRESET_GO_STRICT: &str = r#"
version = "2"

[scanner]
exclude = [
    ".git/**", "vendor/**", "bin/**", "dist/**",
    "testdata/**", ".idea/**", ".vscode/**"
]

[content]
extensions = ["go"]
max_lines = 600
warn_threshold = 0.85
skip_comments = true
skip_blank = true

# Test files need more space for table-driven tests
[[content.rules]]
pattern = "**/*_test.go"
max_lines = 1000
reason = "Test files need more space for table-driven tests and fixtures"

# Example files may be more verbose for clarity
[[content.rules]]
pattern = "**/examples/**/*.go"
max_lines = 800
reason = "Example files may be more verbose for clarity"

[[content.rules]]
pattern = "**/example/**/*.go"
max_lines = 800
reason = "Example files may be more verbose for clarity"

# Main entry points in cmd directory
[[content.rules]]
pattern = "**/cmd/**/*.go"
max_lines = 400
reason = "Command entry points should be concise"

# Internal packages may have more complex logic
[[content.rules]]
pattern = "**/internal/**/*.go"
max_lines = 700
reason = "Internal packages may contain more complex implementations"

# Generated code (protobuf, mock, etc.)
[[content.rules]]
pattern = "**/*.pb.go"
max_lines = 5000
reason = "Generated protobuf files are auto-generated"

[[content.rules]]
pattern = "**/*_mock.go"
max_lines = 2000
reason = "Generated mock files are auto-generated"

[[content.rules]]
pattern = "**/mock_*.go"
max_lines = 2000
reason = "Generated mock files are auto-generated"

[structure]
max_files = 20
max_dirs = 10
warn_threshold = 0.9
deny_files = ["*.bak", "*.tmp", ".DS_Store", "Thumbs.db"]
deny_extensions = [".exe", ".dll", ".so", ".dylib"]

# Relax limits for test directories
[[structure.rules]]
scope = "test/**"
max_files = 50
max_dirs = 15
reason = "Test directories often have more files"

[[structure.rules]]
scope = "testdata/**"
max_files = 100
max_dirs = 30
reason = "Test data directories may contain many fixture files"

# Internal packages may have more subdirectories
[[structure.rules]]
scope = "internal/**"
max_files = 30
max_dirs = 20
reason = "Internal packages may have deeper structure"

# Command directories for multi-binary projects
[[structure.rules]]
scope = "cmd/**"
max_files = 15
max_dirs = 10
reason = "Each command should be relatively small"
"#;

const PRESET_MONOREPO_BASE: &str = r#"
version = "2"

[scanner]
exclude = [
    ".git/**",
    # Rust
    "target/**", "vendor/**",
    # Node.js
    "node_modules/**", "dist/**", "build/**", ".next/**", ".nuxt/**",
    # Python
    "__pycache__/**", ".venv/**", "venv/**", "*.egg-info/**", ".pytest_cache/**",
    # Go
    "vendor/**",
    # General
    "coverage/**", ".cache/**"
]

[content]
extensions = ["rs", "js", "jsx", "ts", "tsx", "py", "go", "java", "kt", "swift", "vue", "svelte"]
max_lines = 600
warn_threshold = 0.85
skip_comments = true
skip_blank = true

# Rust tests
[[content.rules]]
pattern = "**/*_test.rs"
max_lines = 1000
reason = "Test files need more space"

[[content.rules]]
pattern = "**/*_tests.rs"
max_lines = 1000
reason = "Test files need more space"

# Node tests
[[content.rules]]
pattern = "**/*.test.{js,jsx,ts,tsx}"
max_lines = 1000
reason = "Test files need more space"

[[content.rules]]
pattern = "**/*.spec.{js,jsx,ts,tsx}"
max_lines = 1000
reason = "Test files need more space"

[[content.rules]]
pattern = "**/__tests__/**/*.{js,jsx,ts,tsx}"
max_lines = 1000
reason = "Test files need more space"

# Python tests
[[content.rules]]
pattern = "**/test_*.py"
max_lines = 1000
reason = "Test files need more space"

[[content.rules]]
pattern = "**/*_test.py"
max_lines = 1000
reason = "Test files need more space"

# Go tests
[[content.rules]]
pattern = "**/*_test.go"
max_lines = 1000
reason = "Test files need more space"

# Java/Kotlin tests
[[content.rules]]
pattern = "**/src/test/**/*.{java,kt}"
max_lines = 1000
reason = "Test files need more space"

[structure]
max_files = 30
max_dirs = 20
warn_threshold = 0.9
deny_files = ["*.bak", "*.tmp", ".DS_Store", "Thumbs.db"]
deny_extensions = [".exe", ".dll", ".so", ".dylib"]

# Test directories across all languages
[[structure.rules]]
scope = "tests/**"
max_files = 50
max_dirs = 25
reason = "Test directories often have more files"

[[structure.rules]]
scope = "test/**"
max_files = 50
max_dirs = 25
reason = "Test directories often have more files"

[[structure.rules]]
scope = "__tests__/**"
max_files = 50
max_dirs = 25
reason = "Test directories often have more files"

[[structure.rules]]
scope = "**/src/test/**"
max_files = 50
max_dirs = 25
reason = "Java/Kotlin test source directories"
"#;

#[cfg(test)]
#[path = "presets_tests.rs"]
mod tests;
