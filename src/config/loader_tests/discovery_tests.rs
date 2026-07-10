//! Tests for config file discovery from various locations (current dir, user config, fallback).

use std::path::{Path, PathBuf};

use crate::config::loader::ConfigLoader;
use crate::config::{Config, ConfigOrigin, FileConfigLoader, LoadResult};
use crate::error::SlocGuardError;

use super::mock_fs::MockFileSystem;

#[test]
fn returns_default_when_no_config_found() {
    let fs = MockFileSystem::new();
    let loader = FileConfigLoader::with_fs(fs);

    let result = loader.load_located().unwrap();

    assert_eq!(result.config.content.max_lines, 600);
    assert!(result.config.content.skip_comments);
    assert!(result.config.content.skip_blank);
    assert!(result.preset_used.is_none());
    assert_eq!(result.origin, ConfigOrigin::BuiltInDefaults);
}

#[test]
fn public_load_result_keeps_legacy_constructible_shape() {
    let result = LoadResult {
        config: Config::default(),
        preset_used: None,
    };

    assert_eq!(result.config, Config::default());
}

#[test]
fn loads_local_config_from_current_directory() {
    let config_content = r#"
version = "2"

[content]
max_lines = 300
"#;

    let fs = MockFileSystem::new()
        .with_current_dir("/my/project")
        .with_file("/my/project/.sloc-guard.toml", config_content);

    let loader = FileConfigLoader::with_fs(fs);
    let result = loader.load_located().unwrap();

    assert_eq!(result.config.content.max_lines, 300);
    assert_eq!(
        result.origin,
        ConfigOrigin::ProjectFile(PathBuf::from("/my/project/.sloc-guard.toml"))
    );
}

#[test]
fn loads_nearest_config_from_parent_directory() {
    let parent_content = r#"
version = "2"

[content]
max_lines = 321
"#;

    let fs = MockFileSystem::new()
        .with_current_dir("/workspace/project/src/deep")
        .with_file("/workspace/project/.sloc-guard.toml", parent_content);

    let loader = FileConfigLoader::with_fs(fs);
    let result = loader.load_located().unwrap();

    assert_eq!(result.config.content.max_lines, 321);
    assert_eq!(
        result.origin,
        ConfigOrigin::ProjectFile(PathBuf::from("/workspace/project/.sloc-guard.toml"))
    );
}

#[test]
fn nearest_project_config_wins() {
    let outer_content = r#"
version = "2"
[content]
max_lines = 111
"#;
    let inner_content = r#"
version = "2"
[content]
max_lines = 222
"#;

    let fs = MockFileSystem::new()
        .with_current_dir("/workspace/packages/inner/src")
        .with_file("/workspace/.sloc-guard.toml", outer_content)
        .with_file("/workspace/packages/inner/.sloc-guard.toml", inner_content);

    let result = FileConfigLoader::with_fs(fs).load_located().unwrap();

    assert_eq!(result.config.content.max_lines, 222);
    assert_eq!(
        result.origin,
        ConfigOrigin::ProjectFile(PathBuf::from("/workspace/packages/inner/.sloc-guard.toml"))
    );
}

#[test]
fn git_file_stops_parent_config_search_before_user_fallback() {
    let parent_content = r#"
version = "2"
[content]
max_lines = 111
"#;
    let user_content = r#"
version = "2"
[content]
max_lines = 444
"#;

    let fs = MockFileSystem::new()
        .with_current_dir("/workspace/repository/src")
        .with_config_dir(Some(PathBuf::from("/home/user/.config/sloc-guard")))
        .with_file("/workspace/.sloc-guard.toml", parent_content)
        .with_file(
            "/workspace/repository/.git",
            "gitdir: ../git/worktrees/repository",
        )
        .with_file("/home/user/.config/sloc-guard/config.toml", user_content);

    let result = FileConfigLoader::with_fs(fs).load_located().unwrap();

    assert_eq!(result.config.content.max_lines, 444);
    assert_eq!(
        result.origin,
        ConfigOrigin::UserFile(PathBuf::from("/home/user/.config/sloc-guard/config.toml"))
    );
}

#[test]
fn git_directory_stops_parent_config_search() {
    let parent_content = r#"
version = "2"
[content]
max_lines = 111
"#;

    let fs = MockFileSystem::new()
        .with_current_dir("/workspace/repository/src")
        .with_config_dir(None)
        .with_file("/workspace/.sloc-guard.toml", parent_content)
        .with_dir("/workspace/repository/.git");

    let result = FileConfigLoader::with_fs(fs).load_located().unwrap();

    assert_eq!(result.config, Config::default());
    assert_eq!(result.origin, ConfigOrigin::BuiltInDefaults);
}

#[test]
fn config_at_git_boundary_is_selected() {
    let config_content = r#"
version = "2"
[content]
max_lines = 333
"#;

    let fs = MockFileSystem::new()
        .with_current_dir("/workspace/repository/src")
        .with_file(
            "/workspace/repository/.git",
            "gitdir: ../git/worktrees/repository",
        )
        .with_file("/workspace/repository/.sloc-guard.toml", config_content);

    let result = FileConfigLoader::with_fs(fs).load_located().unwrap();

    assert_eq!(result.config.content.max_lines, 333);
    assert_eq!(
        result.origin,
        ConfigOrigin::ProjectFile(PathBuf::from("/workspace/repository/.sloc-guard.toml"))
    );
}

#[test]
fn all_implicit_load_entry_points_share_parent_discovery() {
    let config_content = r#"
version = "2"
[content]
max_lines = 275
"#;

    let fs = MockFileSystem::new()
        .with_current_dir("/workspace/project/src")
        .with_file("/workspace/project/.sloc-guard.toml", config_content);
    let loader = FileConfigLoader::with_fs(fs);

    assert_eq!(loader.load().unwrap().config.content.max_lines, 275);
    assert_eq!(
        loader
            .load_without_extends()
            .unwrap()
            .config
            .content
            .max_lines,
        275
    );
    assert_eq!(
        loader.load_with_sources().unwrap().config.content.max_lines,
        275
    );
    assert_eq!(
        loader
            .load_without_extends_with_sources()
            .unwrap()
            .config
            .content
            .max_lines,
        275
    );
}

#[test]
fn implicit_load_entry_points_propagate_current_directory_errors() {
    let loader = FileConfigLoader::with_fs(MockFileSystem::new().with_current_dir_error());

    assert!(matches!(loader.load(), Err(SlocGuardError::Io { .. })));
    assert!(matches!(
        loader.load_without_extends(),
        Err(SlocGuardError::Io { .. })
    ));
    assert!(matches!(
        loader.load_with_sources(),
        Err(SlocGuardError::Io { .. })
    ));
    assert!(matches!(
        loader.load_without_extends_with_sources(),
        Err(SlocGuardError::Io { .. })
    ));
}

#[test]
fn loads_user_config_as_fallback() {
    let config_content = r#"
version = "2"

[content]
max_lines = 400
"#;

    let fs = MockFileSystem::new()
        .with_config_dir(Some(PathBuf::from("/home/testuser/.config/sloc-guard")))
        .with_file(
            "/home/testuser/.config/sloc-guard/config.toml",
            config_content,
        );

    let loader = FileConfigLoader::with_fs(fs);
    let result = loader.load_located().unwrap();

    assert_eq!(result.config.content.max_lines, 400);
    assert_eq!(
        result.origin,
        ConfigOrigin::UserFile(PathBuf::from(
            "/home/testuser/.config/sloc-guard/config.toml"
        ))
    );
}

#[test]
fn local_config_takes_priority_over_user_config() {
    let local_content = r#"
version = "2"

[content]
max_lines = 200
"#;
    let user_content = r#"
version = "2"

[content]
max_lines = 600
"#;

    let fs = MockFileSystem::new()
        .with_current_dir("/project")
        .with_config_dir(Some(PathBuf::from("/home/user/.config/sloc-guard")))
        .with_file("/project/.sloc-guard.toml", local_content)
        .with_file("/home/user/.config/sloc-guard/config.toml", user_content);

    let loader = FileConfigLoader::with_fs(fs);
    let result = loader.load_located().unwrap();

    assert_eq!(result.config.content.max_lines, 200);
}

#[test]
fn handles_missing_config_dir() {
    let fs = MockFileSystem::new().with_config_dir(None);

    let loader = FileConfigLoader::with_fs(fs);
    let result = loader.load_located().unwrap();

    assert_eq!(result.config, Config::default());
    assert_eq!(result.origin, ConfigOrigin::BuiltInDefaults);
}

#[test]
fn explicit_config_origin_is_absolute_and_normalized() {
    let config_content = r#"
version = "2"
[content]
max_lines = 250
"#;
    let fs = MockFileSystem::new()
        .with_current_dir("/workspace/project/src")
        .with_file("/workspace/project/.sloc-guard.toml", config_content);
    let loader = FileConfigLoader::with_fs(fs);

    let result = loader
        .load_from_path_located(Path::new(".././.sloc-guard.toml"))
        .unwrap();

    assert_eq!(
        result.origin,
        ConfigOrigin::ExplicitFile(PathBuf::from("/workspace/project/.sloc-guard.toml"))
    );
}

#[test]
fn implicit_and_explicit_symlink_configs_use_the_link_directory_for_extends() {
    let linked_config = r#"
version = "2"
extends = "base.toml"
"#;
    let link_base = r#"
version = "2"
[content]
max_lines = 310
"#;
    let target_base = r#"
version = "2"
[content]
max_lines = 120
"#;

    let fs = MockFileSystem::new()
        .with_current_dir("/workspace/repository/src")
        .with_file("/workspace/repository/.sloc-guard.toml", linked_config)
        .with_file("/workspace/repository/base.toml", link_base)
        .with_file("/shared/base.toml", target_base)
        .with_canonical_path(
            "/workspace/repository/.sloc-guard.toml",
            "/shared/config.toml",
        );
    let loader = FileConfigLoader::with_fs(fs);

    let implicit = loader.load_located().unwrap();
    let explicit = loader
        .load_from_path_located(Path::new("../.sloc-guard.toml"))
        .unwrap();

    assert_eq!(implicit.config.content.max_lines, 310);
    assert_eq!(explicit.config.content.max_lines, 310);
    assert_eq!(
        implicit.origin,
        ConfigOrigin::ProjectFile(PathBuf::from("/workspace/repository/.sloc-guard.toml"))
    );
    assert_eq!(
        explicit.origin,
        ConfigOrigin::ExplicitFile(PathBuf::from("/workspace/repository/.sloc-guard.toml"))
    );
}

#[test]
fn default_loader_can_be_created() {
    let _loader = FileConfigLoader::new();
    let _loader_default = FileConfigLoader::default();
}
