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

---

## Phase 6: CLI Symmetry & Usability (Pending)

### Task 6.1: check Structure Parameters
Location: `src/cli.rs`, `src/commands/check.rs`
```
- Add --max-files, --max-dirs to CheckArgs
- Semantics: Override [structure] defaults only (not [[structure.rules]])
- REQUIRE explicit <PATH> argument when using --max-files/--max-dirs
  - Error if used without <PATH>: "error: --max-files/--max-dirs require a target <PATH>"
  - Rationale: Global structure limits are rarely meaningful; per-directory use is intuitive
- Help text: "Overrides default limits for <PATH>; rules take precedence"
```

### Task 6.2: --diff Optional Parameter
Location: `src/cli.rs`, `src/commands/check.rs`
```
- Change --diff from Option<String> to num_args = 0..=1
- Default to "HEAD" when --diff provided without value
- Update help text to clarify behavior
```

### Task 6.3: --history-file Parameter
Location: `src/cli.rs`, `src/commands/stats.rs`
```
- Add --history-file <PATH> to StatsArgs (default: .sloc-guard-history.json)
- Pass custom path to TrendHistory::load/save
```

### Task 6.4: Documentation Clarification
Location: `docs/`, CLI help text
```
- S1: Clarify paths (scan roots) vs --include (allowlist filter)
- S2: Document CLI override scope (overrides [content]/[structure] defaults, not rules)
- S3: Document --diff default behavior (HEAD)
- S4: Clarify --diff structure semantics: limits reporting scope, but counts use full disk state
```

### Task 6.6: check --report-json (Stats in Check)
Location: `src/cli.rs`, `src/commands/check.rs`
```
- Add --report-json <PATH> to CheckArgs
- Output ProjectStatistics alongside check results
- Avoids running stats separately in CI pipelines
```

---

## Phase 6.5: Baseline Consolidation (Pending)

### Task 6.5.1: check --update-baseline
Location: `src/cli.rs`, `src/commands/check.rs`, `src/commands/baseline_cmd.rs`
```
- Add --update-baseline[=MODE] to CheckArgs (optional value)
- MODE values:
  - all (default): Replace baseline with current violations
  - content: Update SLOC violations only
  - structure: Update directory violations only
  - new: Add-only mode (add new violations, preserve existing entries)
    - Prevents accidental removal of entries for fixed files
    - Useful for incremental adoption in legacy projects
- Merge baseline update logic into run_check()
- Deprecate baseline update subcommand (warn + redirect)
```

---

## Phase 6.9: CLI Naming Cleanup (Pending)

### Task 6.9.1: Rename Misleading Parameters
Location: `src/cli.rs`
```
- --fix → --suggest (or --suggestions)
- --no-skip-comments → --count-comments
- --no-skip-blank → --count-blank
- Update all references in commands and help text
```

### Task 6.9.2: config show Format Enum
Location: `src/cli.rs`, `src/commands/config.rs`
```
- Define ConfigOutputFormat enum (Text, Json)
- Replace format: String with typed enum in ConfigArgs
- Add value_parser for validation
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

### Task 9.1: explain Command (High Priority)
Location: `src/cli.rs`, `src/commands/explain.rs` (new)
```
- New command: sloc-guard explain <PATH>
- Output: Which rule matched, override applied, final effective limits
- Shows config source (local/remote) and rule chain for debugging
- Essential for troubleshooting complex configurations
```

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
| **1. CLI Usability (High)** | 6.1-6.2 Structure params, --diff optional |
| **2. Debugging (High Value)** | 9.1 explain command (essential for complex configs) |
| **3. CLI Enhancement (Medium)** | 6.3-6.4, 6.6 --history-file, docs, --report-json |
| **4. Baseline Consolidation** | 6.5.1 check --update-baseline with granularity |
| **5. CLI Cleanup (Low)** | 6.9.1-6.9.2 Renaming, format enum |
| **6. Structure Enhancements** | 9.2 max_depth, 9.4 whitelist mode |
| **7. Visualization** | 7.1-7.2 HTML Charts/Trends |
| **8. UX Improvements** | 9.3 Smart init |
| **9. CI/CD** | 8.1-8.2 GitHub Action & Pre-commit |

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
