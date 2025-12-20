use crate::error::{Result, SlocGuardError};
use toml::Value;

/// Available preset names.
pub const AVAILABLE_PRESETS: &[&str] = &[
    "rust-strict",
    "node-strict",
    "python-strict",
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
        "monorepo-base" => PRESET_MONOREPO_BASE,
        _ => {
            return Err(SlocGuardError::Config(format!(
                "Unknown preset: '{}'. Available presets: {}",
                name,
                AVAILABLE_PRESETS.join(", ")
            )))
        }
    };

    toml::from_str(content)
        .map_err(|e| SlocGuardError::Config(format!("Failed to parse preset '{name}': {e}")))
}

const PRESET_RUST_STRICT: &str = r#"
version = "2"

[scanner]
exclude = ["target/**", "vendor/**", "*.generated.rs"]

[content]
extensions = ["rs"]
max_lines = 500
warn_threshold = 0.85
skip_comments = true
skip_blank = true

[structure]
max_files = 20
max_dirs = 10
warn_threshold = 0.9
"#;

const PRESET_NODE_STRICT: &str = r#"
version = "2"

[scanner]
exclude = ["node_modules/**", "dist/**", "build/**", ".next/**", "coverage/**"]

[content]
extensions = ["js", "jsx", "ts", "tsx", "mjs", "cjs"]
max_lines = 400
warn_threshold = 0.85
skip_comments = true
skip_blank = true

[structure]
max_files = 25
max_dirs = 15
warn_threshold = 0.9
"#;

const PRESET_PYTHON_STRICT: &str = r#"
version = "2"

[scanner]
exclude = ["__pycache__/**", ".venv/**", "venv/**", ".tox/**", "*.egg-info/**", ".pytest_cache/**"]

[content]
extensions = ["py", "pyi"]
max_lines = 400
warn_threshold = 0.85
skip_comments = true
skip_blank = true

[structure]
max_files = 20
max_dirs = 10
warn_threshold = 0.9
"#;

const PRESET_MONOREPO_BASE: &str = r#"
version = "2"

[scanner]
exclude = ["node_modules/**", "target/**", "dist/**", "build/**", "vendor/**", ".git/**"]

[content]
extensions = ["rs", "js", "jsx", "ts", "tsx", "py", "go", "java", "kt", "swift"]
max_lines = 600
warn_threshold = 0.85
skip_comments = true
skip_blank = true

[structure]
max_files = 30
max_dirs = 20
warn_threshold = 0.9
"#;

#[cfg(test)]
#[path = "presets_tests.rs"]
mod tests;
