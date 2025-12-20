use std::fmt::Write;
use std::path::{Path, PathBuf};

use crate::{Result, SlocGuardError};

/// Represents a detected project type with its configuration defaults.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProjectType {
    Rust,
    Node,
    Go,
    Python,
    Java,
    CSharp,
    Unknown,
}

impl ProjectType {
    /// Returns file extensions relevant for this project type.
    #[must_use]
    pub fn extensions(&self) -> Vec<&'static str> {
        match self {
            Self::Rust => vec!["rs"],
            Self::Node => vec!["ts", "tsx", "js", "jsx", "mjs", "cjs"],
            Self::Go => vec!["go"],
            Self::Python => vec!["py", "pyi"],
            Self::Java => vec!["java", "kt"],
            Self::CSharp => vec!["cs"],
            Self::Unknown => vec!["rs", "go", "py", "js", "ts", "c", "cpp"],
        }
    }

    /// Returns recommended `max_lines` for this project type.
    #[must_use]
    pub const fn default_max_lines(&self) -> usize {
        match self {
            Self::Rust => 800,
            Self::Node => 400,
            Self::Go | Self::CSharp => 600,
            Self::Python | Self::Java | Self::Unknown => 500,
        }
    }

    /// Returns typical exclude patterns for this project type.
    #[must_use]
    pub fn exclude_patterns(&self) -> Vec<&'static str> {
        match self {
            Self::Rust => vec!["**/target/**"],
            Self::Node => vec!["**/node_modules/**", "**/dist/**", "**/build/**"],
            Self::Go => vec!["**/vendor/**"],
            Self::Python => vec![
                "**/__pycache__/**",
                "**/.venv/**",
                "**/venv/**",
                "**/.tox/**",
            ],
            Self::Java => vec!["**/target/**", "**/build/**"],
            Self::CSharp => vec!["**/bin/**", "**/obj/**"],
            Self::Unknown => vec![],
        }
    }

    /// Returns the project type name for display.
    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Rust => "Rust",
            Self::Node => "Node.js/TypeScript",
            Self::Go => "Go",
            Self::Python => "Python",
            Self::Java => "Java/Kotlin",
            Self::CSharp => "C#/.NET",
            Self::Unknown => "Unknown",
        }
    }
}

/// A detected project with its location and type.
#[derive(Debug, Clone)]
pub struct DetectedProject {
    /// Relative path from scan root (empty string for root project).
    pub path: String,
    /// Detected project type.
    pub project_type: ProjectType,
}

impl DetectedProject {
    /// Returns true if this is the root project (not a subdirectory).
    #[must_use]
    pub const fn is_root(&self) -> bool {
        self.path.is_empty()
    }
}

/// Result of project detection scan.
#[derive(Debug, Clone, Default)]
pub struct DetectionResult {
    /// Root project type (if detected).
    pub root: Option<ProjectType>,
    /// Detected subprojects (for monorepos).
    pub subprojects: Vec<DetectedProject>,
    /// Whether this appears to be a monorepo.
    pub is_monorepo: bool,
}

impl DetectionResult {
    /// Returns the effective project type for config generation.
    #[must_use]
    pub fn effective_type(&self) -> ProjectType {
        if let Some(ref root) = self.root {
            return root.clone();
        }
        if !self.subprojects.is_empty() {
            return ProjectType::Unknown;
        }
        ProjectType::Unknown
    }
}

/// Trait for detecting project types (for testability).
#[allow(clippy::missing_errors_doc)]
pub trait ProjectDetector {
    /// Check if a file exists at the given path.
    fn exists(&self, path: &Path) -> bool;

    /// List immediate subdirectories of a path.
    fn list_subdirs(&self, path: &Path) -> std::io::Result<Vec<PathBuf>>;

    /// List files in a directory (for C# detection).
    fn list_files(&self, path: &Path) -> std::io::Result<Vec<String>>;
}

/// Real filesystem implementation for project detection.
#[derive(Debug, Default)]
pub struct RealProjectDetector;

impl ProjectDetector for RealProjectDetector {
    fn exists(&self, path: &Path) -> bool {
        path.exists()
    }

    fn list_subdirs(&self, path: &Path) -> std::io::Result<Vec<PathBuf>> {
        let mut subdirs = Vec::new();
        for entry in std::fs::read_dir(path)? {
            let entry = entry?;
            if entry.file_type()?.is_dir() {
                subdirs.push(entry.path());
            }
        }
        Ok(subdirs)
    }

    fn list_files(&self, path: &Path) -> std::io::Result<Vec<String>> {
        let mut files = Vec::new();
        for entry in std::fs::read_dir(path)? {
            let entry = entry?;
            if entry.file_type()?.is_file() && let Some(name) = entry.file_name().to_str() {
                files.push(name.to_string());
            }
        }
        Ok(files)
    }
}

const RUST_MARKERS: &[&str] = &["Cargo.toml"];
const NODE_MARKERS: &[&str] = &["package.json"];
const GO_MARKERS: &[&str] = &["go.mod"];
const PYTHON_MARKERS: &[&str] = &["pyproject.toml", "setup.py", "requirements.txt", "Pipfile"];
const JAVA_MARKERS: &[&str] = &["pom.xml", "build.gradle", "build.gradle.kts"];

/// Detect project type in a directory.
fn detect_project_type<D: ProjectDetector>(detector: &D, dir: &Path) -> Option<ProjectType> {
    for marker in RUST_MARKERS {
        if detector.exists(&dir.join(marker)) {
            return Some(ProjectType::Rust);
        }
    }
    for marker in GO_MARKERS {
        if detector.exists(&dir.join(marker)) {
            return Some(ProjectType::Go);
        }
    }
    for marker in NODE_MARKERS {
        if detector.exists(&dir.join(marker)) {
            return Some(ProjectType::Node);
        }
    }
    for marker in PYTHON_MARKERS {
        if detector.exists(&dir.join(marker)) {
            return Some(ProjectType::Python);
        }
    }
    for marker in JAVA_MARKERS {
        if detector.exists(&dir.join(marker)) {
            return Some(ProjectType::Java);
        }
    }
    if has_csharp_markers(detector, dir) {
        return Some(ProjectType::CSharp);
    }
    None
}

/// Check for C# project markers (requires glob-style matching).
fn has_csharp_markers<D: ProjectDetector>(detector: &D, dir: &Path) -> bool {
    if let Ok(files) = detector.list_files(dir) {
        for name in files {
            let path = Path::new(&name);
            if path.extension().is_some_and(|ext| {
                ext.eq_ignore_ascii_case("csproj") || ext.eq_ignore_ascii_case("sln")
            }) {
                return true;
            }
        }
    }
    false
}

/// Directories to skip during subproject scanning.
fn is_excluded_dir(name: &str) -> bool {
    matches!(
        name,
        "node_modules"
            | "target"
            | "vendor"
            | "dist"
            | "build"
            | "__pycache__"
            | ".venv"
            | "venv"
            | "bin"
            | "obj"
            | "packages"
    )
}

/// Scan directory and detect project structure.
///
/// # Errors
/// Returns an error if directory cannot be read.
pub fn detect_projects<D: ProjectDetector>(
    detector: &D,
    root: &Path,
) -> std::io::Result<DetectionResult> {
    let root_type = detect_project_type(detector, root);

    let subdirs = detector.list_subdirs(root)?;
    let mut subprojects = Vec::new();

    for subdir in subdirs {
        let dir_name = subdir.file_name().unwrap_or_default().to_string_lossy();
        if dir_name.starts_with('.') || is_excluded_dir(&dir_name) {
            continue;
        }

        if let Some(project_type) = detect_project_type(detector, &subdir) {
            let relative_path = subdir
                .strip_prefix(root)
                .unwrap_or(&subdir)
                .to_string_lossy()
                .replace('\\', "/");

            subprojects.push(DetectedProject {
                path: relative_path,
                project_type,
            });
        }
    }

    let is_monorepo = !subprojects.is_empty();

    Ok(DetectionResult {
        root: root_type,
        subprojects,
        is_monorepo,
    })
}

/// Generate V2 config template based on detection result.
#[must_use]
pub fn generate_detected_config(result: &DetectionResult) -> String {
    let mut output = String::new();

    output.push_str("# sloc-guard configuration file\n");
    output.push_str("# Generated with --detect flag\n");

    if result.is_monorepo {
        output.push_str("# Detected: Monorepo\n");
    } else if let Some(ref root_type) = result.root {
        let _ = writeln!(output, "# Detected: {} project", root_type.name());
    } else {
        output.push_str("# Detected: Unknown project type\n");
    }
    output.push('\n');

    output.push_str("version = \"2\"\n\n");

    let effective_type = result.effective_type();

    output.push_str("[scanner]\n");
    output.push_str("gitignore = true\n");

    let mut excludes: Vec<&str> = vec!["**/.git/**"];
    if let Some(ref root_type) = result.root {
        excludes.extend(root_type.exclude_patterns());
    }
    for subproject in &result.subprojects {
        excludes.extend(subproject.project_type.exclude_patterns());
    }
    excludes.sort_unstable();
    excludes.dedup();

    if !excludes.is_empty() {
        output.push_str("exclude = [\n");
        for pattern in &excludes {
            let _ = writeln!(output, "    \"{pattern}\",");
        }
        output.push_str("]\n");
    }
    output.push('\n');

    output.push_str("[content]\n");

    let extensions = result
        .root
        .as_ref()
        .map_or_else(|| effective_type.extensions(), ProjectType::extensions);

    let _ = writeln!(
        output,
        "extensions = [{}]",
        extensions
            .iter()
            .map(|e| format!("\"{e}\""))
            .collect::<Vec<_>>()
            .join(", ")
    );
    let _ = writeln!(output, "max_lines = {}", effective_type.default_max_lines());
    output.push_str("warn_threshold = 0.9\n");
    output.push_str("skip_comments = true\n");
    output.push_str("skip_blank = true\n");
    output.push('\n');

    if result.is_monorepo && !result.subprojects.is_empty() {
        output.push_str("# Per-subdirectory rules (monorepo)\n");
        for subproject in &result.subprojects {
            let _ = writeln!(
                output,
                "\n# {} ({})",
                subproject.path,
                subproject.project_type.name()
            );
            output.push_str("[[content.rules]]\n");
            let _ = writeln!(output, "pattern = \"{}/**\"", subproject.path);
            let _ = writeln!(
                output,
                "max_lines = {}",
                subproject.project_type.default_max_lines()
            );
        }
        output.push('\n');
    }

    output.push_str("# [structure]\n");
    output.push_str("# Uncomment to enable directory structure checks\n");
    output.push_str("# max_files = 30\n");
    output.push_str("# max_dirs = 10\n");

    output
}

/// Generate config by detecting project type from a directory.
///
/// # Errors
/// Returns an error if the directory cannot be read.
pub fn generate_detected_config_from_dir(dir: &Path) -> Result<String> {
    let detector = RealProjectDetector;
    let result = detect_projects(&detector, dir).map_err(SlocGuardError::Io)?;

    if let Some(ref root_type) = result.root {
        eprintln!("Detected root project: {}", root_type.name());
    }
    if result.is_monorepo {
        eprintln!(
            "Detected monorepo with {} subprojects:",
            result.subprojects.len()
        );
        for sp in &result.subprojects {
            eprintln!("  - {}: {}", sp.path, sp.project_type.name());
        }
    }
    if result.root.is_none() && result.subprojects.is_empty() {
        eprintln!("No project markers detected, using generic defaults");
    }

    Ok(generate_detected_config(&result))
}

#[cfg(test)]
#[path = "detect_tests.rs"]
mod tests;
