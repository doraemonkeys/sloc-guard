# sloc-guard Project Overview

> **Doc Maintenance**: Keep concise, avoid redundancy, clean up outdated content promptly to reduce AI context usage.
> **Scope**: This document reflects the current codebase state only and does not describe future plans.
> **Goal**: Help AI quickly locate relevant code by module, type, and data flow.

**SLOC (Source Lines of Code) enforcement tool** - enforces file size limits by counting code lines (excluding comments and blanks) and enforces directory structure limits (file/folder counts).

## Quick Reference

```
Rust CLI tool | Clap v4 | TOML config | Exit: 0=pass, 1=threshold exceeded, 2=config error
```

## Module Map

| Module | Purpose |
|--------|---------|
| `cli` | Clap CLI: `check` (with `--files`, `--diff`, `--staged`, `--ratchet`, `--write-sarif`, `--write-json`, `--warnings-as-errors`, `--fail-fast`, `--no-sloc-cache`), `stats` (subcommands: `summary`, `files`, `breakdown`, `trend`, `history`, `report`; `breakdown`/`report` support `--depth` for directory grouping; common flags: `--no-sloc-cache`), `snapshot` (record history entry; uses common stats flags), `init` (with `--detect`), `config`, `explain` commands; global flags: `--extends-policy`, `--no-config`, `--no-extends` |
| `config/*` | `Config` (scanner/content/structure/check separation), `ContentConfig`, `StructureConfig`, `TrendConfig`, `CheckConfig`; loader with `extends` inheritance (local/remote/preset); presets module (rust-strict, node-strict, python-strict, monorepo-base); remote fetching with `FetchPolicy` (Normal: 1h TTL, Offline: ignore TTL, ForceRefresh: skip cache), cache in state directory, `extends_sha256` hash verification; `expires.rs`: date parsing/validation |
| `language/registry` | `LanguageRegistry`, `Language`, `CommentSyntax` - predefined + custom via [languages.<name>] config |
| `counter/*` | `CommentDetector`, `SlocCounter` → `CountResult{Stats, IgnoredFile}`, inline ignore directives |
| `scanner/*` | `FileScanner` trait (`scan()`, `scan_with_structure()`); `ScanResult`, `AllowlistRule`, `StructureScanConfig`; `directory.rs`: `DirectoryScanner` (walkdir + optional .gitignore via `ignore` crate); `composite.rs`: `CompositeScanner` (gitignore-aware/regular fallback), `scan_files()`; `filter.rs`: `GlobFilter` |
| `checker/*` | `Checker` trait; `result.rs`: `CheckResult` enum; `threshold.rs`: `ThresholdChecker` with pre-indexed extension lookup; `explain.rs`: `ContentExplanation`, `StructureExplanation` for rule chain debugging; `structure/`: `StructureChecker` (split into `builder.rs`, `compiled_rules.rs`, `validation.rs`, `violation.rs`) |
| `git/diff` | `GitDiff` - gix-based diff between committed trees (`--diff ref` or `--diff base..target` for explicit range) and staged files detection (`--staged` mode); `GitContext` - current commit hash and branch for trend entries |
| `baseline`/`cache` | `Baseline` (Content/Structure entries), `Cache` (mtime+size validation, file locking for concurrent access) |
| `state` | State file path resolution: `discover_project_root()` (walks up to find `.git/` or `.sloc-guard.toml`), `cache_path()`, `history_path()`, `baseline_path()` → `.git/sloc-guard/` (git repo) or `.sloc-guard/` (fallback); file locking utilities (`try_lock_exclusive_with_timeout`, `try_lock_shared_with_timeout`) for concurrent access protection; timestamp utilities (`current_unix_timestamp`, `try_current_unix_timestamp`) |
| `output/*` | `TextFormatter`, `JsonFormatter`, `SarifFormatter`, `MarkdownFormatter`, `HtmlFormatter` (with `with_stats()` for project stats, `with_trend_history()` for trend chart, `with_project_root()` for relative paths); `StatsTextFormatter`, `StatsJsonFormatter`, `StatsMarkdownFormatter`, `StatsHtmlFormatter` (with `with_project_root()`, `with_trend_history()` for trend chart, use `output_mode` field); `ScanProgress` (progress bar); `ErrorOutput` (colored error/warning output); `path.rs`: `display_path()` for relative path output with forward-slash normalization; `trend_formatting.rs`: relative time, trend arrows/colors/percentages; `svg/`: chart primitives (Axis, Bar, Line, BarChart, HorizontalBarChart, LineChart, FileSizeHistogram, LanguageBreakdownChart, TrendLineChart with delta indicators and smart X-axis labels, SvgBuilder) with viewBox scaling, CSS variables, hover effects, print styles, accessibility |
| `error` | `SlocGuardError` with `error_type()`, `message()`, `detail()`, `suggestion()` methods; `io_with_path()`/`io_with_context()` constructors for contextual IO errors; `message()` includes error kind for `FileAccess`/`Io` and glob details for `InvalidPattern` |
| `commands/*` | `run_check`, `run_stats`, `run_snapshot`, `run_config`, `run_init`, `run_explain`; check split into: `check_baseline_ops.rs`, `check_git_diff.rs`, `check_output.rs`, `check_processing.rs`, `check_validation.rs`; `context.rs`: `CheckContext`/`StatsContext` for DI; `detect.rs`: project type auto-detection |
| `analyzer` | `FunctionParser` - multi-language split suggestions (--suggest) |
| `stats` | `TrendHistory` - historical stats with delta computation, file locking, retention policy (max_entries, max_age_days, min_interval_secs); `parse_duration` - human-readable duration parsing for `--since` |
| `main` | CLI parsing, command dispatch to `commands/*` |

## Key Types

```rust
// Config (priority: CLI > file > defaults; extends: local/remote/preset)
// Presets: preset:rust-strict|node-strict|python-strict|monorepo-base
// Array Merge: arrays append (parent + child); use "$reset" to clear parent
FetchPolicy::Normal | Offline | ForceRefresh
Config { version, extends, extends_sha256, scanner, content, structure, baseline, trend, stats, check }
ScannerConfig { gitignore, exclude }
CheckConfig { warnings_as_errors, fail_fast }
BaselineConfig { ratchet: Option<RatchetMode> }
TrendConfig { max_entries, max_age_days, min_interval_secs, min_code_delta, auto_snapshot_on_check }
StatsConfig { report: StatsReportConfig }
StatsReportConfig { exclude, top_count, breakdown_by, depth, trend_since }
ContentConfig { extensions, max_lines, warn_threshold, warn_at, skip_comments, skip_blank, exclude, rules }
ContentRule { pattern, max_lines, warn_threshold, warn_at, skip_comments, skip_blank, reason, expires }
StructureConfig { max_files, max_dirs, max_depth, warn_threshold, warn_files_at, warn_dirs_at, warn_files_threshold, warn_dirs_threshold, count_exclude, allow_extensions, allow_files, allow_dirs, deny_extensions, deny_patterns, deny_files, deny_dirs, rules }
StructureRule { scope, max_files, max_dirs, max_depth, relative_depth, warn_threshold, warn_files_at, warn_dirs_at, warn_files_threshold, warn_dirs_threshold, allow_extensions, allow_patterns, allow_files, allow_dirs, deny_extensions, deny_patterns, deny_files, deny_dirs, file_naming_pattern, siblings, reason, expires }
SiblingRule::Directed { match_pattern, require, severity } | Group { group, severity }
SiblingSeverity::Error | Warn
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
ViolationCategory::Content | Structure { violation_type, triggering_rule }

// Structure checking
DirStats { file_count, dir_count, depth }
ViolationType::FileCount | DirCount | MaxDepth | DisallowedFile | DisallowedDirectory | DeniedFile { pattern_or_extension } | DeniedDirectory { pattern } | NamingConvention { expected_pattern } | MissingSibling { expected_sibling_pattern } | GroupIncomplete { group_patterns, missing_members }
StructureViolation { path, violation_type, actual, limit, is_warning, override_reason, triggering_rule_pattern }

// Explain (rule chain debugging)
MatchStatus::Matched | Superseded | NoMatch
ContentRuleMatch::Excluded { pattern } | Rule { index, pattern, reason } | Default
WarnAtSource::RuleAbsolute { index } | RulePercentage { index, threshold } | GlobalAbsolute | GlobalPercentage { threshold }
ContentExplanation { path, is_excluded, matched_rule, effective_limit, effective_warn_at, warn_at_source, warn_threshold, skip_*, rule_chain }
StructureRuleMatch::Rule { index, pattern, reason } | Default
StructureExplanation { path, matched_rule, effective_max_files, effective_max_dirs, effective_max_depth, warn_threshold, rule_chain }

// Output
OutputFormat::Text | Json | Sarif | Markdown | Html
ColorMode::Auto | Always | Never

// Stats
FileStatistics { path, stats, language }
ProjectStatistics { files, total_*, by_language, by_directory, top_files, average_code_lines, trend, output_mode }
FileSortOrder::Code | Total | Comment | Blank | Name
StatsOutputMode::Full | SummaryOnly | FilesOnly
GroupBy::None | Lang | Dir

// Trend (state::history_path())
TrendEntry { timestamp, total_files, total_lines, code, comment, blank, git_ref?, git_branch? }
TrendDelta { *_delta, previous_timestamp, previous_git_ref?, previous_git_branch? }
// TrendHistory: apply_retention(), should_add(), find_entry_at_or_before(), compute_delta_since()

// Git/Baseline/Cache
GitContext { commit, branch? }
GitContext::from_path(path) → Option<GitContext>
GitDiff::get_changed_files(base_ref), get_changed_files_range(base, target), get_staged_files()
Baseline { version, files: HashMap<path, BaselineEntry> }
BaselineEntry::Content { lines, hash } | Structure { violation_type, count }
StructureViolationType::Files | Dirs
BaselineUpdateMode::All | Content | Structure | New
RatchetMode::Warn | Auto | Strict
RatchetResult { stale_entries, stale_paths }
Cache { version, config_hash, files: HashMap<path, CacheEntry{hash, stats, mtime, size}> }
LockError::Timeout | Io(io::Error)

// Split suggestions (--suggest)
FunctionInfo { name, start_line, end_line, line_count }
SplitSuggestion { original_path, total_lines, limit, functions, chunks }
FunctionParser: Rust, Go, Python, JS/TS, C/C++

// Context for DI (commands/context.rs)
FileReader trait { read(), metadata() }
RealFileReader
FileScanner trait { scan(), scan_all(), scan_with_structure(), scan_all_with_structure() }
ScanResult { files, dir_stats, allowlist_violations }
StructureScanConfig { count_exclude, scanner_exclude, scanner_exclude_dir_names, allowlist_rules, global_allow_*, global_deny_* }
AllowlistRule { scope, allow_extensions, allow_patterns, allow_files, allow_dirs, deny_extensions, deny_patterns, deny_files, naming_pattern_str }
CompositeScanner
CheckContext { registry, threshold_checker, structure_checker, structure_scan_config, scanner, file_reader }
CheckOptions { args, cli, paths, config, ctx, cache, baseline, project_root }
StatsContext { registry, allowed_extensions }

// Project Detection (init --detect)
ProjectType::Rust | Node | Go | Python | Java | CSharp | Unknown
DetectedProject { path, project_type }
DetectionResult { root, subprojects, is_monorepo }
ProjectDetector trait { exists(), list_subdirs(), list_files() }
```

## Data Flow

### Common Pipeline (check/stats/baseline)

```
CLI args → load_config() → [if --extends-policy=offline] use cache only, error on miss
         → [if extends] resolve chain (local/remote/preset:*, cycle detection)
         → [if extends_sha256] verify remote config hash, error on mismatch
         → [if !--no-sloc-cache] load_cache(config_hash)
         → LanguageRegistry
         → DirectoryScanner (with or without gitignore support)
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
→ [if stats history] run_history(): load TrendHistory, format entries (text/json), output
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
2. `[content]` defaults

**Structure (directory limits):**
1. `[[structure.rules]]` - glob pattern, LAST declared match wins (use `reason`/`expires` for exemptions)
2. `[structure]` defaults

## Dependencies

`clap` v4, `serde`/`toml`/`serde_json`, `walkdir`, `globset`, `ignore`, `rayon`, `indicatif`, `gix`, `sha2`, `regex`, `reqwest` (blocking + rustls-tls), `thiserror`

## Test
Each module has `*_tests.rs`. Run: `make ci` or `cargo test`
