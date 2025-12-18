use std::collections::HashMap;
use std::io::{Error, ErrorKind};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use super::*;

struct MockFileSystem {
    files: Mutex<HashMap<PathBuf, String>>,
    current_dir: PathBuf,
    home_dir: Option<PathBuf>,
}

impl MockFileSystem {
    fn new() -> Self {
        Self {
            files: Mutex::new(HashMap::new()),
            current_dir: PathBuf::from("/project"),
            home_dir: Some(PathBuf::from("/home/user")),
        }
    }

    fn with_file(self, path: impl Into<PathBuf>, content: &str) -> Self {
        self.files
            .lock()
            .unwrap()
            .insert(path.into(), content.to_string());
        self
    }

    fn with_current_dir(mut self, path: impl Into<PathBuf>) -> Self {
        self.current_dir = path.into();
        self
    }

    fn with_home_dir(mut self, path: Option<PathBuf>) -> Self {
        self.home_dir = path;
        self
    }
}

impl FileSystem for MockFileSystem {
    fn read_to_string(&self, path: &Path) -> std::io::Result<String> {
        let normalized = normalize_path(path);
        self.files
            .lock()
            .unwrap()
            .get(&normalized)
            .cloned()
            .ok_or_else(|| Error::new(ErrorKind::NotFound, "file not found"))
    }

    fn exists(&self, path: &Path) -> bool {
        let normalized = normalize_path(path);
        self.files.lock().unwrap().contains_key(&normalized)
    }

    fn current_dir(&self) -> std::io::Result<PathBuf> {
        Ok(self.current_dir.clone())
    }

    fn home_dir(&self) -> Option<PathBuf> {
        self.home_dir.clone()
    }
}

fn normalize_path(path: &Path) -> PathBuf {
    let path_str = path.to_string_lossy().replace('\\', "/");
    let mut components = Vec::new();
    for part in path_str.split('/') {
        match part {
            ".." => {
                components.pop();
            }
            "." | "" => {}
            _ => components.push(part),
        }
    }
    let normalized = if path_str.starts_with('/') {
        format!("/{}", components.join("/"))
    } else {
        components.join("/")
    };
    PathBuf::from(normalized)
}

#[test]
fn returns_default_when_no_config_found() {
    let fs = MockFileSystem::new();
    let loader = FileConfigLoader::with_fs(fs);

    let config = loader.load().unwrap();

    assert_eq!(config.default.max_lines, 500);
    assert!(config.default.skip_comments);
    assert!(config.default.skip_blank);
}

#[test]
fn loads_local_config_from_current_directory() {
    let config_content = r"
[default]
max_lines = 300
";

    let fs = MockFileSystem::new()
        .with_current_dir("/my/project")
        .with_file("/my/project/.sloc-guard.toml", config_content);

    let loader = FileConfigLoader::with_fs(fs);
    let config = loader.load().unwrap();

    assert_eq!(config.default.max_lines, 300);
}

#[test]
fn loads_user_config_as_fallback() {
    let config_content = r"
[default]
max_lines = 400
";

    let fs = MockFileSystem::new()
        .with_home_dir(Some(PathBuf::from("/home/testuser")))
        .with_file(
            "/home/testuser/.config/sloc-guard/config.toml",
            config_content,
        );

    let loader = FileConfigLoader::with_fs(fs);
    let config = loader.load().unwrap();

    assert_eq!(config.default.max_lines, 400);
}

#[test]
fn local_config_takes_priority_over_user_config() {
    let local_content = r"
[default]
max_lines = 200
";
    let user_content = r"
[default]
max_lines = 600
";

    let fs = MockFileSystem::new()
        .with_current_dir("/project")
        .with_home_dir(Some(PathBuf::from("/home/user")))
        .with_file("/project/.sloc-guard.toml", local_content)
        .with_file("/home/user/.config/sloc-guard/config.toml", user_content);

    let loader = FileConfigLoader::with_fs(fs);
    let config = loader.load().unwrap();

    assert_eq!(config.default.max_lines, 200);
}

#[test]
fn load_from_explicit_path() {
    let config_content = r#"
[default]
max_lines = 700
extensions = ["rs", "py"]
"#;

    let fs = MockFileSystem::new().with_file("/custom/path/config.toml", config_content);

    let loader = FileConfigLoader::with_fs(fs);
    let config = loader
        .load_from_path(Path::new("/custom/path/config.toml"))
        .unwrap();

    assert_eq!(config.default.max_lines, 700);
    assert_eq!(config.default.extensions, vec!["rs", "py"]);
}

#[test]
fn returns_error_for_invalid_toml() {
    let invalid_content = "this is not valid toml [[[";

    let fs = MockFileSystem::new().with_file("/project/.sloc-guard.toml", invalid_content);

    let loader = FileConfigLoader::with_fs(fs);
    let result = loader.load_from_path(Path::new("/project/.sloc-guard.toml"));

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(matches!(err, SlocGuardError::TomlParse(_)));
}

#[test]
fn returns_error_for_nonexistent_explicit_path() {
    let fs = MockFileSystem::new();

    let loader = FileConfigLoader::with_fs(fs);
    let result = loader.load_from_path(Path::new("/does/not/exist.toml"));

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(matches!(err, SlocGuardError::FileRead { .. }));
}

#[test]
fn handles_missing_home_dir() {
    let fs = MockFileSystem::new().with_home_dir(None);

    let loader = FileConfigLoader::with_fs(fs);
    let config = loader.load().unwrap();

    assert_eq!(config, Config::default());
}

#[test]
fn parses_full_config_with_rules_and_overrides() {
    let config_content = r#"
[default]
max_lines = 500
extensions = ["rs", "go"]
include_paths = ["src", "lib"]
skip_comments = true
skip_blank = true
warn_threshold = 0.85

[rules.rust]
extensions = ["rs"]
max_lines = 300

[exclude]
patterns = ["**/target/**", "**/vendor/**"]

[[override]]
path = "src/legacy.rs"
max_lines = 800
reason = "Legacy code"
"#;

    let fs = MockFileSystem::new().with_file("/config.toml", config_content);

    let loader = FileConfigLoader::with_fs(fs);
    let config = loader.load_from_path(Path::new("/config.toml")).unwrap();

    assert_eq!(config.default.max_lines, 500);
    assert_eq!(config.default.extensions, vec!["rs", "go"]);
    assert_eq!(config.default.include_paths, vec!["src", "lib"]);
    assert!(config.default.skip_comments);
    assert!(config.default.skip_blank);
    assert!((config.default.warn_threshold - 0.85).abs() < f64::EPSILON);

    let rust_rule = config.rules.get("rust").unwrap();
    assert_eq!(rust_rule.extensions, vec!["rs"]);
    assert_eq!(rust_rule.max_lines, Some(300));

    assert_eq!(
        config.exclude.patterns,
        vec!["**/target/**", "**/vendor/**"]
    );

    assert_eq!(config.overrides.len(), 1);
    assert_eq!(config.overrides[0].path, "src/legacy.rs");
    assert_eq!(config.overrides[0].max_lines, 800);
    assert_eq!(config.overrides[0].reason, Some("Legacy code".to_string()));
}

#[test]
fn default_loader_can_be_created() {
    let _loader = FileConfigLoader::new();
    let _loader_default = FileConfigLoader::default();
}

#[test]
fn extends_loads_base_config() {
    let base_content = r#"
[default]
max_lines = 300
extensions = ["rs", "go"]
"#;
    let child_content = r#"
extends = "/base/config.toml"

[default]
max_lines = 500
"#;

    let fs = MockFileSystem::new()
        .with_file("/base/config.toml", base_content)
        .with_file("/project/config.toml", child_content);

    let loader = FileConfigLoader::with_fs(fs);
    let config = loader
        .load_from_path(Path::new("/project/config.toml"))
        .unwrap();

    assert_eq!(config.default.max_lines, 500);
    assert_eq!(config.default.extensions, vec!["rs", "go"]);
    assert!(config.extends.is_none());
}

#[test]
fn extends_with_relative_path() {
    let base_content = r"
[default]
max_lines = 200
";
    let child_content = r#"
extends = "../base/config.toml"

[default]
skip_comments = false
"#;

    let fs = MockFileSystem::new()
        .with_file("/configs/base/config.toml", base_content)
        .with_file("/configs/project/config.toml", child_content);

    let loader = FileConfigLoader::with_fs(fs);
    let config = loader
        .load_from_path(Path::new("/configs/project/config.toml"))
        .unwrap();

    assert_eq!(config.default.max_lines, 200);
    assert!(!config.default.skip_comments);
}

#[test]
fn extends_chain_works() {
    let grandparent_content = r#"
[default]
max_lines = 100

[exclude]
patterns = ["**/vendor/**"]
"#;
    let parent_content = r#"
extends = "/configs/grandparent.toml"

[default]
max_lines = 200
"#;
    let child_content = r#"
extends = "/configs/parent.toml"

[default]
max_lines = 300
"#;

    let fs = MockFileSystem::new()
        .with_file("/configs/grandparent.toml", grandparent_content)
        .with_file("/configs/parent.toml", parent_content)
        .with_file("/configs/child.toml", child_content);

    let loader = FileConfigLoader::with_fs(fs);
    let config = loader
        .load_from_path(Path::new("/configs/child.toml"))
        .unwrap();

    assert_eq!(config.default.max_lines, 300);
    assert_eq!(config.exclude.patterns, vec!["**/vendor/**"]);
}

#[test]
fn extends_detects_direct_cycle() {
    let config_a = r#"
extends = "/configs/b.toml"
"#;
    let config_b = r#"
extends = "/configs/a.toml"
"#;

    let fs = MockFileSystem::new()
        .with_file("/configs/a.toml", config_a)
        .with_file("/configs/b.toml", config_b);

    let loader = FileConfigLoader::with_fs(fs);
    let result = loader.load_from_path(Path::new("/configs/a.toml"));

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(matches!(err, SlocGuardError::Config(msg) if msg.contains("Circular")));
}

#[test]
fn extends_detects_self_reference() {
    let config = r#"
extends = "/configs/self.toml"

[default]
max_lines = 100
"#;

    let fs = MockFileSystem::new().with_file("/configs/self.toml", config);

    let loader = FileConfigLoader::with_fs(fs);
    let result = loader.load_from_path(Path::new("/configs/self.toml"));

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(matches!(err, SlocGuardError::Config(msg) if msg.contains("Circular")));
}

#[test]
fn extends_merges_rules() {
    let base_content = r#"
[rules.rust]
extensions = ["rs"]
max_lines = 300

[rules.python]
extensions = ["py"]
max_lines = 400
"#;
    let child_content = r#"
extends = "/base.toml"

[rules.rust]
max_lines = 500

[rules.go]
extensions = ["go"]
max_lines = 600
"#;

    let fs = MockFileSystem::new()
        .with_file("/base.toml", base_content)
        .with_file("/child.toml", child_content);

    let loader = FileConfigLoader::with_fs(fs);
    let config = loader.load_from_path(Path::new("/child.toml")).unwrap();

    // Child overrides max_lines but inherits extensions from base
    let rust_rule = config.rules.get("rust").unwrap();
    assert_eq!(rust_rule.max_lines, Some(500));
    assert_eq!(rust_rule.extensions, vec!["rs"]);

    // Python rule inherited entirely from base
    let python_rule = config.rules.get("python").unwrap();
    assert_eq!(python_rule.max_lines, Some(400));
    assert_eq!(python_rule.extensions, vec!["py"]);

    // Go rule defined only in child
    let go_rule = config.rules.get("go").unwrap();
    assert_eq!(go_rule.max_lines, Some(600));
    assert_eq!(go_rule.extensions, vec!["go"]);
}

#[test]
fn extends_error_on_missing_base() {
    let child_content = r#"
extends = "/nonexistent/base.toml"

[default]
max_lines = 100
"#;

    let fs = MockFileSystem::new().with_file("/child.toml", child_content);

    let loader = FileConfigLoader::with_fs(fs);
    let result = loader.load_from_path(Path::new("/child.toml"));

    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        SlocGuardError::FileRead { .. }
    ));
}

#[test]
fn load_without_extends_ignores_extends_field() {
    let base_content = r"
[default]
max_lines = 100
";
    let child_content = r#"
extends = "/base.toml"

[default]
max_lines = 200
"#;

    let fs = MockFileSystem::new()
        .with_file("/base.toml", base_content)
        .with_file("/project/.sloc-guard.toml", child_content)
        .with_current_dir("/project");

    let loader = FileConfigLoader::with_fs(fs);
    let config = loader.load_without_extends().unwrap();

    // Should have max_lines from child only, not merged with base
    assert_eq!(config.default.max_lines, 200);
    // Extends field should be preserved in the config
    assert_eq!(config.extends, Some("/base.toml".to_string()));
}

#[test]
fn load_from_path_without_extends_ignores_extends() {
    let base_content = r#"
[default]
max_lines = 100
extensions = ["rs", "go"]
"#;
    let child_content = r#"
extends = "/base.toml"

[default]
max_lines = 300
"#;

    let fs = MockFileSystem::new()
        .with_file("/base.toml", base_content)
        .with_file("/child.toml", child_content);

    let loader = FileConfigLoader::with_fs(fs);
    let config = loader
        .load_from_path_without_extends(Path::new("/child.toml"))
        .unwrap();

    // Should have only child's max_lines, not merged
    assert_eq!(config.default.max_lines, 300);
    // Extensions should be default (not from base)
    assert_eq!(
        config.default.extensions,
        Config::default().default.extensions
    );
    // Extends field should be preserved
    assert_eq!(config.extends, Some("/base.toml".to_string()));
}

#[test]
fn load_without_extends_falls_back_to_user_config() {
    let user_content = r#"
extends = "https://example.com/base.toml"

[default]
max_lines = 400
"#;

    let fs = MockFileSystem::new()
        .with_home_dir(Some(PathBuf::from("/home/user")))
        .with_file("/home/user/.config/sloc-guard/config.toml", user_content);

    let loader = FileConfigLoader::with_fs(fs);
    let config = loader.load_without_extends().unwrap();

    assert_eq!(config.default.max_lines, 400);
    assert_eq!(
        config.extends,
        Some("https://example.com/base.toml".to_string())
    );
}

#[test]
fn config_with_valid_version_loads_successfully() {
    let config_content = r#"
version = "1"

[default]
max_lines = 300
"#;

    let fs = MockFileSystem::new().with_file("/config.toml", config_content);

    let loader = FileConfigLoader::with_fs(fs);
    let config = loader.load_from_path(Path::new("/config.toml")).unwrap();

    assert_eq!(config.version, Some("1".to_string()));
    assert_eq!(config.default.max_lines, 300);
}

#[test]
fn config_without_version_loads_successfully() {
    let config_content = r"
[default]
max_lines = 400
";

    let fs = MockFileSystem::new().with_file("/config.toml", config_content);

    let loader = FileConfigLoader::with_fs(fs);
    let config = loader.load_from_path(Path::new("/config.toml")).unwrap();

    assert!(config.version.is_none());
    assert_eq!(config.default.max_lines, 400);
}

#[test]
fn config_with_unsupported_version_returns_error() {
    let config_content = r#"
version = "99"

[default]
max_lines = 300
"#;

    let fs = MockFileSystem::new().with_file("/config.toml", config_content);

    let loader = FileConfigLoader::with_fs(fs);
    let result = loader.load_from_path(Path::new("/config.toml"));

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(matches!(err, SlocGuardError::Config(msg) if msg.contains("Unsupported config version")));
}

#[test]
fn config_with_unsupported_version_error_contains_version() {
    let config_content = r#"
version = "2.0"

[default]
max_lines = 300
"#;

    let fs = MockFileSystem::new().with_file("/config.toml", config_content);

    let loader = FileConfigLoader::with_fs(fs);
    let result = loader.load_from_path(Path::new("/config.toml"));

    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("2.0"));
    assert!(err_msg.contains("'1'")); // Should mention supported version
}
