use std::collections::HashSet;
use std::path::{Path, PathBuf};

use super::*;

/// Mock detector for testing.
struct MockDetector {
    existing_files: HashSet<PathBuf>,
    subdirs: Vec<PathBuf>,
    files_in_dir: Vec<String>,
}

impl MockDetector {
    fn new() -> Self {
        Self {
            existing_files: HashSet::new(),
            subdirs: Vec::new(),
            files_in_dir: Vec::new(),
        }
    }

    fn with_file(mut self, path: impl Into<PathBuf>) -> Self {
        self.existing_files.insert(path.into());
        self
    }

    fn with_subdir(mut self, path: impl Into<PathBuf>) -> Self {
        self.subdirs.push(path.into());
        self
    }

    fn with_dir_file(mut self, name: &str) -> Self {
        self.files_in_dir.push(name.to_string());
        self
    }
}

impl ProjectDetector for MockDetector {
    fn exists(&self, path: &Path) -> bool {
        self.existing_files.contains(path)
    }

    fn list_subdirs(&self, _path: &Path) -> std::io::Result<Vec<PathBuf>> {
        Ok(self.subdirs.clone())
    }

    fn list_files(&self, _path: &Path) -> std::io::Result<Vec<String>> {
        Ok(self.files_in_dir.clone())
    }
}

#[test]
fn detect_rust_project() {
    let detector = MockDetector::new().with_file("/project/Cargo.toml");

    let result = detect_projects(&detector, Path::new("/project")).unwrap();

    assert_eq!(result.root, Some(ProjectType::Rust));
    assert!(!result.is_monorepo);
}

#[test]
fn detect_node_project() {
    let detector = MockDetector::new().with_file("/project/package.json");

    let result = detect_projects(&detector, Path::new("/project")).unwrap();

    assert_eq!(result.root, Some(ProjectType::Node));
}

#[test]
fn detect_go_project() {
    let detector = MockDetector::new().with_file("/project/go.mod");

    let result = detect_projects(&detector, Path::new("/project")).unwrap();

    assert_eq!(result.root, Some(ProjectType::Go));
}

#[test]
fn detect_python_project_pyproject() {
    let detector = MockDetector::new().with_file("/project/pyproject.toml");

    let result = detect_projects(&detector, Path::new("/project")).unwrap();

    assert_eq!(result.root, Some(ProjectType::Python));
}

#[test]
fn detect_python_project_requirements() {
    let detector = MockDetector::new().with_file("/project/requirements.txt");

    let result = detect_projects(&detector, Path::new("/project")).unwrap();

    assert_eq!(result.root, Some(ProjectType::Python));
}

#[test]
fn detect_java_project_maven() {
    let detector = MockDetector::new().with_file("/project/pom.xml");

    let result = detect_projects(&detector, Path::new("/project")).unwrap();

    assert_eq!(result.root, Some(ProjectType::Java));
}

#[test]
fn detect_java_project_gradle() {
    let detector = MockDetector::new().with_file("/project/build.gradle");

    let result = detect_projects(&detector, Path::new("/project")).unwrap();

    assert_eq!(result.root, Some(ProjectType::Java));
}

#[test]
fn detect_csharp_project() {
    let detector = MockDetector::new().with_dir_file("MyProject.csproj");

    let result = detect_projects(&detector, Path::new("/project")).unwrap();

    assert_eq!(result.root, Some(ProjectType::CSharp));
}

#[test]
fn detect_csharp_solution() {
    let detector = MockDetector::new().with_dir_file("MyApp.sln");

    let result = detect_projects(&detector, Path::new("/project")).unwrap();

    assert_eq!(result.root, Some(ProjectType::CSharp));
}

#[test]
fn detect_monorepo() {
    let detector = MockDetector::new()
        .with_file("/project/package.json")
        .with_subdir("/project/api")
        .with_subdir("/project/web")
        .with_file("/project/api/Cargo.toml")
        .with_file("/project/web/package.json");

    let result = detect_projects(&detector, Path::new("/project")).unwrap();

    assert_eq!(result.root, Some(ProjectType::Node));
    assert!(result.is_monorepo);
    assert_eq!(result.subprojects.len(), 2);

    let api = result
        .subprojects
        .iter()
        .find(|p| p.path.contains("api"))
        .unwrap();
    assert_eq!(api.project_type, ProjectType::Rust);

    let web = result
        .subprojects
        .iter()
        .find(|p| p.path.contains("web"))
        .unwrap();
    assert_eq!(web.project_type, ProjectType::Node);
}

#[test]
fn detect_unknown_when_no_markers() {
    let detector = MockDetector::new();

    let result = detect_projects(&detector, Path::new("/project")).unwrap();

    assert_eq!(result.root, None);
    assert_eq!(result.effective_type(), ProjectType::Unknown);
}

#[test]
fn skips_hidden_directories() {
    let detector = MockDetector::new()
        .with_subdir("/project/.git")
        .with_subdir("/project/.vscode")
        .with_file("/project/.git/Cargo.toml");

    let result = detect_projects(&detector, Path::new("/project")).unwrap();

    assert!(result.subprojects.is_empty());
}

#[test]
fn skips_excluded_directories() {
    let detector = MockDetector::new()
        .with_subdir("/project/node_modules")
        .with_subdir("/project/target")
        .with_file("/project/node_modules/package.json");

    let result = detect_projects(&detector, Path::new("/project")).unwrap();

    assert!(result.subprojects.is_empty());
}

#[test]
fn project_type_extensions() {
    assert_eq!(ProjectType::Rust.extensions(), vec!["rs"]);
    assert!(ProjectType::Node.extensions().contains(&"ts"));
    assert!(ProjectType::Node.extensions().contains(&"js"));
    assert_eq!(ProjectType::Go.extensions(), vec!["go"]);
    assert!(ProjectType::Python.extensions().contains(&"py"));
    assert!(ProjectType::Java.extensions().contains(&"java"));
    assert!(ProjectType::Java.extensions().contains(&"kt"));
    assert_eq!(ProjectType::CSharp.extensions(), vec!["cs"]);
}

#[test]
fn project_type_max_lines() {
    assert_eq!(ProjectType::Rust.default_max_lines(), 800);
    assert_eq!(ProjectType::Node.default_max_lines(), 400);
    assert_eq!(ProjectType::Go.default_max_lines(), 600);
    assert_eq!(ProjectType::Python.default_max_lines(), 500);
    assert_eq!(ProjectType::Java.default_max_lines(), 500);
    assert_eq!(ProjectType::CSharp.default_max_lines(), 600);
    assert_eq!(ProjectType::Unknown.default_max_lines(), 500);
}

#[test]
fn project_type_exclude_patterns() {
    assert!(ProjectType::Rust
        .exclude_patterns()
        .contains(&"**/target/**"));
    assert!(ProjectType::Node
        .exclude_patterns()
        .contains(&"**/node_modules/**"));
    assert!(ProjectType::Go.exclude_patterns().contains(&"**/vendor/**"));
    assert!(ProjectType::Python
        .exclude_patterns()
        .contains(&"**/__pycache__/**"));
    assert!(ProjectType::Java
        .exclude_patterns()
        .contains(&"**/target/**"));
    assert!(ProjectType::Java
        .exclude_patterns()
        .contains(&"**/build/**"));
    assert!(ProjectType::CSharp.exclude_patterns().contains(&"**/bin/**"));
    assert!(ProjectType::CSharp.exclude_patterns().contains(&"**/obj/**"));
    assert!(ProjectType::Unknown.exclude_patterns().is_empty());
}

#[test]
fn project_type_name() {
    assert_eq!(ProjectType::Rust.name(), "Rust");
    assert_eq!(ProjectType::Node.name(), "Node.js/TypeScript");
    assert_eq!(ProjectType::Go.name(), "Go");
    assert_eq!(ProjectType::Python.name(), "Python");
    assert_eq!(ProjectType::Java.name(), "Java/Kotlin");
    assert_eq!(ProjectType::CSharp.name(), "C#/.NET");
    assert_eq!(ProjectType::Unknown.name(), "Unknown");
}

#[test]
fn generate_config_for_rust_project() {
    let result = DetectionResult {
        root: Some(ProjectType::Rust),
        subprojects: vec![],
        is_monorepo: false,
    };

    let config = generate_detected_config(&result);

    assert!(config.contains("version = \"2\""));
    assert!(config.contains("\"rs\""));
    assert!(config.contains("**/target/**"));
    assert!(config.contains("max_lines = 800"));
}

#[test]
fn generate_config_for_node_project() {
    let result = DetectionResult {
        root: Some(ProjectType::Node),
        subprojects: vec![],
        is_monorepo: false,
    };

    let config = generate_detected_config(&result);

    assert!(config.contains("version = \"2\""));
    assert!(config.contains("\"ts\""));
    assert!(config.contains("\"js\""));
    assert!(config.contains("**/node_modules/**"));
    assert!(config.contains("max_lines = 400"));
}

#[test]
fn generate_config_for_java_project() {
    let result = DetectionResult {
        root: Some(ProjectType::Java),
        subprojects: vec![],
        is_monorepo: false,
    };

    let config = generate_detected_config(&result);

    assert!(config.contains("version = \"2\""));
    assert!(config.contains("\"java\""));
    assert!(config.contains("\"kt\""));
    assert!(config.contains("**/target/**"));
    assert!(config.contains("**/build/**"));
    assert!(config.contains("max_lines = 500"));
}

#[test]
fn generate_config_for_csharp_project() {
    let result = DetectionResult {
        root: Some(ProjectType::CSharp),
        subprojects: vec![],
        is_monorepo: false,
    };

    let config = generate_detected_config(&result);

    assert!(config.contains("version = \"2\""));
    assert!(config.contains("\"cs\""));
    assert!(config.contains("**/bin/**"));
    assert!(config.contains("**/obj/**"));
    assert!(config.contains("max_lines = 600"));
}

#[test]
fn generate_config_for_python_project() {
    let result = DetectionResult {
        root: Some(ProjectType::Python),
        subprojects: vec![],
        is_monorepo: false,
    };

    let config = generate_detected_config(&result);

    assert!(config.contains("version = \"2\""));
    assert!(config.contains("\"py\""));
    assert!(config.contains("**/__pycache__/**"));
    assert!(config.contains("max_lines = 500"));
}

#[test]
fn generate_config_for_go_project() {
    let result = DetectionResult {
        root: Some(ProjectType::Go),
        subprojects: vec![],
        is_monorepo: false,
    };

    let config = generate_detected_config(&result);

    assert!(config.contains("version = \"2\""));
    assert!(config.contains("\"go\""));
    assert!(config.contains("**/vendor/**"));
    assert!(config.contains("max_lines = 600"));
}

#[test]
fn generate_config_for_monorepo() {
    let result = DetectionResult {
        root: Some(ProjectType::Node),
        subprojects: vec![
            DetectedProject {
                path: "packages/api".to_string(),
                project_type: ProjectType::Rust,
            },
            DetectedProject {
                path: "packages/web".to_string(),
                project_type: ProjectType::Node,
            },
        ],
        is_monorepo: true,
    };

    let config = generate_detected_config(&result);

    assert!(config.contains("[[content.rules]]"));
    assert!(config.contains("pattern = \"packages/api/**\""));
    assert!(config.contains("pattern = \"packages/web/**\""));
    assert!(config.contains("max_lines = 800")); // Rust
    assert!(config.contains("max_lines = 400")); // Node
}

#[test]
fn generate_config_for_unknown_project() {
    let result = DetectionResult {
        root: None,
        subprojects: vec![],
        is_monorepo: false,
    };

    let config = generate_detected_config(&result);

    assert!(config.contains("version = \"2\""));
    assert!(config.contains("Unknown project type"));
    assert!(config.contains("max_lines = 500"));
}

#[test]
fn generated_config_is_valid_toml() {
    let result = DetectionResult {
        root: Some(ProjectType::Rust),
        subprojects: vec![],
        is_monorepo: false,
    };

    let config = generate_detected_config(&result);
    let parsed: std::result::Result<toml::Value, _> = toml::from_str(&config);

    assert!(parsed.is_ok(), "Generated config should be valid TOML");
}

#[test]
fn generated_monorepo_config_is_valid_toml() {
    let result = DetectionResult {
        root: Some(ProjectType::Node),
        subprojects: vec![
            DetectedProject {
                path: "api".to_string(),
                project_type: ProjectType::Rust,
            },
            DetectedProject {
                path: "web".to_string(),
                project_type: ProjectType::Node,
            },
        ],
        is_monorepo: true,
    };

    let config = generate_detected_config(&result);
    let parsed: std::result::Result<toml::Value, _> = toml::from_str(&config);

    assert!(
        parsed.is_ok(),
        "Generated monorepo config should be valid TOML"
    );
}

#[test]
fn detected_project_is_root() {
    let root_project = DetectedProject {
        path: String::new(),
        project_type: ProjectType::Rust,
    };
    assert!(root_project.is_root());

    let sub_project = DetectedProject {
        path: "packages/api".to_string(),
        project_type: ProjectType::Rust,
    };
    assert!(!sub_project.is_root());
}

#[test]
fn detection_result_effective_type_with_root() {
    let result = DetectionResult {
        root: Some(ProjectType::Rust),
        subprojects: vec![],
        is_monorepo: false,
    };
    assert_eq!(result.effective_type(), ProjectType::Rust);
}

#[test]
fn detection_result_effective_type_monorepo_without_root() {
    let result = DetectionResult {
        root: None,
        subprojects: vec![DetectedProject {
            path: "api".to_string(),
            project_type: ProjectType::Rust,
        }],
        is_monorepo: true,
    };
    assert_eq!(result.effective_type(), ProjectType::Unknown);
}

#[test]
fn rust_priority_over_node() {
    let detector = MockDetector::new()
        .with_file("/project/Cargo.toml")
        .with_file("/project/package.json");

    let result = detect_projects(&detector, Path::new("/project")).unwrap();

    assert_eq!(result.root, Some(ProjectType::Rust));
}

#[test]
fn go_priority_over_node() {
    let detector = MockDetector::new()
        .with_file("/project/go.mod")
        .with_file("/project/package.json");

    let result = detect_projects(&detector, Path::new("/project")).unwrap();

    assert_eq!(result.root, Some(ProjectType::Go));
}
