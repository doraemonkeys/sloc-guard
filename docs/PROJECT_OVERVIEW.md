# sloc-guard Project Overview

> **Doc Maintenance**: Keep concise, avoid redundancy, clean up outdated content promptly to reduce AI context usage.
> **Scope**: This document reflects the current codebase state only and does not describe future plans.

**SLOC (Source Lines of Code) enforcement tool** - enforces file size limits by counting code lines (excluding comments and blanks) and enforces directory structure limits (file/folder counts).

## Quick Reference

```
Rust CLI tool | Clap v4 | TOML config | Exit: 0=pass, 1=threshold exceeded, 2=config error
```

## Module Map

| Module | Purpose |
|--------|---------|
| `cli` | Clap CLI: `check` (with `--files`, `--diff`, `--staged`), `stats`, `init` (with `--detect`), `config`, `explain` commands; global flags: `--offline`, `--no-config`, `--no-extends` |
| `config/*` | `Config` (v2: scanner/content/structure separation), `ContentConfig`, `StructureConfig`; loader with `extends` inheritance (local/remote/preset); presets module (rust-strict, node-strict, python-strict, monorepo-base); remote fetching (1h TTL cache in `.sloc-guard/remote-cache/`, `--offline` mode, `extends_sha256` hash verification); `expires.rs`: date parsing/validation |
| `language/registry` | `LanguageRegistry`, `Language`, `CommentSyntax` - predefined + custom via [languages.<name>] config |
| `counter/*` | `CommentDetector`, `SlocCounter` → `CountResult{Stats, IgnoredFile}`, inline ignore directives |
| `scanner/*` | `FileScanner` trait (`scan()`, `scan_with_structure()`); `types.rs`: `ScanResult`, `AllowlistRule`, `StructureScanConfig`; `directory.rs`: `DirectoryScanner` (walkdir + optional .gitignore via `ignore` crate); `gitignore.rs`: `GitAwareScanner` (gix with .gitignore); `composite.rs`: `CompositeScanner` (git/non-git fallback), `scan_files()`; `filter.rs`: `GlobFilter` |
| `checker/*` | `Checker` trait; `result.rs`: `CheckResult` enum; `threshold.rs`: `ThresholdChecker` with pre-indexed extension lookup; `explain.rs`: `ContentExplanation`, `StructureExplanation` for rule chain debugging; `structure/`: `StructureChecker` (split into `builder.rs`, `compiled_rules.rs`, `validation.rs`, `violation.rs`) |
| `git/diff` | `GitDiff` - gix-based diff between committed trees (`--diff ref` or `--diff base..target` for explicit range) and staged files detection (`--staged` mode) |
| `baseline`/`cache` | `Baseline` V2 (Content/Structure entries, V1 auto-migration), `Cache` (mtime+size validation, file locking for concurrent access) |
| `state` | State file path resolution: `discover_project_root()` (walks up to find `.git/` or `.sloc-guard.toml`), `cache_path()`, `history_path()`, `baseline_path()` → `.git/sloc-guard/` (git repo) or `.sloc-guard/` (fallback); file locking utilities (`try_lock_exclusive_with_timeout`, `try_lock_shared_with_timeout`) for concurrent access protection |
| `output/*` | `TextFormatter`, `JsonFormatter`, `SarifFormatter`, `MarkdownFormatter`, `HtmlFormatter`; `StatsTextFormatter`, `StatsJsonFormatter`, `StatsMarkdownFormatter`; `ScanProgress` (progress bar) |
| `path_utils` | `path_matches_override()` - shared path suffix matching for override path resolution (handles Windows/Unix separators) |
| `error` | `SlocGuardError` |
| `commands/*` | `run_check`, `run_stats`, `run_config`, `run_init`, `run_explain`; check split into: `check_baseline_ops.rs`, `check_git_diff.rs`, `check_output.rs`, `check_processing.rs`, `check_validation.rs`; `context.rs`: `CheckContext`/`StatsContext` for DI; `detect.rs`: project type auto-detection |
| `analyzer` | `FunctionParser` - multi-language split suggestions (--suggest) |
| `stats` | `TrendHistory` - historical stats with delta computation, file locking for concurrent access |
| `main` | CLI parsing, command dispatch to `commands/*` |

## Key Types

```rust
// Config (priority: CLI > file > defaults; extends: local/remote/preset with 1h TTL cache for remote)
// V2 schema separates scanner/content/structure concerns
// Presets: extends = "preset:rust-strict|node-strict|python-strict|monorepo-base"
// Hash Lock: extends_sha256 = "<sha256>" verifies remote config integrity
Config { version, extends, extends_sha256, scanner, content, structure }
ScannerConfig { gitignore: true, exclude: Vec<glob> }  // Physical discovery, no extension filter
ContentConfig { extensions, max_lines, warn_threshold, skip_comments, skip_blank, exclude, rules, languages }  // exclude: glob patterns to skip SLOC but keep for structure
ContentRule { pattern, max_lines, warn_threshold, skip_comments, skip_blank, reason, expires }  // [[content.rules]]
StructureConfig { max_files, max_dirs, max_depth, warn_threshold, warn_files_at, warn_dirs_at, warn_files_threshold, warn_dirs_threshold, count_exclude, allow_extensions, allow_files, allow_dirs, deny_extensions, deny_patterns, deny_files, deny_dirs, rules }
StructureRule { scope, max_files, max_dirs, max_depth, relative_depth, warn_threshold, warn_files_at, warn_dirs_at, warn_files_threshold, warn_dirs_threshold, allow_extensions, allow_patterns, allow_files, allow_dirs, deny_extensions, deny_patterns, deny_files, deny_dirs, file_naming_pattern, file_pattern, require_sibling, reason, expires }  // [[structure.rules]]
CustomLanguageConfig { extensions, single_line_comments, multi_line_comments }

// Line counting (ignore directives: ignore-file, ignore-next N, ignore-start/end)
LineStats { total, code, comment, blank, ignored }
CountResult::Stats(LineStats) | IgnoredFile
CommentSyntax { single_line, multi_line }

// Check results (enum with associated data)
CheckResult::Passed { path, stats, limit, override_reason, violation_category }
          | Warning { ..., suggestions }
          | Failed { ..., suggestions }
          | Grandfathered { ... }
ViolationCategory::Content | Structure { violation_type, triggering_rule }  // distinguishes SLOC vs structure violations
// Accessor methods: path(), stats(), limit(), override_reason(), suggestions(), violation_category()
// Consuming: into_grandfathered(), with_suggestions()

// Structure checking
DirStats { file_count, dir_count, depth }  // immediate children counts + depth from scan root
ViolationType::FileCount | DirCount | MaxDepth | DisallowedFile | DisallowedDirectory | DeniedFile { pattern_or_extension } | DeniedDirectory { pattern } | NamingConvention { expected_pattern } | MissingSibling { expected_sibling_pattern }
StructureViolation { path, violation_type, actual, limit, is_warning, override_reason, triggering_rule_pattern }

// Explain (rule chain debugging)
MatchStatus::Matched | Superseded | NoMatch
ContentRuleMatch::Excluded { pattern } | Rule { index, pattern, reason } | Default
ContentExplanation { path, is_excluded, matched_rule, effective_limit, warn_threshold, skip_*, rule_chain }
StructureRuleMatch::Rule { index, pattern, reason } | Default
StructureExplanation { path, matched_rule, effective_max_files, effective_max_dirs, effective_max_depth, warn_threshold, rule_chain }

// Output
OutputFormat::Text | Json | Sarif | Markdown | Html
ColorMode::Auto | Always | Never

// Stats
FileStatistics { path, stats, language }
ProjectStatistics { files, total_*, by_language, by_directory, top_files, average_code_lines, trend }
GroupBy::None | Lang | Dir

// Trend (state::history_path() → .git/sloc-guard/history.json or .sloc-guard/history.json)
TrendEntry { timestamp, total_files, total_lines, code, comment, blank }
TrendDelta { *_delta, previous_timestamp }

// Git/Baseline/Cache
GitDiff::get_changed_files(base_ref) → HashSet<PathBuf>  // --diff ref (compares to HEAD)
GitDiff::get_changed_files_range(base, target) → HashSet<PathBuf>  // --diff base..target
GitDiff::get_staged_files() → HashSet<PathBuf>  // --staged mode
// Baseline V2 (.sloc-guard-baseline.json in project root)
Baseline { version: 2, files: HashMap<path, BaselineEntry> }
BaselineEntry::Content { lines, hash } | Structure { violation_type, count }
StructureViolationType::Files | Dirs
BaselineUpdateMode::All | Content | Structure | New  // --update-baseline mode
// Cache (state::cache_path() → .git/sloc-guard/cache.json or .sloc-guard/cache.json)
Cache { version, config_hash, files: HashMap<path, CacheEntry{hash, stats, mtime, size}> }
// File locking (state module) - prevents concurrent access corruption
LockError::Timeout | Io(io::Error)  // try_lock_*_with_timeout returns Result<(), LockError>

// Split suggestions (--suggest)
FunctionInfo { name, start_line, end_line, line_count }
SplitSuggestion { original_path, total_lines, limit, functions, chunks }
FunctionParser: Rust, Go, Python, JS/TS, C/C++

// Context for DI (commands/context.rs)
FileReader trait { read(), metadata() }  // IO abstraction for file reading
RealFileReader  // Production impl using std::fs
FileScanner trait { scan(), scan_all(), scan_with_structure(), scan_all_with_structure() }  // IO abstraction for directory traversal
ScanResult { files, dir_stats, allowlist_violations }  // Unified scan output
StructureScanConfig { count_exclude, scanner_exclude, scanner_exclude_dir_names, allowlist_rules, global_allow_extensions, global_allow_files, global_allow_dirs, global_deny_extensions, global_deny_patterns, global_deny_files }  // Config for structure-aware scanning
AllowlistRule { scope, allow_extensions, allow_patterns, allow_files, allow_dirs, deny_extensions, deny_patterns, deny_files, naming_pattern_str }  // Directory allowlist matching
CompositeScanner  // Production impl with git/non-git fallback
CheckContext { registry, threshold_checker, structure_checker, structure_scan_config, scanner, file_reader }  // from_config() or new()
CheckOptions { args, cli, paths, config, ctx, cache, baseline, project_root }  // Encapsulates run_check_with_context params
StatsContext { registry, allowed_extensions }  // from_config() or new()

// Project Detection (init --detect)
ProjectType::Rust | Node | Go | Python | Java | CSharp | Unknown  // auto-detected from marker files
DetectedProject { path, project_type }  // subproject in monorepo
DetectionResult { root, subprojects, is_monorepo }  // detection output
ProjectDetector trait { exists(), list_subdirs(), list_files() }  // for testability
```

## Data Flow

### Common Pipeline (check/stats/baseline)

```
CLI args → load_config() → [if --offline] use cache only, error on miss
         → [if extends] resolve chain (local/remote/preset:*, cycle detection)
         → [if extends_sha256] verify remote config hash, error on mismatch
         → [if v1 config] migrate_v1_to_v2() auto-conversion (path_rules rejected with error)
         → expand_language_rules() → [content.languages.X] to [[content.rules]]
         → [if !--no-cache] load_cache(config_hash)
         → LanguageRegistry
         → [if gitignore] GitAwareScanner else DirectoryScanner
            Scanner returns ALL files (exclude patterns only, no extension filter)
         → ThresholdChecker::should_process() filters by content.exclude, then content.extensions OR rule match
         → parallel file processing (rayon):
              cache lookup by mtime+size → [miss] SlocCounter::count() → update cache
         → save_cache()
```

### check-specific

```
→ CheckContext::from_config(config, warn_threshold, exclude_patterns, use_gitignore)
   → creates injectable context with CompositeScanner + RealFileReader + StructureScanConfig
→ [if --files] Pure incremental mode: skip directory scan, use provided files, disable structure checks
   [else] ctx.scanner.scan_all_with_structure(paths, structure_scan_config) → ScanResult { files, dir_stats, allowlist_violations }
   (single WalkDir traversal collects both file list AND directory statistics)
→ [if --baseline] load_baseline() | [if --diff] filter changed files
→ get_skip_settings_for_path() → per-file skip_comments/skip_blank
→ process_file_with_cache(ctx.file_reader) → ThresholdChecker::check() → CheckResult (parallel)
→ [if !--files] StructureChecker::check(dir_stats) → StructureViolation (uses pre-collected stats, no traversal)
→ merge allowlist_violations from ScanResult
→ [if baseline] mark Grandfathered | [if --update-baseline] save violations to baseline
→ [if --suggest] generate_split_suggestions()
→ [if --report-json] ProjectStatistics → StatsJsonFormatter → write to path
→ format (Text/Json/Sarif/Markdown/Html) → output
```

### stats-specific

```
→ StatsContext::from_config() creates injectable context
→ collect FileStatistics → ProjectStatistics
→ [if --group-by] language/directory breakdown | [if --top N] top files
→ [if --trend] TrendHistory delta → save history
→ format (StatsText/Json/Markdown) → output
```

### config validate/show

```
validate: toml::from_str() → validate_config_semantics()
show: load_config() → format_config_text() or JSON
```

### explain-specific

```
→ load_config() → path.is_file()?
   [file] ThresholdChecker::explain(path) → ContentExplanation
   [dir]  StructureChecker::explain(path) → StructureExplanation
→ format (Text/Json) → output rule chain with match status
```

## Rule Priority (high→low)

**Content (SLOC limits):**
1. `[[content.rules]]` - glob pattern, LAST declared match wins (use `reason`/`expires` for exemptions)
2. `[content.languages.<ext>]` - extension shorthand
3. `[content]` defaults

**Structure (directory limits):**
1. `[[structure.rules]]` - glob pattern, LAST declared match wins (use `reason`/`expires` for exemptions)
2. `[structure]` defaults

## Dependencies

`clap` v4, `serde`/`toml`/`serde_json`, `walkdir`, `globset`, `ignore`, `rayon`, `indicatif`, `gix`, `sha2`, `regex`, `reqwest` (blocking + rustls-tls), `thiserror`

## Test
Each module has `*_tests.rs`. Run: `make ci` or `cargo test`
