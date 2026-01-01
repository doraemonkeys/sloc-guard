use std::collections::HashMap;

use crate::config::CustomLanguageConfig;

/// Pattern kind for dynamic comment/string markers
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum PatternKind {
    /// Static start/end markers (e.g., `/*` and `*/`)
    #[default]
    Static,
    /// Lua long brackets: `--[=*[` with matching `]=*]`
    /// The level (number of `=` signs) is captured dynamically.
    LuaLongBracket,
    /// Rust raw strings: `r#*"` with matching `"#*`
    /// The level (number of `#` signs) is captured dynamically.
    /// This is used to skip raw strings when searching for comment markers.
    RustRawString,
}

/// Metadata for a multi-line comment style
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MultiLineComment {
    pub start: String,
    pub end: String,
    /// Whether this comment style supports nesting (e.g., Rust `/* /* */ */`)
    pub supports_nesting: bool,
    /// Whether the start marker must appear at line start (column 0) after trimming
    /// (e.g., Ruby `=begin`)
    pub must_be_at_line_start: bool,
    /// Pattern kind for dynamic matching
    pub pattern_kind: PatternKind,
}

impl MultiLineComment {
    #[must_use]
    pub fn new(start: &str, end: &str) -> Self {
        Self {
            start: start.to_string(),
            end: end.to_string(),
            supports_nesting: false,
            must_be_at_line_start: false,
            pattern_kind: PatternKind::Static,
        }
    }

    #[must_use]
    pub const fn with_nesting(mut self) -> Self {
        self.supports_nesting = true;
        self
    }

    #[must_use]
    pub const fn at_line_start(mut self) -> Self {
        self.must_be_at_line_start = true;
        self
    }

    #[must_use]
    pub const fn with_pattern_kind(mut self, kind: PatternKind) -> Self {
        self.pattern_kind = kind;
        self
    }
}

/// Helper to create Lua long bracket comment pattern
/// Matches `--[=*[` and dynamically computes `]=*]` end marker
#[derive(Debug, Clone)]
pub struct LuaLongBracket {
    /// If true, requires `--` prefix (comment). If false, matches raw `[=*[` (string).
    pub is_comment: bool,
}

impl LuaLongBracket {
    /// Create a Lua long bracket comment pattern (--[=*[ ... ]=*])
    #[must_use]
    pub const fn comment() -> Self {
        Self { is_comment: true }
    }
}

impl From<LuaLongBracket> for MultiLineComment {
    fn from(lua: LuaLongBracket) -> Self {
        // Use placeholder markers; actual matching is done via PatternKind
        let start = if lua.is_comment { "--[[" } else { "[[" };
        Self {
            start: start.to_string(),
            end: "]]".to_string(),
            supports_nesting: false,
            must_be_at_line_start: false,
            pattern_kind: PatternKind::LuaLongBracket,
        }
    }
}

/// Helper to create Rust raw string pattern
/// Matches `r#*"` and dynamically computes `"#*` end marker
#[derive(Debug, Clone)]
pub struct RustRawString;

impl RustRawString {
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl Default for RustRawString {
    fn default() -> Self {
        Self::new()
    }
}

impl From<RustRawString> for MultiLineComment {
    fn from(_: RustRawString) -> Self {
        // Use placeholder markers; actual matching is done via PatternKind
        Self {
            start: "r\"".to_string(),
            end: "\"".to_string(),
            supports_nesting: false,
            must_be_at_line_start: false,
            pattern_kind: PatternKind::RustRawString,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommentSyntax {
    pub single_line: Vec<String>,
    pub multi_line: Vec<MultiLineComment>,
}

impl CommentSyntax {
    #[must_use]
    pub fn new(single_line: Vec<&str>, multi_line: Vec<(&str, &str)>) -> Self {
        Self {
            single_line: single_line.into_iter().map(String::from).collect(),
            multi_line: multi_line
                .into_iter()
                .map(|(s, e)| MultiLineComment::new(s, e))
                .collect(),
        }
    }

    /// Create with detailed multi-line comment configuration
    #[must_use]
    pub fn with_multi_line(single_line: Vec<&str>, multi_line: Vec<MultiLineComment>) -> Self {
        Self {
            single_line: single_line.into_iter().map(String::from).collect(),
            multi_line,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Language {
    pub name: String,
    pub extensions: Vec<String>,
    pub comment_syntax: CommentSyntax,
}

impl Language {
    #[must_use]
    pub fn new(name: &str, extensions: Vec<&str>, comment_syntax: CommentSyntax) -> Self {
        Self {
            name: name.to_string(),
            extensions: extensions.into_iter().map(String::from).collect(),
            comment_syntax,
        }
    }
}

#[derive(Debug)]
pub struct LanguageRegistry {
    languages: Vec<Language>,
    extension_map: HashMap<String, usize>,
}

impl LanguageRegistry {
    #[must_use]
    pub fn new() -> Self {
        Self {
            languages: Vec::new(),
            extension_map: HashMap::new(),
        }
    }

    pub fn register(&mut self, language: Language) {
        let idx = self.languages.len();
        for ext in &language.extensions {
            // Warn in debug builds if the same extension appears multiple times
            // within a single language definition (likely a typo)
            debug_assert!(
                !language.extensions.iter().filter(|e| *e == ext).count() > 1,
                "Duplicate extension '{ext}' within language '{}'",
                language.name
            );
            self.extension_map.insert(ext.clone(), idx);
        }
        self.languages.push(language);
    }

    #[must_use]
    pub fn get_by_extension(&self, ext: &str) -> Option<&Language> {
        self.extension_map.get(ext).map(|&idx| &self.languages[idx])
    }

    #[must_use]
    pub fn all(&self) -> &[Language] {
        &self.languages
    }

    /// Create a registry with built-in languages plus custom language definitions.
    ///
    /// # Extension Override Behavior
    /// Custom languages are registered **after** built-ins. If a custom language uses
    /// an extension already registered by a built-in (e.g., `.rs` for Rust), the custom
    /// definition **silently overrides** the built-in. This is intentional—it allows users
    /// to customize comment syntax for specific extensions when needed.
    ///
    /// Use [`with_custom_languages_checked`] if you need to detect overrides.
    #[must_use]
    pub fn with_custom_languages(custom: &HashMap<String, CustomLanguageConfig>) -> Self {
        Self::with_custom_languages_checked(custom).0
    }

    /// Create a registry with built-in languages plus custom language definitions,
    /// returning a list of extensions that were overridden.
    ///
    /// # Returns
    /// A tuple of `(registry, overridden_extensions)` where `overridden_extensions`
    /// contains `(extension, original_language_name, new_language_name)` for each
    /// extension that was remapped from a built-in to a custom language.
    #[must_use]
    pub fn with_custom_languages_checked(
        custom: &HashMap<String, CustomLanguageConfig>,
    ) -> (Self, Vec<(String, String, String)>) {
        let mut registry = Self::default();
        let mut overrides = Vec::new();

        for (name, config) in custom {
            // Track which extensions will be overridden
            for ext in &config.extensions {
                if let Some(existing_lang) = registry.get_by_extension(ext) {
                    overrides.push((ext.clone(), existing_lang.name.clone(), name.clone()));
                }
            }

            let syntax = CommentSyntax {
                single_line: config.single_line_comments.clone(),
                multi_line: config
                    .multi_line_comments
                    .iter()
                    .map(|(s, e)| MultiLineComment::new(s, e))
                    .collect(),
            };
            let language = Language {
                name: name.clone(),
                extensions: config.extensions.clone(),
                comment_syntax: syntax,
            };
            registry.register(language);
        }

        (registry, overrides)
    }
}

impl Default for LanguageRegistry {
    fn default() -> Self {
        let mut registry = Self::new();

        // Rust supports nested block comments and raw strings
        registry.register(Language::new(
            "Rust",
            vec!["rs"],
            CommentSyntax::with_multi_line(
                vec!["//", "///", "//!"],
                vec![
                    MultiLineComment::new("/*", "*/").with_nesting(),
                    RustRawString::new().into(),
                ],
            ),
        ));

        registry.register(Language::new(
            "Go",
            vec!["go"],
            CommentSyntax::new(vec!["//"], vec![("/*", "*/")]),
        ));

        registry.register(Language::new(
            "Python",
            vec!["py", "pyi"],
            CommentSyntax::new(vec!["#"], vec![("'''", "'''"), ("\"\"\"", "\"\"\"")]),
        ));

        registry.register(Language::new(
            "JavaScript",
            vec!["js", "mjs", "cjs"],
            CommentSyntax::new(vec!["//"], vec![("/*", "*/")]),
        ));

        registry.register(Language::new(
            "TypeScript",
            vec!["ts", "mts", "cts", "tsx"],
            CommentSyntax::new(vec!["//"], vec![("/*", "*/")]),
        ));

        registry.register(Language::new(
            "C",
            vec!["c", "h"],
            CommentSyntax::new(vec!["//"], vec![("/*", "*/")]),
        ));

        registry.register(Language::new(
            "C++",
            vec!["cpp", "hpp", "cc", "cxx", "hxx"],
            CommentSyntax::new(vec!["//"], vec![("/*", "*/")]),
        ));

        registry.register(Language::new(
            "Java",
            vec!["java"],
            CommentSyntax::new(vec!["//"], vec![("/*", "*/")]),
        ));

        registry.register(Language::new(
            "Kotlin",
            vec!["kt", "kts"],
            CommentSyntax::new(vec!["//"], vec![("/*", "*/")]),
        ));

        // Swift supports nested block comments
        registry.register(Language::new(
            "Swift",
            vec!["swift"],
            CommentSyntax::with_multi_line(
                vec!["//", "///"],
                vec![MultiLineComment::new("/*", "*/").with_nesting()],
            ),
        ));

        registry.register(Language::new(
            "Dart",
            vec!["dart"],
            CommentSyntax::new(vec!["//", "///"], vec![("/*", "*/")]),
        ));

        registry.register(Language::new(
            "C#",
            vec!["cs"],
            CommentSyntax::new(vec!["//", "///"], vec![("/*", "*/")]),
        ));

        registry.register(Language::new(
            "PHP",
            vec!["php"],
            CommentSyntax::new(vec!["//", "#"], vec![("/*", "*/")]),
        ));

        // Ruby =begin/=end must be at line start (column 0)
        registry.register(Language::new(
            "Ruby",
            vec!["rb", "rake"],
            CommentSyntax::with_multi_line(
                vec!["#"],
                vec![MultiLineComment::new("=begin", "=end").at_line_start()],
            ),
        ));

        registry.register(Language::new(
            "Shell",
            vec!["sh", "bash", "zsh"],
            CommentSyntax::new(vec!["#"], vec![]),
        ));

        registry.register(Language::new(
            "Scala",
            vec!["scala", "sc"],
            CommentSyntax::new(vec!["//"], vec![("/*", "*/")]),
        ));

        // Lua supports long brackets with varying levels: --[[ ]], --[=[ ]=], --[==[ ]==], etc.
        registry.register(Language::new(
            "Lua",
            vec!["lua"],
            CommentSyntax::with_multi_line(vec!["--"], vec![LuaLongBracket::comment().into()]),
        ));

        registry.register(Language::new(
            "SQL",
            vec!["sql"],
            CommentSyntax::new(vec!["--"], vec![("/*", "*/")]),
        ));

        registry.register(Language::new(
            "Vue",
            vec!["vue"],
            CommentSyntax::new(vec!["//"], vec![("/*", "*/"), ("<!--", "-->")]),
        ));

        registry.register(Language::new(
            "JSX",
            vec!["jsx"],
            CommentSyntax::new(vec!["//"], vec![("/*", "*/")]),
        ));

        registry
    }
}

#[cfg(test)]
#[path = "registry_tests.rs"]
mod tests;
