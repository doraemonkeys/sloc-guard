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
| `cli` | `cli.rs` | Clap-derived CLI: `check` (--baseline, --no-cache, --no-gitignore, --fix), `stats` (--no-cache, --group-by, --top, --no-gitignore, --trend), `init`, `config`, `baseline` commands; global flags: --no-config, --no-extends |
| `config/model` | `config/model.rs` | `Config`, `DefaultConfig`, `RuleConfig` (with warn_threshold), `ExcludeConfig`, `FileOverride`, `PathRule`, `CustomLanguageConfig`, `StructureConfig`, `StructureRule` |
| `config/loader` | `config/loader.rs` | `FileConfigLoader` - loads `.sloc-guard.toml` or `~/.config/sloc-guard/config.toml`, supports `extends` for config inheritance (local path or remote URL) |
| `config/remote` | `config/remote.rs` | `HttpClient` trait, `ReqwestClient`, `fetch_remote_config[_with_client]`, `is_remote_url`, `clear_cache` - remote config fetching with 1h TTL cache, DI for testability |
| `language/registry` | `language/registry.rs` | `LanguageRegistry`, `Language`, `CommentSyntax` - predefined + custom via [languages.<name>] config |
| `counter/comment` | `counter/comment.rs` | `CommentDetector` - detects single/multi-line comments |
| `counter/sloc` | `counter/sloc.rs` | `SlocCounter` → `CountResult{Stats(LineStats), IgnoredFile}`, inline ignore directives (file/next/block) |
| `scanner/filter` | `scanner/filter.rs` | `GlobFilter` - extension + exclude pattern filtering |
| `scanner/mod` | `scanner/mod.rs` | `DirectoryScanner` - walkdir-based file discovery |
| `scanner/gitignore` | `scanner/gitignore.rs` | `GitAwareScanner` - gix dirwalk with .gitignore support |
| `checker/threshold` | `checker/threshold.rs` | `ThresholdChecker` with pre-indexed extension lookup → `CheckResult{status, stats, limit, suggestions}` |
| `git/diff` | `git/diff.rs` | `GitDiff` - gix-based changed files detection for `--diff` mode |
| `baseline` | `baseline/types.rs` | `Baseline`, `BaselineEntry` - baseline file for grandfathering violations |
| `cache` | `cache/types.rs` | `Cache`, `CacheEntry`, `CachedLineStats`, `compute_config_hash` - file hash caching with mtime+size validation |
| `output/text` | `output/text.rs` | `TextFormatter`, `ColorMode` - human-readable output with color and verbose support |
| `output/json` | `output/json.rs` | `JsonFormatter` - structured JSON output |
| `output/sarif` | `output/sarif.rs` | `SarifFormatter` - SARIF 2.1.0 output for GitHub Code Scanning |
| `output/markdown` | `output/markdown.rs` | `MarkdownFormatter` - table-based markdown output for PR comments |
| `output/html` | `output/html.rs` | `HtmlFormatter` - standalone HTML report with embedded CSS, summary cards, all files table with sortable columns and status filtering |
| `output/stats` | `output/stats.rs` | `StatsTextFormatter`, `StatsJsonFormatter`, `StatsMarkdownFormatter`, `LanguageStats`, `DirectoryStats` - stats output with language/directory breakdown, top-N files, average |
| `output/progress` | `output/progress.rs` | `ScanProgress` - indicatif-based progress bar, disabled in quiet mode or non-TTY |
| `error` | `error.rs` | `SlocGuardError` enum: Config/FileRead/InvalidPattern/Io/TomlParse/JsonSerialize/Git |
| `commands/common` | `commands/common.rs` | Shared utilities: `load_config`, `load_cache`, `save_cache`, `resolve_scan_paths`, `write_output`, `process_file_with_cache`, `get_file_metadata` |
| `commands/check` | `commands/check.rs` | `run_check` - check command execution, baseline loading/comparison, git diff filtering, output formatting |
| `commands/stats` | `commands/stats.rs` | `run_stats` - stats command execution, file stats collection, trend tracking |
| `commands/baseline` | `commands/baseline_cmd.rs` | `run_baseline` - baseline update command execution |
| `commands/config` | `commands/config.rs` | `run_config`, `validate_config_semantics`, `format_config_text` |
| `commands/init` | `commands/init.rs` | `run_init`, `generate_config_template` |
| `analyzer` | `analyzer/*.rs` | `SplitAnalyzer`, `FunctionParser` - parses functions for multi-language split suggestions (--fix mode) |
| `stats` | `stats/trend.rs` | `TrendHistory`, `TrendEntry`, `TrendDelta` - historical stats storage (.sloc-guard-history.json), delta computation |
| `main` | `main.rs` | CLI parsing, command dispatch to `commands/*` |

## Key Types

```rust
// Config priority: CLI args > config file > defaults
Config { extends: Option<String>, default: DefaultConfig, rules: HashMap<String, RuleConfig>, path_rules: Vec<PathRule>, exclude: ExcludeConfig, overrides: Vec<FileOverride>, languages: HashMap<String, CustomLanguageConfig>, structure: StructureConfig }
// extends: local path or remote URL (http/https) to base config, merged recursively with cycle detection
// Remote configs cached at ~/.cache/sloc-guard/configs/ (Windows: %LOCALAPPDATA%\sloc-guard\configs\) with 1h TTL
DefaultConfig { max_lines: 500, extensions: [rs,go,py,js,ts,c,cpp], include_paths, skip_comments: true, skip_blank: true, warn_threshold: 0.9, strict: false, gitignore: true }
RuleConfig { extensions: Vec<String>, max_lines: Option<usize>, skip_comments: Option<bool>, skip_blank: Option<bool>, warn_threshold: Option<f64> }
PathRule { pattern: String, max_lines: usize, warn_threshold: Option<f64> }  // glob patterns like "src/generated/**"
FileOverride { path: String, max_lines: usize, reason: Option<String> }  // per-file override with optional reason
CustomLanguageConfig { extensions: Vec<String>, single_line_comments: Vec<String>, multi_line_comments: Vec<(String, String)> }  // custom language via [languages.<name>]
StructureConfig { max_files: Option<usize>, max_dirs: Option<usize>, ignore: Vec<String>, rules: Vec<StructureRule> }  // [structure] section
StructureRule { pattern: String, max_files: Option<usize>, max_dirs: Option<usize> }  // [[structure.rules]] per-directory overrides

// Line counting
LineStats { total, code, comment, blank, ignored }  // sloc() returns code count
CountResult::Stats(LineStats) | IgnoredFile  // IgnoredFile when "// sloc-guard:ignore-file" in first 10 lines
// Inline ignore directives (in single-line comments only):
//   // sloc-guard:ignore-file - ignores entire file (first 10 lines only)
//   // sloc-guard:ignore-next N - ignores next N lines (counts as ignored)
//   // sloc-guard:ignore-start / ignore-end - ignores block (counts as ignored)
CommentSyntax { single_line: Vec<String>, multi_line: Vec<(String, String)> }
LanguageRegistry::with_custom_languages(&config.languages) // builds registry with custom languages (override built-in if same extension)

// Check results
CheckStatus::Passed | Warning | Failed | Grandfathered
CheckResult { path, status, stats, limit, override_reason: Option<String>, suggestions: Option<SplitSuggestion> }

// Output formatting
OutputFormat::Text | Json | Sarif | Markdown | Html  // --format flag
ColorMode::Auto | Always | Never  // controls ANSI color output
TextFormatter::with_verbose(mode, verbose)  // verbose >= 1 shows passed files
HtmlFormatter::new().with_suggestions(show)  // standalone HTML report with embedded CSS, sortable columns, status filtering

// Stats results (no threshold checking)
FileStatistics { path, stats: LineStats, language: String }
ProjectStatistics { files, total_files, total_lines, total_code, total_comment, total_blank, by_language, by_directory, top_files, average_code_lines, trend }
LanguageStats { language, files, total_lines, code, comment, blank }
DirectoryStats { directory, files, total_lines, code, comment, blank }
GroupBy::None | Lang | Dir  // --group-by option for stats command
ProjectStatistics::with_language_breakdown() → computes per-language stats
ProjectStatistics::with_directory_breakdown() → computes per-directory stats
ProjectStatistics::with_top_files(n) → computes top N files by code lines + average
ProjectStatistics::with_trend(delta) → adds trend delta from previous run

// Trend tracking (--trend flag)
TrendEntry { timestamp, total_files, total_lines, code, comment, blank }  // historical snapshot
TrendDelta { files_delta, lines_delta, code_delta, comment_delta, blank_delta, previous_timestamp }  // delta from previous
TrendHistory { version, entries: Vec<TrendEntry> }  // .sloc-guard-history.json
TrendHistory::load_or_default(path) → TrendHistory  // loads or creates empty
TrendHistory::compute_delta(stats) → Option<TrendDelta>  // compares with latest entry

// Git integration
GitDiff::discover(path) → GitDiff  // Finds git repo from path
ChangedFiles::get_changed_files(base_ref) → HashSet<PathBuf>  // Files changed since reference

// Baseline (grandfathering violations)
Baseline { version: u32, files: HashMap<String, BaselineEntry> }  // .sloc-guard-baseline.json
BaselineEntry { lines: usize, hash: String }  // SHA-256 content hash
compute_file_hash(path) → String  // SHA-256 of file content

// Cache (file hash caching with metadata validation)
Cache { version: u32, config_hash: String, files: HashMap<String, CacheEntry> }  // .sloc-guard-cache.json
CacheEntry { hash: String, stats: CachedLineStats, mtime: u64, size: u64 }  // mtime+size for fast validation
compute_config_hash(config) → String  // SHA-256 of serialized config

// Progress bar (disabled in quiet mode or non-TTY)
ScanProgress::new(total, quiet) → ScanProgress  // Thread-safe with AtomicU64
ScanProgress::inc()  // Increment counter (rayon-safe)
ScanProgress::finish()  // Clear progress bar

// Split suggestions (--fix mode)
FunctionInfo { name, start_line, end_line, line_count }  // Parsed function boundaries
SplitChunk { name, functions, start_line, end_line, line_count }  // Suggested file chunk
SplitSuggestion { original_path, total_lines, limit, functions, chunks }  // Full split recommendation
FunctionParser trait + get_parser(language) → Box<dyn FunctionParser>  // Rust, Go, Python, JS/TS, C/C++
```

## Data Flow (check command)

```
CLI args → load_config() → [if extends && !--no-extends] resolve extends chain (local or remote, cycle detection) → merge configs
         → [if remote URL] fetch_remote_config(url) with cache lookup (1h TTL)
         → apply_cli_overrides()
         → [if --baseline] load_baseline() → Baseline
         → [if !--no-cache] load_cache(config_hash) → Cache
         → LanguageRegistry::with_custom_languages(config.languages)
         → GlobFilter::new(extensions, excludes)
         → [if gitignore enabled] GitAwareScanner::scan(paths) else DirectoryScanner::scan(paths)
         → [if --diff] GitDiff::get_changed_files() → filter to changed only
         → ScanProgress::new(file_count, quiet)
         → for each file (parallel with rayon):
              get_file_metadata(mtime, size) → check cache by metadata
              [if cache hit] use cached LineStats (no file read)
              [if cache miss] read_file_with_hash()
                              LanguageRegistry::get_by_extension()
                              SlocCounter::count(content) → CountResult
                              update cache with stats + metadata
              [if IgnoredFile] skip file (inline ignore directive)
              [if Stats] ThresholdChecker::check(path, stats) → CheckResult
              progress.inc()
         → progress.finish()
         → [if !--no-cache] save_cache()
         → [if baseline] apply_baseline_comparison() → mark Failed as Grandfathered
         → [if --fix] generate_split_suggestions(results, registry) → add SplitSuggestion to failed CheckResults
         → TextFormatter/JsonFormatter/SarifFormatter/MarkdownFormatter/HtmlFormatter::format(results)
         → write to stdout or --output file
```

## Data Flow (stats command)

```
CLI args → load_config()
         → [if !--no-cache] load_cache(config_hash) → Cache
         → LanguageRegistry::with_custom_languages(config.languages)
         → GlobFilter::new(extensions, excludes)
         → [if gitignore enabled] GitAwareScanner::scan(paths) else DirectoryScanner::scan(paths)
         → ScanProgress::new(file_count, quiet)
         → for each file (parallel with rayon):
              get_file_metadata(mtime, size) → check cache by metadata
              [if cache hit] use cached LineStats (no file read)
              [if cache miss] read_file_with_hash()
                              LanguageRegistry::get_by_extension()
                              SlocCounter::count(content) → CountResult
                              update cache with stats + metadata
              [if IgnoredFile] skip file
              [if Stats] collect_file_stats() → FileStatistics { path, stats, language }
              progress.inc()
         → progress.finish()
         → [if !--no-cache] save_cache()
         → ProjectStatistics::new(file_stats)
         → [if --group-by lang] with_language_breakdown() → compute LanguageStats
         → [if --group-by dir] with_directory_breakdown() → compute DirectoryStats
         → [if --top N] with_top_files(N) → compute top files + average
         → [if --trend] TrendHistory::load_or_default() → compute_delta() → with_trend() → save history
         → StatsTextFormatter/StatsJsonFormatter/StatsMarkdownFormatter::format(stats)
         → write to stdout or --output file
```

## Data Flow (config commands)

```
config validate:
  config_path → read file → toml::from_str() → validate_config_semantics()
             → check: warn_threshold range, glob patterns validity, override paths, rules validity

config show:
  config_path → load_config() → format_config_text() or serde_json::to_string_pretty()
```

## Data Flow (baseline update command)

```
CLI args → load_config()
         → LanguageRegistry::with_custom_languages(config.languages)
         → GlobFilter::new(extensions, excludes)
         → [if gitignore enabled] GitAwareScanner::scan(paths) else DirectoryScanner::scan(paths)
         → ScanProgress::new(file_count, quiet)
         → for each file (parallel with rayon):
              process_file() → CheckResult
              [if Failed] collect (path, lines)
              progress.inc()
         → progress.finish()
         → for each violation:
              compute_file_hash(path) → SHA-256
              Baseline::set(path, lines, hash)
         → Baseline::save(output_path)
```

## Threshold Resolution (priority high→low)

1. `[[override]]` - path suffix match (by components: `legacy.rs` matches `src/legacy.rs`)
2. `[[path_rules]]` - glob pattern match (e.g., `src/generated/**`)
3. `[rules.*]` - extension match
4. `[default]` - fallback

## Dependencies

- `clap` v4 - CLI parsing
- `serde` + `toml` + `serde_json` - config/output serialization
- `walkdir` - directory traversal
- `globset` - glob pattern matching
- `rayon` - parallel file processing
- `indicatif` - progress bar display
- `gix` - git integration (--diff mode)
- `sha2` - SHA-256 hashing (baseline, cache, remote config cache)
- `regex` - function pattern matching (split suggestions)
- `reqwest` - HTTP client for remote config fetching (blocking + rustls-tls)
- `thiserror` - error handling

## Test Files

Each module has `*_tests.rs` sibling file. Run: `make ci` or `cargo test`

