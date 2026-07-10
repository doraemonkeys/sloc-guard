use std::path::{Path, PathBuf};

use crate::config::ConfigOrigin;

use super::{ConfigNotice, ConfigNoticeOptions, select_config_notice, select_preset_notice};

const fn options(
    quiet: bool,
    verbose: u8,
    human_text: bool,
    stderr_is_terminal: bool,
) -> ConfigNoticeOptions {
    ConfigNoticeOptions {
        quiet,
        verbose,
        human_text,
        stderr_is_terminal,
    }
}

#[test]
fn built_in_defaults_are_announced_for_interactive_text_output() {
    let notice = select_config_notice(
        &ConfigOrigin::BuiltInDefaults,
        options(false, 0, true, true),
    );

    assert_eq!(notice, Some(ConfigNotice::BuiltInDefaults));
}

#[test]
fn built_in_defaults_are_hidden_for_non_terminal_output() {
    let notice = select_config_notice(
        &ConfigOrigin::BuiltInDefaults,
        options(false, 0, true, false),
    );

    assert_eq!(notice, None);
}

#[test]
fn built_in_defaults_are_hidden_for_structured_output_by_default() {
    let notice = select_config_notice(
        &ConfigOrigin::BuiltInDefaults,
        options(false, 0, false, true),
    );

    assert_eq!(notice, None);
}

#[test]
fn verbose_mode_announces_defaults_for_non_interactive_structured_output() {
    let notice = select_config_notice(
        &ConfigOrigin::BuiltInDefaults,
        options(false, 1, false, false),
    );

    assert_eq!(notice, Some(ConfigNotice::BuiltInDefaults));
}

#[test]
fn quiet_mode_suppresses_defaults_even_when_verbose_and_interactive() {
    let notice = select_config_notice(&ConfigOrigin::BuiltInDefaults, options(true, 2, true, true));

    assert_eq!(notice, None);
}

#[test]
fn disabled_configuration_never_emits_a_notice() {
    for notice_options in [
        options(false, 0, true, true),
        options(false, 2, false, false),
        options(true, 2, true, true),
    ] {
        assert_eq!(
            select_config_notice(&ConfigOrigin::Disabled, notice_options),
            None
        );
    }
}

#[test]
fn file_sources_are_hidden_without_verbose_mode() {
    let origins = [
        ConfigOrigin::ProjectFile(PathBuf::from("project/.sloc-guard.toml")),
        ConfigOrigin::UserFile(PathBuf::from("user/config.toml")),
        ConfigOrigin::ExplicitFile(PathBuf::from("custom.toml")),
    ];

    for origin in &origins {
        assert_eq!(
            select_config_notice(origin, options(false, 0, true, true)),
            None
        );
    }
}

#[test]
fn verbose_mode_reports_every_file_source() {
    let paths = [
        "project/.sloc-guard.toml",
        "user/config.toml",
        "custom.toml",
    ];
    let origins = [
        ConfigOrigin::ProjectFile(PathBuf::from(paths[0])),
        ConfigOrigin::UserFile(PathBuf::from(paths[1])),
        ConfigOrigin::ExplicitFile(PathBuf::from(paths[2])),
    ];

    for (origin, expected_path) in origins.iter().zip(paths) {
        assert_eq!(
            select_config_notice(origin, options(false, 1, false, false)),
            Some(ConfigNotice::File(Path::new(expected_path)))
        );
    }
}

#[test]
fn quiet_mode_suppresses_verbose_file_source() {
    let origin = ConfigOrigin::ProjectFile(PathBuf::from("project/.sloc-guard.toml"));

    assert_eq!(
        select_config_notice(&origin, options(true, 1, true, true)),
        None
    );
}

#[test]
fn default_notice_has_actionable_text() {
    let notice = ConfigNotice::BuiltInDefaults;

    assert_eq!(
        notice.message(),
        "No configuration file found; using built-in defaults."
    );
    assert_eq!(
        notice.suggestion(),
        Some("Run `sloc-guard init` or pass `--config <PATH>` to use a configuration file")
    );
}

#[test]
fn preset_notices_follow_contextual_output_policy() {
    assert_eq!(
        select_preset_notice("rust-strict", options(false, 0, true, true)),
        Some(ConfigNotice::Preset("rust-strict"))
    );
    assert_eq!(
        select_preset_notice("rust-strict", options(false, 0, false, true)),
        None
    );
    assert_eq!(
        select_preset_notice("rust-strict", options(true, 2, true, true)),
        None
    );
}

#[test]
fn preset_notice_has_actionable_text() {
    let notice = ConfigNotice::Preset("rust-strict");

    assert_eq!(notice.message(), "Using preset: rust-strict");
    assert_eq!(
        notice.suggestion(),
        Some("Run `sloc-guard config show` to see effective settings")
    );
}

#[test]
fn file_notice_includes_the_selected_path_without_a_suggestion() {
    let notice = ConfigNotice::File(Path::new("project/.sloc-guard.toml"));

    assert_eq!(
        notice.message(),
        "Using configuration: project/.sloc-guard.toml"
    );
    assert_eq!(notice.suggestion(), None);
}
