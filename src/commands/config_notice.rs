//! Configuration-source notices for command-line output.
//!
//! The configuration loader reports facts through [`ConfigOrigin`]. This module
//! owns the presentation policy so loading remains free of terminal side effects.

use std::io::IsTerminal;
use std::path::Path;

use crate::config::ConfigOrigin;
use crate::output::{ColorMode, ErrorOutput};

const DEFAULT_CONFIG_HELP: &str =
    "Run `sloc-guard init` or pass `--config <PATH>` to use a configuration file";

/// Output conditions that determine whether configuration-source information is useful.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConfigNoticeOptions {
    /// Suppress all non-essential output. This takes precedence over verbosity.
    pub quiet: bool,
    /// CLI verbosity level.
    pub verbose: u8,
    /// Whether the command's primary output is human-readable text.
    pub human_text: bool,
    /// Whether stderr is attached to an interactive terminal.
    pub stderr_is_terminal: bool,
}

/// A configuration notice selected for display.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigNotice<'a> {
    /// No configuration file was found, so built-in defaults are active.
    BuiltInDefaults,
    /// A configuration file is active and its path should be shown in verbose output.
    File(&'a Path),
    /// A named preset contributes to the effective configuration.
    Preset(&'a str),
}

impl ConfigNotice<'_> {
    fn message(self) -> String {
        match self {
            Self::BuiltInDefaults => {
                "No configuration file found; using built-in defaults.".to_string()
            }
            Self::File(path) => format!("Using configuration: {}", path.display()),
            Self::Preset(name) => format!("Using preset: {name}"),
        }
    }

    const fn suggestion(self) -> Option<&'static str> {
        match self {
            Self::BuiltInDefaults => Some(DEFAULT_CONFIG_HELP),
            Self::Preset(_) => Some("Run `sloc-guard config show` to see effective settings"),
            Self::File(_) => None,
        }
    }
}

const fn contextual_notice_is_visible(options: ConfigNoticeOptions) -> bool {
    !options.quiet && (options.verbose > 0 || (options.human_text && options.stderr_is_terminal))
}

/// Select a notice without performing I/O.
///
/// Quiet mode always wins. Built-in defaults are announced for interactive text
/// output or when verbosity is explicitly requested. File-backed sources are
/// shown only in verbose mode. An explicit `--no-config` choice is never announced.
#[must_use]
pub fn select_config_notice(
    origin: &ConfigOrigin,
    options: ConfigNoticeOptions,
) -> Option<ConfigNotice<'_>> {
    if options.quiet {
        return None;
    }

    match origin {
        ConfigOrigin::Disabled => None,
        ConfigOrigin::BuiltInDefaults => {
            contextual_notice_is_visible(options).then_some(ConfigNotice::BuiltInDefaults)
        }
        ConfigOrigin::ProjectFile(path)
        | ConfigOrigin::UserFile(path)
        | ConfigOrigin::ExplicitFile(path) => {
            (options.verbose > 0).then_some(ConfigNotice::File(path))
        }
    }
}

/// Select a preset notice using the same contextual-output policy as the defaults notice.
#[must_use]
pub fn select_preset_notice(
    preset_name: &str,
    options: ConfigNoticeOptions,
) -> Option<ConfigNotice<'_>> {
    contextual_notice_is_visible(options).then_some(ConfigNotice::Preset(preset_name))
}

fn options(quiet: bool, verbose: u8, human_text: bool) -> ConfigNoticeOptions {
    ConfigNoticeOptions {
        quiet,
        verbose,
        human_text,
        stderr_is_terminal: std::io::stderr().is_terminal(),
    }
}

fn print_notice(notice: ConfigNotice<'_>, color_mode: ColorMode) {
    ErrorOutput::new(color_mode).print_info_with_detail(
        &notice.message(),
        None,
        notice.suggestion(),
    );
}

/// Print the selected configuration notice to stderr, if any.
///
/// This is intentionally a thin command-layer adapter. Configuration discovery
/// and loading must not call it directly.
pub fn print_config_notice(
    origin: &ConfigOrigin,
    quiet: bool,
    verbose: u8,
    human_text: bool,
    color_mode: ColorMode,
) {
    if let Some(notice) = select_config_notice(origin, options(quiet, verbose, human_text)) {
        print_notice(notice, color_mode);
    }
}

/// Print preset provenance to stderr when contextual notices are visible.
pub fn print_preset_notice(
    preset_name: &str,
    quiet: bool,
    verbose: u8,
    human_text: bool,
    color_mode: ColorMode,
) {
    if let Some(notice) = select_preset_notice(preset_name, options(quiet, verbose, human_text)) {
        print_notice(notice, color_mode);
    }
}

#[cfg(test)]
#[path = "config_notice_tests.rs"]
mod tests;
