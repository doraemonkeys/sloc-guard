# sloc-guard Project Overview

> **Doc Maintenance**: Keep concise, avoid redundancy, clean up outdated content promptly to reduce AI context usage.

**SLOC (Source Lines of Code) enforcement tool** - enforces file size limits by counting code lines, excluding comments and blanks.

## Quick Reference

```
Rust CLI tool | Clap v4 | TOML config | Exit: 0=pass, 1=threshold exceeded, 2=config error
```

## Module Map

| Module | File(s) | Purpose |
|--------|---------|---------|
| `cli` | `cli.rs` | Clap-derived CLI: `check` (--baseline), `stats`, `init`, `config`, `baseline` commands |
| `config/model` | `config/model.rs` | `Config`, `DefaultConfig`, `RuleConfig`, `ExcludeConfig`, `FileOverride`, `PathRule` |
| `config/loader` | `config/loader.rs` | `FileConfigLoader` - loads `.sloc-guard.toml` or `~/.config/sloc-guard/config.toml` |
| `language/registry` | `language/registry.rs` | `LanguageRegistry`, `Language`, `CommentSyntax` - predefined: Rust/Go/Python/JS/TS/C/C++ |
| `counter/comment` | `counter/comment.rs` | `CommentDetector` - detects single/multi-line comments |
| `counter/sloc` | `counter/sloc.rs` | `SlocCounter` → `CountResult{Stats(LineStats), IgnoredFile}`, inline ignore directive |
| `scanner/filter` | `scanner/filter.rs` | `GlobFilter` - extension + exclude pattern filtering |
| `scanner/mod` | `scanner/mod.rs` | `DirectoryScanner` - walkdir-based file discovery |
| `checker/threshold` | `checker/threshold.rs` | `ThresholdChecker` with pre-indexed extension lookup → `CheckResult{status, stats, limit}` |
| `git/diff` | `git/diff.rs` | `GitDiff` - gix-based changed files detection for `--diff` mode |
| `baseline` | `baseline/types.rs` | `Baseline`, `BaselineEntry` - baseline file for grandfathering violations |
| `cache` | `cache/types.rs` | `Cache`, `CacheEntry`, `CachedLineStats`, `compute_config_hash` - file hash caching |
| `output/text` | `output/text.rs` | `TextFormatter`, `ColorMode` - human-readable output with color and verbose support |
| `output/json` | `output/json.rs` | `JsonFormatter` - structured JSON output |
| `output/sarif` | `output/sarif.rs` | `SarifFormatter` - SARIF 2.1.0 output for GitHub Code Scanning |
| `output/stats` | `output/stats.rs` | `StatsTextFormatter`, `StatsJsonFormatter` - stats command output |
| `error` | `error.rs` | `SlocGuardError` enum: Config/FileRead/InvalidPattern/Io/TomlParse/JsonSerialize/Git |
| `main` | `main.rs` | Command dispatch: `run_check`, `run_stats`, `run_init`, `run_config`, `run_baseline` |

## Key Types

```rust
// Config priority: CLI args > config file > defaults
Config { default: DefaultConfig, rules: HashMap<String, RuleConfig>, path_rules: Vec<PathRule>, exclude: ExcludeConfig, overrides: Vec<FileOverride> }
DefaultConfig { max_lines: 500, extensions: [rs,go,py,js,ts,c,cpp], include_paths, skip_comments: true, skip_blank: true, warn_threshold: 0.9, strict: false }
PathRule { pattern: String, max_lines: usize, warn_threshold: Option<f64> }  // glob patterns like "src/generated/**"

// Line counting
LineStats { total, code, comment, blank }  // sloc() returns code count
CountResult::Stats(LineStats) | IgnoredFile  // IgnoredFile when "// sloc-guard:ignore-file" in first 10 lines
CommentSyntax { single_line: Vec<&str>, multi_line: Vec<(start, end)> }

// Check results
CheckStatus::Passed | Warning | Failed | Grandfathered
CheckResult { path, status, stats, limit }

// Output formatting
ColorMode::Auto | Always | Never  // controls ANSI color output
TextFormatter::with_verbose(mode, verbose)  // verbose >= 1 shows passed files

// Stats results (no threshold checking)
FileStatistics { path, stats: LineStats }
ProjectStatistics { files, total_files, total_lines, total_code, total_comment, total_blank }

// Git integration
GitDiff::discover(path) → GitDiff  // Finds git repo from path
ChangedFiles::get_changed_files(base_ref) → HashSet<PathBuf>  // Files changed since reference

// Baseline (grandfathering violations)
Baseline { version: u32, files: HashMap<String, BaselineEntry> }  // .sloc-guard-baseline.json
BaselineEntry { lines: usize, hash: String }  // SHA-256 content hash
compute_file_hash(path) → String  // SHA-256 of file content

// Cache (file hash caching)
Cache { version: u32, config_hash: String, files: HashMap<String, CacheEntry> }  // .sloc-guard-cache
CacheEntry { hash: String, stats: CachedLineStats }  // file content hash + cached stats
compute_config_hash(config) → String  // SHA-256 of serialized config
```

## Data Flow (check command)

```
CLI args → load_config() → apply_cli_overrides()
         → [if --baseline] load_baseline() → Baseline
         → GlobFilter::new(extensions, excludes)
         → DirectoryScanner::scan(paths)
         → [if --diff] GitDiff::get_changed_files() → filter to changed only
         → for each file:
              LanguageRegistry::get_by_extension()
              SlocCounter::count(content) → CountResult
              [if IgnoredFile] skip file (inline ignore directive)
              [if Stats] ThresholdChecker::check(path, stats) → CheckResult
         → [if baseline] apply_baseline_comparison() → mark Failed as Grandfathered
         → TextFormatter/JsonFormatter/SarifFormatter::format(results)
         → write to stdout or --output file
```

## Data Flow (stats command)

```
CLI args → load_config()
         → GlobFilter::new(extensions, excludes)
         → DirectoryScanner::scan(paths)
         → for each file:
              LanguageRegistry::get_by_extension()
              SlocCounter::count(content) → CountResult
              [if IgnoredFile] skip file
              [if Stats] collect_file_stats() → FileStatistics
         → ProjectStatistics::new(file_stats)
         → StatsTextFormatter/StatsJsonFormatter::format(stats)
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
         → GlobFilter::new(extensions, excludes)
         → DirectoryScanner::scan(paths)
         → for each file:
              process_file() → CheckResult
              [if Failed] collect (path, lines)
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
- `gix` - git integration (--diff mode)
- `sha2` - SHA-256 hashing (baseline, cache)
- `thiserror` - error handling

## Test Files

Each module has `*_tests.rs` sibling file. Run: `make ci` or `cargo test`

