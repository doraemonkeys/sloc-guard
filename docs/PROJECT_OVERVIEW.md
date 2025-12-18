# sloc-guard Project Overview

> **Doc Maintenance**: Keep concise, avoid redundancy, clean up outdated content promptly to reduce AI context usage.

**SLOC (Source Lines of Code) enforcement tool** - enforces file size limits by counting code lines (excluding comments and blanks) and enforces directory structure limits (file/folder counts).

## Quick Reference

```
Rust CLI tool | Clap v4 | TOML config | Exit: 0=pass, 1=threshold exceeded, 2=config error
```

## Module Map

| Module | File(s) | Purpose |
|--------|---------|---------|
| `cli` | `cli.rs` | Clap CLI: `check`, `stats`, `init`, `config`, `baseline` commands |
| `config/*` | `config/*.rs` | `Config`, `DefaultConfig`, `RuleConfig`, `PathRule`, `FileOverride`, `StructureConfig`; loader with `extends` inheritance; remote fetching (1h TTL cache) |
| `language/registry` | `language/registry.rs` | `LanguageRegistry`, `Language`, `CommentSyntax` - predefined + custom via [languages.<name>] config |
| `counter/*` | `counter/*.rs` | `CommentDetector`, `SlocCounter` → `CountResult{Stats, IgnoredFile}`, inline ignore directives |
| `scanner/*` | `scanner/*.rs` | `GlobFilter`, `DirectoryScanner` (walkdir), `GitAwareScanner` (gix with .gitignore) |
| `checker/threshold` | `checker/threshold.rs` | `ThresholdChecker` with pre-indexed extension lookup → `CheckResult{status, stats, limit, suggestions}` |
| `git/diff` | `git/diff.rs` | `GitDiff` - gix-based changed files detection for `--diff` mode |
| `baseline`/`cache` | `*/types.rs` | `Baseline` (grandfathering), `Cache` (mtime+size validation) |
| `output/*` | `output/*.rs` | `TextFormatter`, `JsonFormatter`, `SarifFormatter`, `MarkdownFormatter`, `HtmlFormatter`; `StatsTextFormatter`, `StatsJsonFormatter`, `StatsMarkdownFormatter`; `ScanProgress` (progress bar) |
| `error` | `error.rs` | `SlocGuardError` enum: Config/FileRead/InvalidPattern/Io/TomlParse/JsonSerialize/Git |
| `commands/*` | `commands/*.rs` | `run_check`, `run_stats`, `run_baseline`, `run_config`, `run_init`; shared utilities |
| `analyzer` | `analyzer/*.rs` | `FunctionParser` - multi-language split suggestions (--fix) |
| `stats` | `stats/trend.rs` | `TrendHistory` - historical stats with delta computation |
| `main` | `main.rs` | CLI parsing, command dispatch to `commands/*` |

## Key Types

```rust
// Config (priority: CLI > file > defaults; extends: local/remote with 1h TTL cache)
Config { extends, default, rules, path_rules, exclude, overrides, languages, structure }
DefaultConfig { max_lines: 500, extensions: [rs,go,py,js,ts,c,cpp], skip_comments: true, skip_blank: true, warn_threshold: 0.9, strict: false, gitignore: true }
RuleConfig { extensions, max_lines, skip_comments, skip_blank, warn_threshold }
PathRule { pattern, max_lines, warn_threshold }  // glob: "src/generated/**"
FileOverride { path, max_lines, reason }
CustomLanguageConfig { extensions, single_line_comments, multi_line_comments }
StructureConfig { max_files, max_dirs, ignore, rules: Vec<StructureRule> }

// Line counting (ignore directives: ignore-file, ignore-next N, ignore-start/end)
LineStats { total, code, comment, blank, ignored }
CountResult::Stats(LineStats) | IgnoredFile
CommentSyntax { single_line, multi_line }

// Check results
CheckStatus::Passed | Warning | Failed | Grandfathered
CheckResult { path, status, stats, limit, override_reason, suggestions }

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
```

## Data Flow

### Common Pipeline (check/stats/baseline)

```
CLI args → load_config() → [if extends] resolve chain (local/remote, cycle detection)
         → [if !--no-cache] load_cache(config_hash)
         → LanguageRegistry + GlobFilter
         → [if gitignore] GitAwareScanner else DirectoryScanner
         → parallel file processing (rayon):
              cache lookup by mtime+size → [miss] SlocCounter::count() → update cache
         → save_cache()
```

### check-specific

```
→ [if --baseline] load_baseline() | [if --diff] filter changed files
→ ThresholdChecker::check() → CheckResult
→ [if baseline] mark Grandfathered | [if --fix] generate_split_suggestions()
→ format (Text/Json/Sarif/Markdown/Html) → output
```

### stats-specific

```
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

## Threshold Resolution (priority high→low)

1. `[[override]]` - path suffix match (by components: `legacy.rs` matches `src/legacy.rs`)
2. `[[path_rules]]` - glob pattern match (e.g., `src/generated/**`)
3. `[rules.*]` - extension match
4. `[default]` - fallback

## Dependencies

`clap` v4, `serde`/`toml`/`serde_json`, `walkdir`, `globset`, `rayon`, `indicatif`, `gix`, `sha2`, `regex`, `reqwest` (blocking + rustls-tls), `thiserror`

## Test

Each module has `*_tests.rs`. Run: `make ci` or `cargo test`