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
        self.files
            .lock()
            .unwrap()
            .get(path)
            .cloned()
            .ok_or_else(|| Error::new(ErrorKind::NotFound, "file not found"))
    }

    fn exists(&self, path: &Path) -> bool {
        self.files.lock().unwrap().contains_key(path)
    }

    fn current_dir(&self) -> std::io::Result<PathBuf> {
        Ok(self.current_dir.clone())
    }

    fn home_dir(&self) -> Option<PathBuf> {
        self.home_dir.clone()
    }
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
    assert_eq!(
        config.overrides[0].reason,
        Some("Legacy code".to_string())
    );
}

#[test]
fn default_loader_can_be_created() {
    let _loader = FileConfigLoader::new();
    let _loader_default = FileConfigLoader::default();
}
