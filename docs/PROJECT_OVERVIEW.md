# sloc-guard Project Overview

> **Doc Maintenance**: Keep concise, avoid redundancy, clean up outdated content promptly to reduce AI context usage.
> **Scope**: This document reflects the current codebase state only and does not describe future plans.

**SLOC (Source Lines of Code) enforcement tool** - enforces file size limits by counting code lines (excluding comments and blanks) and enforces directory structure limits (file/folder counts).

## Quick Reference

```
Rust CLI tool | Clap v4 | TOML config | Exit: 0=pass, 1=threshold exceeded, 2=config error
```

## Module Map

| Module | File(s) | Purpose |
|--------|---------|---------|
| `cli` | `cli.rs` | Clap CLI: `check`, `stats`, `init`, `config`, `baseline` commands |
| `config/*` | `config/*.rs` | `Config` (v2: scanner/content/structure separation), `ContentConfig`, `StructureConfig`, `ContentOverride`, `StructureOverride`; loader with `extends` inheritance; remote fetching (1h TTL cache) |
| `language/registry` | `language/registry.rs` | `LanguageRegistry`, `Language`, `CommentSyntax` - predefined + custom via [languages.<name>] config |
| `counter/*` | `counter/*.rs` | `CommentDetector`, `SlocCounter` → `CountResult{Stats, IgnoredFile}`, inline ignore directives |
| `scanner/*` | `scanner/*.rs` | `GlobFilter`, `DirectoryScanner` (walkdir), `GitAwareScanner` (gix with .gitignore) |
| `checker/threshold` | `checker/threshold.rs` | `ThresholdChecker` with pre-indexed extension lookup → `CheckResult` enum (Passed/Warning/Failed/Grandfathered) |
| `checker/structure` | `checker/structure.rs` | `StructureChecker` - directory file/subdir count limits with glob-based rules |
| `git/diff` | `git/diff.rs` | `GitDiff` - gix-based changed files detection for `--diff` mode |
| `baseline`/`cache` | `*/types.rs` | `Baseline` (grandfathering), `Cache` (mtime+size validation) |
| `output/*` | `output/*.rs` | `TextFormatter`, `JsonFormatter`, `SarifFormatter`, `MarkdownFormatter`, `HtmlFormatter`; `StatsTextFormatter`, `StatsJsonFormatter`, `StatsMarkdownFormatter`; `ScanProgress` (progress bar) |
| `error` | `error.rs` | `SlocGuardError` enum: Config/FileRead/InvalidPattern/Io/TomlParse/JsonSerialize/Git |
| `commands/*` | `commands/*.rs` | `run_check`, `run_stats`, `run_baseline`, `run_config`, `run_init`; `CheckContext`/`StatsContext` for DI |
| `analyzer` | `analyzer/*.rs` | `FunctionParser` - multi-language split suggestions (--fix) |
| `stats` | `stats/trend.rs` | `TrendHistory` - historical stats with delta computation |
| `main` | `main.rs` | CLI parsing, command dispatch to `commands/*` |

## Key Types

```rust
// Config (priority: CLI > file > defaults; extends: local/remote with 1h TTL cache)
// V2 schema separates scanner/content/structure concerns
Config { version, scanner, content, structure }
ScannerConfig { gitignore: true, exclude: Vec<glob> }  // Physical discovery, no extension filter
ContentConfig { extensions, max_lines, warn_threshold, skip_comments, skip_blank, rules, languages, overrides }
ContentRule { pattern, max_lines, warn_threshold, skip_comments, skip_blank }  // [[content.rules]]
ContentOverride { path, max_lines, reason }  // [[content.override]] - file only
StructureConfig { max_files, max_dirs, warn_threshold, count_exclude, rules, overrides }
StructureRule { pattern, max_files, max_dirs, warn_threshold }  // [[structure.rules]]
StructureOverride { path, max_files, max_dirs, reason }  // [[structure.override]] - dir only
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
DirStats { file_count, dir_count }  // immediate children counts
ViolationType::FileCount | DirCount
StructureViolation { path, violation_type, actual, limit, is_warning, override_reason }

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
GitDiff::get_changed_files(base_ref) → HashSet<PathBuf>
Baseline { version, files: HashMap<path, BaselineEntry{lines, hash}> }  // .sloc-guard-baseline.json
Cache { version, config_hash, files: HashMap<path, CacheEntry{hash, stats, mtime, size}> }  // .sloc-guard-cache.json

// Split suggestions (--fix)
FunctionInfo { name, start_line, end_line, line_count }
SplitSuggestion { original_path, total_lines, limit, functions, chunks }
FunctionParser: Rust, Go, Python, JS/TS, C/C++

// Context for DI (commands/context.rs)
CheckContext { registry, threshold_checker, structure_checker }  // from_config() or new()
StatsContext { registry, allowed_extensions }  // from_config() or new()
```

## Data Flow

### Common Pipeline (check/stats/baseline)

```
CLI args → load_config() → [if extends] resolve chain (local/remote, cycle detection)
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
→ CheckContext::from_config() creates injectable context
→ [if --baseline] load_baseline() | [if --diff] filter changed files
→ get_skip_settings_for_path() → per-file skip_comments/skip_blank
→ ThresholdChecker::check() → CheckResult (parallel, per-file)
→ StructureChecker::check_directory() → StructureViolation → CheckResult (per-dir)
→ [if baseline] mark Grandfathered | [if --fix] generate_split_suggestions()
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

### baseline-specific

```
→ collect Failed files → compute_file_hash() → Baseline::save()
```

### config validate/show

```
validate: toml::from_str() → validate_config_semantics()
show: load_config() → format_config_text() or JSON
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
