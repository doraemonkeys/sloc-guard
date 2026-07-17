//! Stable path semantics for project-relative configuration and rule matching.
//!
//! [`ProjectPaths`] separates physical paths (used for filesystem access) from logical paths
//! (used for configuration matching, output, and persistent identity). This keeps rule behavior
//! stable when the command is invoked from a subdirectory of the configuration root.

mod logical_glob;

use std::path::{Path, PathBuf};

pub(crate) use logical_glob::{
    ScopeMatcher, compile_logical_path_glob, matching_logical_path_globs, normalize_for_matching,
    normalize_pattern_for_matching,
};

/// Resolve `path` against `cwd` without touching the filesystem.
///
/// Unlike canonicalization, this only removes lexical `.`/`..` components and therefore preserves
/// the location of a symlink supplied by the caller. Configuration loading uses this distinction so
/// relative `extends` paths and rule roots are anchored at the selected config path, not at the
/// symlink target.
#[must_use]
pub(crate) fn lexical_absolute(path: &Path, cwd: &Path) -> PathBuf {
    LexicalPath::parse(cwd)
        .resolve(&LexicalPath::parse(path))
        .to_path_buf()
}

/// Shared identity mapping for scanner/filter implementations without a project context.
pub(crate) static UNROOTED_PROJECT_PATHS: ProjectPaths = ProjectPaths::unrooted();

/// Converts between invocation-relative physical paths and configuration-root-relative paths.
///
/// An unrooted context is an identity mapping. It exists for callers that do not have a project
/// context and preserves the behavior of existing unit-level scanner and checker APIs.
#[derive(Clone, Debug)]
pub struct ProjectPaths {
    config_root: Option<PathBuf>,
    invocation_cwd: Option<PathBuf>,
}

impl ProjectPaths {
    /// Create a rooted context, capturing the process working directory as the invocation root.
    #[must_use]
    pub fn rooted(config_root: PathBuf) -> Self {
        let invocation_cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        Self::rooted_with_cwd(config_root, invocation_cwd)
    }

    /// Create a rooted context with an explicit invocation working directory.
    ///
    /// Relative configuration roots are resolved against `invocation_cwd`. A relative
    /// `invocation_cwd` is first resolved against the process working directory.
    #[must_use]
    pub fn rooted_with_cwd(config_root: PathBuf, invocation_cwd: PathBuf) -> Self {
        let process_cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let process_cwd = LexicalPath::parse(&process_cwd);
        let invocation_cwd = process_cwd.resolve(&LexicalPath::parse(invocation_cwd));
        let config_root = invocation_cwd.resolve(&LexicalPath::parse(config_root));

        Self {
            config_root: Some(config_root.to_path_buf()),
            invocation_cwd: Some(invocation_cwd.to_path_buf()),
        }
    }

    /// Create an identity context for callers without a stable configuration root.
    #[must_use]
    pub const fn unrooted() -> Self {
        Self {
            config_root: None,
            invocation_cwd: None,
        }
    }

    /// Return the configuration root, if this context is rooted.
    #[must_use]
    pub fn config_root(&self) -> Option<&Path> {
        self.config_root.as_deref()
    }

    /// Return the working directory from which relative physical paths are interpreted.
    #[must_use]
    pub fn invocation_cwd(&self) -> Option<&Path> {
        self.invocation_cwd.as_deref()
    }

    /// Return whether this context has a stable configuration root.
    #[must_use]
    pub const fn is_rooted(&self) -> bool {
        self.config_root.is_some()
    }

    /// Map a physical path to its configuration-root-relative logical identity.
    ///
    /// Relative physical paths are interpreted relative to the invocation working directory.
    /// Paths outside the root use `..` components when both paths share an anchor. A path on a
    /// different Windows drive cannot be represented relative to the root and remains absolute.
    /// The configuration root itself is represented as `.`.
    #[must_use]
    pub fn logical(&self, path: &Path) -> PathBuf {
        let (Some(config_root), Some(invocation_cwd)) = (&self.config_root, &self.invocation_cwd)
        else {
            return path.to_path_buf();
        };

        let config_root = LexicalPath::parse(config_root);
        let invocation_cwd = LexicalPath::parse(invocation_cwd);
        let physical = invocation_cwd.resolve(&LexicalPath::parse(path));
        let logical = physical.relative_to(&config_root).to_path_buf();
        if logical.as_os_str().is_empty() {
            PathBuf::from(".")
        } else {
            logical
        }
    }

    /// Map a configuration-root-relative logical path back to a physical path.
    ///
    /// Absolute inputs are already physical and are returned in lexically normalized form.
    #[must_use]
    pub fn physical(&self, path: &Path) -> PathBuf {
        let Some(config_root) = &self.config_root else {
            return path.to_path_buf();
        };

        LexicalPath::parse(config_root)
            .resolve(&LexicalPath::parse(path))
            .to_path_buf()
    }

    /// Return the number of logical components below the configuration root.
    ///
    /// The configuration root itself has depth zero. For paths outside the root, leading `..`
    /// components contribute to the depth just like ordinary logical components.
    #[must_use]
    pub fn logical_depth(&self, path: &Path) -> usize {
        LexicalPath::parse(self.logical(path)).components.len()
    }
}

impl Default for ProjectPaths {
    fn default() -> Self {
        Self::unrooted()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum Anchor {
    Relative,
    UnixRoot,
    WindowsRoot,
    Drive(String),
    Unc { server: String, share: String },
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct LexicalPath {
    anchor: Anchor,
    rooted: bool,
    components: Vec<String>,
}

impl LexicalPath {
    fn parse(path: impl AsRef<Path>) -> Self {
        let path = path.as_ref();
        let raw = path.to_string_lossy();
        let windows_rooted = raw.starts_with('\\') && !raw.starts_with("\\\\");
        let normalized = normalize_for_matching(path);
        let normalized = normalized.to_string_lossy();
        let value = normalized.as_ref();

        if let Some(unc) = value.strip_prefix("//") {
            let mut parts = unc.split('/').filter(|part| !part.is_empty());
            if let (Some(server), Some(share)) = (parts.next(), parts.next()) {
                return Self::from_parts(
                    Anchor::Unc {
                        server: server.to_string(),
                        share: share.to_string(),
                    },
                    true,
                    parts,
                );
            }
        }

        let bytes = value.as_bytes();
        if bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':' {
            let rooted = bytes.get(2) == Some(&b'/');
            let remainder = if rooted { &value[3..] } else { &value[2..] };
            return Self::from_parts(
                Anchor::Drive(value[..2].to_string()),
                rooted,
                remainder.split('/'),
            );
        }

        if let Some(remainder) = value.strip_prefix('/') {
            let anchor = if windows_rooted {
                Anchor::WindowsRoot
            } else {
                Anchor::UnixRoot
            };
            return Self::from_parts(anchor, true, remainder.split('/'));
        }

        Self::from_parts(Anchor::Relative, false, value.split('/'))
    }

    fn from_parts<'a>(
        anchor: Anchor,
        rooted: bool,
        parts: impl IntoIterator<Item = &'a str>,
    ) -> Self {
        let mut components = Vec::new();
        for part in parts {
            match part {
                ".." if components.last().is_some_and(|last| last != "..") => {
                    components.pop();
                }
                ".." if !rooted => components.push(part.to_string()),
                "" | "." | ".." => {}
                _ => components.push(part.to_string()),
            }
        }

        Self {
            anchor,
            rooted,
            components,
        }
    }

    fn resolve(&self, path: &Self) -> Self {
        let relative_components = match (&self.anchor, &path.anchor) {
            (Anchor::Drive(drive), Anchor::WindowsRoot) if path.rooted => {
                return Self {
                    anchor: Anchor::Drive(drive.clone()),
                    rooted: true,
                    components: path.components.clone(),
                };
            }
            (Anchor::Drive(base), Anchor::Drive(relative))
                if !path.rooted && base.eq_ignore_ascii_case(relative) =>
            {
                &path.components
            }
            (_, Anchor::Relative) if !path.rooted => &path.components,
            _ => return path.clone(),
        };

        let mut resolved = self.clone();
        for component in relative_components {
            if component == ".." {
                if resolved.components.last().is_some_and(|last| last != "..") {
                    resolved.components.pop();
                } else if !resolved.rooted {
                    resolved.components.push(component.clone());
                }
            } else {
                resolved.components.push(component.clone());
            }
        }
        resolved
    }

    fn relative_to(&self, root: &Self) -> Self {
        if !self.same_anchor(root) {
            return self.clone();
        }

        let windows_semantics = matches!(&self.anchor, Anchor::Drive(_) | Anchor::Unc { .. });
        let common = self
            .components
            .iter()
            .zip(&root.components)
            .take_while(|(left, right)| component_eq(left, right, windows_semantics))
            .count();

        let mut components = vec!["..".to_string(); root.components.len() - common];
        components.extend(self.components[common..].iter().cloned());
        Self {
            anchor: Anchor::Relative,
            rooted: false,
            components,
        }
    }

    fn same_anchor(&self, other: &Self) -> bool {
        match (&self.anchor, &other.anchor) {
            (Anchor::Relative, Anchor::Relative)
            | (Anchor::UnixRoot, Anchor::UnixRoot)
            | (Anchor::WindowsRoot, Anchor::WindowsRoot) => true,
            (Anchor::Drive(left), Anchor::Drive(right)) => {
                self.rooted == other.rooted && left.eq_ignore_ascii_case(right)
            }
            (
                Anchor::Unc {
                    server: left_server,
                    share: left_share,
                },
                Anchor::Unc {
                    server: right_server,
                    share: right_share,
                },
            ) => {
                left_server.eq_ignore_ascii_case(right_server)
                    && left_share.eq_ignore_ascii_case(right_share)
            }
            _ => false,
        }
    }

    fn to_path_buf(&self) -> PathBuf {
        let body = self.components.join("/");
        let rendered = match &self.anchor {
            Anchor::Relative => body,
            Anchor::UnixRoot | Anchor::WindowsRoot if body.is_empty() => "/".to_string(),
            Anchor::UnixRoot | Anchor::WindowsRoot => format!("/{body}"),
            Anchor::Drive(drive) if self.rooted && body.is_empty() => format!("{drive}/"),
            Anchor::Drive(drive) if self.rooted => format!("{drive}/{body}"),
            Anchor::Drive(drive) => format!("{drive}{body}"),
            Anchor::Unc { server, share } if body.is_empty() => format!("//{server}/{share}"),
            Anchor::Unc { server, share } => format!("//{server}/{share}/{body}"),
        };
        PathBuf::from(rendered)
    }
}

fn component_eq(left: &str, right: &str, windows_semantics: bool) -> bool {
    if windows_semantics {
        left.eq_ignore_ascii_case(right)
    } else {
        left == right
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn child_invocation_maps_to_config_root_relative_logical_path() {
        let paths =
            ProjectPaths::rooted_with_cwd(PathBuf::from("/repo"), PathBuf::from("/repo/core"));

        assert_eq!(
            paths.logical(Path::new("./session/receive_test.go")),
            PathBuf::from("core/session/receive_test.go")
        );
        assert_eq!(
            paths.physical(Path::new("core/session/receive_test.go")),
            PathBuf::from("/repo/core/session/receive_test.go")
        );
    }

    #[test]
    fn unix_paths_are_lexically_normalized() {
        let paths = ProjectPaths::rooted_with_cwd(
            PathBuf::from("/repo/project/./"),
            PathBuf::from("/repo/project/core/../core"),
        );

        assert_eq!(
            paths.logical(Path::new("./session/generated/../receive_test.go")),
            PathBuf::from("core/session/receive_test.go")
        );
        assert_eq!(paths.config_root(), Some(Path::new("/repo/project")));
        assert_eq!(
            paths.invocation_cwd(),
            Some(Path::new("/repo/project/core"))
        );
    }

    #[test]
    fn windows_paths_are_normalized_independently_of_host_platform() {
        let paths = ProjectPaths::rooted_with_cwd(
            PathBuf::from(r"C:\repo"),
            PathBuf::from(r"C:\repo\core"),
        );

        assert_eq!(
            paths.logical(Path::new(r".\session\receive_test.go")),
            PathBuf::from("core/session/receive_test.go")
        );
        assert_eq!(
            paths.physical(Path::new("core/session/receive_test.go")),
            PathBuf::from("C:/repo/core/session/receive_test.go")
        );
    }

    #[test]
    fn config_root_itself_has_dot_logical_path_and_zero_depth() {
        let paths = ProjectPaths::rooted_with_cwd(
            PathBuf::from("/repo/project"),
            PathBuf::from("/repo/project/core"),
        );

        assert_eq!(
            paths.logical(Path::new("/repo/project")),
            PathBuf::from(".")
        );
        assert_eq!(paths.logical_depth(Path::new("/repo/project")), 0);
        assert_eq!(paths.logical_depth(Path::new("/repo/project/src/api")), 2);
    }

    #[test]
    fn path_outside_root_uses_parent_components_and_round_trips() {
        let paths = ProjectPaths::rooted_with_cwd(
            PathBuf::from("/repo/project"),
            PathBuf::from("/repo/project/core"),
        );
        let physical = Path::new("/repo/shared/types.rs");
        let logical = paths.logical(physical);

        assert_eq!(logical, PathBuf::from("../shared/types.rs"));
        assert_eq!(paths.physical(&logical), physical);
        assert_eq!(paths.logical_depth(physical), 3);
    }

    #[test]
    fn windows_path_on_another_drive_stays_absolute() {
        let paths = ProjectPaths::rooted_with_cwd(
            PathBuf::from(r"C:\repo"),
            PathBuf::from(r"C:\repo\core"),
        );
        let physical = Path::new(r"D:\shared\types.rs");
        let logical = paths.logical(physical);

        assert_eq!(logical, PathBuf::from("D:/shared/types.rs"));
        assert_eq!(
            paths.physical(&logical),
            PathBuf::from("D:/shared/types.rs")
        );
    }

    #[test]
    fn windows_anchor_and_components_are_ascii_case_insensitive() {
        let paths = ProjectPaths::rooted_with_cwd(
            PathBuf::from(r"C:\Repo"),
            PathBuf::from(r"c:\repo\Core"),
        );

        assert_eq!(
            paths.logical(Path::new(r"C:\REPO\core\src\lib.rs")),
            PathBuf::from("core/src/lib.rs")
        );
    }

    #[test]
    fn windows_root_relative_path_uses_invocation_drive() {
        let paths = ProjectPaths::rooted_with_cwd(
            PathBuf::from(r"C:\repo"),
            PathBuf::from(r"C:\repo\core"),
        );

        assert_eq!(
            paths.logical(Path::new(r"\repo\src\lib.rs")),
            PathBuf::from("src/lib.rs")
        );
    }

    #[test]
    fn windows_drive_relative_path_uses_matching_drive_cwd() {
        let paths = ProjectPaths::rooted_with_cwd(
            PathBuf::from(r"C:\repo"),
            PathBuf::from(r"C:\repo\core"),
        );

        assert_eq!(
            paths.logical(Path::new(r"C:session\receive_test.go")),
            PathBuf::from("core/session/receive_test.go")
        );
    }

    #[test]
    fn unc_paths_round_trip() {
        let paths = ProjectPaths::rooted_with_cwd(
            PathBuf::from(r"\\server\share\repo"),
            PathBuf::from(r"\\server\share\repo\core"),
        );
        let logical = paths.logical(Path::new(r".\src\lib.rs"));

        assert_eq!(logical, PathBuf::from("core/src/lib.rs"));
        assert_eq!(
            paths.physical(&logical),
            PathBuf::from("//server/share/repo/core/src/lib.rs")
        );
    }

    #[test]
    fn relative_config_root_is_resolved_from_invocation_cwd() {
        let paths =
            ProjectPaths::rooted_with_cwd(PathBuf::from(".."), PathBuf::from("/repo/project/core"));

        assert_eq!(paths.config_root(), Some(Path::new("/repo/project")));
        assert_eq!(
            paths.logical(Path::new("./session/file.rs")),
            PathBuf::from("core/session/file.rs")
        );
    }

    #[test]
    fn unrooted_and_default_are_identity_contexts() {
        let input = Path::new(r".\src\generated\..\lib.rs");
        for paths in [ProjectPaths::unrooted(), ProjectPaths::default()] {
            assert!(!paths.is_rooted());
            assert_eq!(paths.config_root(), None);
            assert_eq!(paths.invocation_cwd(), None);
            assert_eq!(paths.logical(input), input);
            assert_eq!(paths.physical(input), input);
        }
    }
}
