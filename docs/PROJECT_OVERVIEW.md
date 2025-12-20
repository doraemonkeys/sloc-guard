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
| `cli` | Clap CLI: `check` (with `--files`, `--diff`, `--staged`), `stats`, `init` (with `--detect`), `config`, `baseline`, `explain` commands |
| `config/*` | `Config` (v2: scanner/content/structure separation), `ContentConfig`, `StructureConfig`, `ContentOverride`, `StructureOverride`; loader with `extends` inheritance (local/remote/preset); presets module (rust-strict, node-strict, python-strict, monorepo-base); remote fetching (1h TTL cache) |
| `language/registry` | `LanguageRegistry`, `Language`, `CommentSyntax` - predefined + custom via [languages.<name>] config |
| `counter/*` | `CommentDetector`, `SlocCounter` → `CountResult{Stats, IgnoredFile}`, inline ignore directives |
| `scanner/*` | `FileScanner` trait (`scan()`, `scan_with_structure()`), `GlobFilter`, `DirectoryScanner` (walkdir), `GitAwareScanner` (gix with .gitignore), `CompositeScanner` (git/non-git fallback), `ScanResult`, `StructureScanConfig` |
| `checker/threshold` | `ThresholdChecker` with pre-indexed extension lookup → `CheckResult` enum (Passed/Warning/Failed/Grandfathered) |
| `checker/structure` | `StructureChecker` - directory file/subdir/depth limits with glob-based rules |
| `checker/explain` | `ContentExplanation`, `StructureExplanation` - rule chain debugging types |
| `git/diff` | `GitDiff` - gix-based changed files detection (`--diff` mode) and staged files detection (`--staged` mode) |
| `baseline`/`cache` | `Baseline` V2 (Content/Structure entries, V1 auto-migration), `Cache` (mtime+size validation) |
| `output/*` | `TextFormatter`, `JsonFormatter`, `SarifFormatter`, `MarkdownFormatter`, `HtmlFormatter`; `StatsTextFormatter`, `StatsJsonFormatter`, `StatsMarkdownFormatter`; `ScanProgress` (progress bar) |
| `error` | `SlocGuardError` enum: Config/FileRead/InvalidPattern/Io/TomlParse/JsonSerialize/Git |
| `commands/*` | `run_check`, `run_stats`, `run_baseline`, `run_config`, `run_init`, `run_explain`; `CheckContext`/`StatsContext` for DI; `detect` module for project type auto-detection |
| `analyzer` | `FunctionParser` - multi-language split suggestions (--suggest) |
| `stats` | `TrendHistory` - historical stats with delta computation |
| `main` | CLI parsing, command dispatch to `commands/*` |

## Key Types

```rust
// Config (priority: CLI > file > defaults; extends: local/remote/preset with 1h TTL cache for remote)
// V2 schema separates scanner/content/structure concerns
// Presets: extends = "preset:rust-strict|node-strict|python-strict|monorepo-base"
Config { version, scanner, content, structure }
ScannerConfig { gitignore: true, exclude: Vec<glob> }  // Physical discovery, no extension filter
ContentConfig { extensions, max_lines, warn_threshold, skip_comments, skip_blank, rules, languages, overrides }
ContentRule { pattern, max_lines, warn_threshold, skip_comments, skip_blank }  // [[content.rules]]
ContentOverride { path, max_lines, reason }  // [[content.override]] - file only
StructureConfig { max_files, max_dirs, max_depth, warn_threshold, count_exclude, rules, overrides }
StructureRule { pattern, max_files, max_dirs, max_depth, warn_threshold, allow_extensions, allow_patterns }  // [[structure.rules]]
StructureOverride { path, max_files, max_dirs, max_depth, reason }  // [[structure.override]] - dir only
CustomLanguageConfig { extensions, single_line_comments, multi_line_comments }

// Line counting (ignore directives: ignore-file, ignore-next N, ignore-start/end)
LineStats { total, code, comment, blank, ignored }
CountResult::Stats(LineStats) | IgnoredFile
CommentSyntax { single_line, multi_line }

// Check results (enum with associated data)
CheckResult::Passed { path, stats, limit, override_reason }
          | Warning { ..., suggestions }
          | Failed { ..., suggestions }
          | Grandfathered { ... }
// Accessor methods: path(), stats(), limit(), override_reason(), suggestions()
// Consuming: into_grandfathered(), with_suggestions()

// Structure checking
DirStats { file_count, dir_count, depth }  // immediate children counts + depth from scan root
ViolationType::FileCount | DirCount | MaxDepth | DisallowedFile
StructureViolation { path, violation_type, actual, limit, is_warning, override_reason, triggering_rule_pattern }

// Explain (rule chain debugging)
MatchStatus::Matched | Superseded | NoMatch
ContentRuleMatch::Override { index, reason } | Rule { index, pattern } | Default
ContentExplanation { path, matched_rule, effective_limit, warn_threshold, skip_*, rule_chain }
StructureRuleMatch::Override { index, reason } | Rule { index, pattern } | Default
StructureExplanation { path, matched_rule, effective_max_files, effective_max_dirs, effective_max_depth, warn_threshold, rule_chain }

// Output
OutputFormat::Text | Json | Sarif | Markdown | Html
ColorMode::Auto | Always | Never

// Stats
FileStatistics { path, stats, language }
ProjectStatistics { files, total_*, by_language, by_directory, top_files, average_code_lines, trend }
GroupBy::None | Lang | Dir

// Trend (.sloc-guard-history.json)
TrendEntry { timestamp, total_files, total_lines, code, comment, blank }
TrendDelta { *_delta, previous_timestamp }

// Git/Baseline/Cache
GitDiff::get_changed_files(base_ref) → HashSet<PathBuf>  // --diff mode
GitDiff::get_staged_files() → HashSet<PathBuf>  // --staged mode
// Baseline V2 (.sloc-guard-baseline.json) - auto-migrates V1 format
Baseline { version: 2, files: HashMap<path, BaselineEntry> }
BaselineEntry::Content { lines, hash } | Structure { violation_type, count }
StructureViolationType::Files | Dirs
BaselineUpdateMode::All | Content | Structure | New  // --update-baseline mode
Cache { version, config_hash, files: HashMap<path, CacheEntry{hash, stats, mtime, size}> }  // .sloc-guard-cache.json

// Split suggestions (--suggest)
FunctionInfo { name, start_line, end_line, line_count }
SplitSuggestion { original_path, total_lines, limit, functions, chunks }
FunctionParser: Rust, Go, Python, JS/TS, C/C++

// Context for DI (commands/context.rs)
FileReader trait { read(), metadata() }  // IO abstraction for file reading
RealFileReader  // Production impl using std::fs
FileScanner trait { scan(), scan_all(), scan_with_structure(), scan_all_with_structure() }  // IO abstraction for directory traversal
ScanResult { files, dir_stats, allowlist_violations }  // Unified scan output
StructureScanConfig { count_exclude, scanner_exclude, scanner_exclude_dir_names, allowlist_rules }  // Config for structure-aware scanning
AllowlistRule { pattern, allow_extensions, allow_patterns }  // Directory allowlist matching
CompositeScanner  // Production impl with git/non-git fallback
CheckContext { registry, threshold_checker, structure_checker, structure_scan_config, scanner, file_reader }  // from_config() or new()
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
CLI args → load_config() → [if extends] resolve chain (local/remote/preset:*, cycle detection)
         → [if v1 config] migrate_v1_to_v2() auto-conversion
         → expand_language_rules() → [content.languages.X] to [[content.rules]]
         → [if !--no-cache] load_cache(config_hash)
         → LanguageRegistry
         → [if gitignore] GitAwareScanner else DirectoryScanner
            Scanner returns ALL files (exclude patterns only, no extension filter)
         → ThresholdChecker::should_process() filters by content.extensions
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

### baseline-specific (deprecated: use `check --update-baseline` instead)

```
→ collect Failed files → compute_file_hash() → Baseline::save()
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
1. `[[content.override]]` - exact path match (file only)
2. `[[content.rules]]` - glob pattern, LAST declared match wins
3. `[content.languages.<ext>]` - extension shorthand
4. `[content]` defaults

**Structure (directory limits):**
1. `[[structure.override]]` - exact path match (dir only)
2. `[[structure.rules]]` - glob pattern, LAST declared match wins
3. `[structure]` defaults

## Dependencies

`clap` v4, `serde`/`toml`/`serde_json`, `walkdir`, `globset`, `rayon`, `indicatif`, `gix`, `sha2`, `regex`, `reqwest` (blocking + rustls-tls), `thiserror`

## Test
Each module has `*_tests.rs`. Run: `make ci` or `cargo test`
