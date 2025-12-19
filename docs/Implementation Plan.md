# sloc-guard Implementation Plan

> **Doc Maintenance**: Keep concise, avoid redundancy, clean up outdated content promptly to reduce AI context usage.

## Quick Reference

```
Exit Codes: 0=pass, 1=threshold exceeded, 2=config error, 3=IO error
Lint: make ci
```

## Performance Notes

> **Completed optimizations**: Parallel processing (rayon), HashSet for extensions, pre-indexed rule lookup, streaming file reading for large files (>10MB), metadata-based cache validation (mtime + size check avoids file read on cache hit).
>
> **Future considerations**: When adding new features, maintain these patterns:
> - Use `par_iter()` for file processing loops
> - Prefer O(1) lookups (HashMap/HashSet) over linear searches
> - Use `BufReader` for large file handling
> - **Structure Checks**: Perform directory entry counting using metadata only (no file opening).

---

## Completed (Compressed)

All modules in PROJECT_OVERVIEW.md Module Map are implemented.

- **Phase 1-3**: Core MVP, Color Support, Git Diff Mode, Git-Aware Exclude
- **Phase 4**: Path-Based Rules, Inline Ignore, Strict Mode, Baseline, SARIF Output, Progress Bar, File Hash Cache, Per-rule warn_threshold, Override with Reason, Custom Language Definition, Config Inheritance, Split Suggestions, Remote Config.
- **Phase 5 (Partial)**: Language Breakdown, Top-N & Metrics, Markdown Output, Directory Statistics, Trend Tracking, HTML Report, Structure Guard.
- **Phase 5.5 (Refactoring & V2 Config)**:
  - **Architecture**: Scanner/Structure separation, `ScannerConfig` vs `ContentConfig`, `CheckResult` enum refactor, Dependency Injection (Context) implementation.
  - **Config V2**: Separated `[[content.override]]` vs `[[structure.override]]` (with required reason), Versioning (auto-migration v1→v2), `warn_threshold` for structure, Unlimited (`-1`) limits.
  - **UX**: Extension-based rule sugar (`[content.languages.rs]`), Explicit Rule Priority (Override > Rule > Lang > Default), Structure pattern semantics clarification, Renamed `structure.count_exclude`.
- **Phase 6 (Partial)**:
  - **6.1**: `--max-files`, `--max-dirs` CLI params for `check` command. Requires explicit `<PATH>` argument, overrides `[structure]` defaults (not rules).
  - **6.2**: `--diff` optional parameter. Defaults to `HEAD` when provided without value (`--diff` same as `--diff HEAD`).
  - **6.3**: `--history-file` parameter for `stats` command. Custom path for trend history file (default: `.sloc-guard-history.json`).
  - **6.4**: Documentation Clarification - CLI help text updates (paths vs --include, CLI override scope, --diff behavior), README.md creation.
  - **6.5.1**: `--update-baseline[=MODE]` for `check` command. Modes: `all`(default), `content`, `structure`, `new`. Baseline V2 format (tagged enum for content/structure entries). V1 auto-migration. Deprecates `baseline update` subcommand.
  - **6.6**: `--report-json <PATH>` for `check` command. Outputs `ProjectStatistics` JSON alongside check results (avoids separate stats run in CI).
  - **6.9.1**: CLI parameter renames: `--fix` → `--suggest`, `--no-skip-comments` → `--count-comments`, `--no-skip-blank` → `--count-blank`.
  - **6.9.2**: `config show` format enum - `ConfigOutputFormat` (Text, Json) replaces String parameter.
- **Phase 9 (Partial)**:
  - **9.1**: `explain` command - shows which rules/overrides apply to a path, displays rule chain with match status.

---

## Phase 10: Code Quality & Testability (Complete)

### Task 10.1: IO Abstraction for Pure Unit Testing ✅
Location: `src/commands/check.rs`, `src/commands/context.rs`, `src/scanner/*.rs`
```
- Define Scanner trait (abstracts scan_files) - FileScanner trait with scan_all method
- Define FileReader trait (abstracts file reading in process_file_with_cache)
- Inject IO abstractions via CheckContext (scanner + file_reader fields)
- Enable pure unit testing of run_check_with_context without real file system
- CompositeScanner handles git/non-git fallback logic
```

### Task 10.2: Replace unwrap() with expect() ✅
Location: `src/config/loader.rs:133`
```
- .expect("key exists: iterating over collected keys")
- Documents intent for code path guaranteed by iteration
```

---

## Phase 7: Statistics Extension (Pending)

### Task 7.1: HTML Charts (Pure CSS)
Location: `src/output/html.rs`
```
- File size distribution bar chart (pure CSS)
- Language/extension breakdown pie chart
- No external dependencies
```

### Task 7.2: HTML Trend Visualization
Location: `src/output/html.rs`
```
- Integrate with .sloc-guard-history.json (if exists)
- Line chart showing SLOC over time
- Delta indicators (+/-) from previous run
```

---

## Phase 8: CI/CD Support (Pending)

### Task 8.1: GitHub Action
```
- Create reusable GitHub Action
- Input: paths, config-path, fail-on-warning
- Output: total-files, passed, failed, warnings
```

### Task 8.2: Pre-commit Hook
```
- Document .pre-commit-config.yaml setup
- Support staged files only mode
```

---

## Phase 9: Advanced Features (Pending)

### Task 9.2: Structure max_depth
Location: `src/config/structure.rs`, `src/checker/structure.rs`
```
- Add max_depth to [structure] and [[structure.rules]]
- Limits directory nesting depth (prevents deeply nested structures)
- StructureChecker tracks depth during traversal
```

### Task 9.3: init --detect (Smart Init)
Location: `src/commands/init.rs`
```
- Add --detect flag to init command
- Auto-detect project type (Cargo.toml→Rust, package.json→Node, etc.)
- Generate language-appropriate default rules
- Reduces configuration barrier for new users
```

### Task 9.4: Structure Whitelist Mode
Location: `src/config/structure.rs`, `src/checker/structure.rs`
```
- Add allow_extensions / allow_patterns to [[structure.rules]]
- Stricter than count_exclude: files not matching are violations
- Enforces architectural purity (e.g., only .rs in src/domain/models)
```

---

## Priority Order

| Priority | Tasks |
|----------|-------|
| ~~**1. Code Quality**~~ | ~~10.1 IO Abstraction, 10.2 expect() cleanup~~ ✅ |
| **2. Structure Enhancements** | 9.2 max_depth, 9.4 whitelist mode |
| **3. Visualization** | 7.1-7.2 HTML Charts/Trends |
| **4. UX Improvements** | 9.3 Smart init |
| **5. CI/CD** | 8.1-8.2 GitHub Action & Pre-commit |

---

## Architecture Notes

### Dependency Flow

```
main.rs (CLI parsing + dispatch)
  -> commands/check | stats | baseline_cmd | init | config
  -> context (shared: load_config, cache)
  -> config/loader (load config)
  -> scanner (find files)
  -> language/registry (get comment syntax)
  -> counter/sloc (count lines)
  -> checker/threshold (check limits)
  -> checker/structure (check structure limits)
  -> output/* (format results)
```
