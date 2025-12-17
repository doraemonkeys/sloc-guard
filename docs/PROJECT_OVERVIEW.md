# sloc-guard Project Overview

**SLOC (Source Lines of Code) enforcement tool** - enforces file size limits by counting code lines, excluding comments and blanks.

## Quick Reference

```
Rust CLI tool | Clap v4 | TOML config | Exit: 0=pass, 1=threshold exceeded, 2=config error
```

## Module Map

| Module | File(s) | Purpose |
|--------|---------|---------|
| `cli` | `cli.rs` | Clap-derived CLI: `check`, `stats`, `init`, `config` commands |
| `config/model` | `config/model.rs` | `Config`, `DefaultConfig`, `RuleConfig`, `ExcludeConfig`, `FileOverride` |
| `config/loader` | `config/loader.rs` | `FileConfigLoader` - loads `.sloc-guard.toml` or `~/.config/sloc-guard/config.toml` |
| `language/registry` | `language/registry.rs` | `LanguageRegistry`, `Language`, `CommentSyntax` - predefined: Rust/Go/Python/JS/TS/C/C++ |
| `counter/comment` | `counter/comment.rs` | `CommentDetector` - detects single/multi-line comments |
| `counter/sloc` | `counter/sloc.rs` | `SlocCounter` → `LineStats{total, code, comment, blank}` |
| `scanner/filter` | `scanner/filter.rs` | `GlobFilter` - extension + exclude pattern filtering |
| `scanner/mod` | `scanner/mod.rs` | `DirectoryScanner` - walkdir-based file discovery |
| `checker/threshold` | `checker/threshold.rs` | `ThresholdChecker` → `CheckResult{status, stats, limit}` |
| `output/text` | `output/text.rs` | `TextFormatter` - human-readable output with icons |
| `output/json` | `output/json.rs` | `JsonFormatter` - structured JSON output |
| `error` | `error.rs` | `SlocGuardError` enum: Config/FileRead/InvalidPattern/Io/TomlParse/JsonSerialize |
| `main` | `main.rs` | Command dispatch, `run_check` implemented, others TODO |

## Key Types

```rust
// Config priority: CLI args > config file > defaults
Config { default: DefaultConfig, rules: HashMap<String, RuleConfig>, exclude: ExcludeConfig, overrides: Vec<FileOverride> }
DefaultConfig { max_lines: 500, extensions: [rs,go,py,js,ts,c,cpp], include_paths, skip_comments: true, skip_blank: true, warn_threshold: 0.9 }

// Line counting
LineStats { total, code, comment, blank }  // sloc() returns code count
CommentSyntax { single_line: Vec<&str>, multi_line: Vec<(start, end)> }

// Check results
CheckStatus::Passed | Warning | Failed
CheckResult { path, status, stats, limit }
```

## Data Flow (check command)

```
CLI args → load_config() → apply_cli_overrides()
         → GlobFilter::new(extensions, excludes)
         → DirectoryScanner::scan(paths)
         → for each file:
              LanguageRegistry::get_by_extension()
              SlocCounter::count(content) → LineStats
              ThresholdChecker::check(path, stats) → CheckResult
         → TextFormatter/JsonFormatter::format(results)
         → write to stdout or --output file
```

## Threshold Resolution (priority high→low)

1. `[[override]]` - path exact match
2. `[rules.*]` - extension match
3. `[default]` - fallback

## CLI Structure

```
sloc-guard [--verbose/-v] [--quiet/-q] [--color auto|always|never] [--no-config]
  check [paths] [-c config] [--max-lines N] [--ext rs,go] [-x exclude] [-I include]
        [--no-skip-comments] [--no-skip-blank] [--warn-threshold 0.9]
        [-f text|json|sarif|markdown] [-o file] [--warn-only] [--diff ref]
  stats [paths] [-c config] [--ext] [-x] [-I] [-f] [-o]
  init [-o path] [--force]
  config validate [-c config]
  config show [-c config] [-f text|json]
```

## Implementation Status

| Feature | Status |
|---------|--------|
| `check` command | ✅ Working |
| `stats` command | ❌ TODO stub |
| `init` command | ❌ TODO stub |
| `config validate/show` | ❌ TODO stub |
| SARIF output | ❌ Pending |
| Markdown output | ❌ Pending |
| `--diff` git mode | ❌ Pending |
| Color output | ❌ Pending (colored crate ready) |

## Dependencies

- `clap` v4 - CLI parsing
- `serde` + `toml` + `serde_json` - config/output serialization
- `walkdir` - directory traversal
- `globset` - glob pattern matching
- `gix` - git integration (unused yet)
- `colored` - terminal colors (unused yet)
- `thiserror` - error handling

## Test Files

Each module has `*_tests.rs` sibling file. Run: `make ci` or `cargo test`

