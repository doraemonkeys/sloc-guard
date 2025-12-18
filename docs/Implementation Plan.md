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

All modules in PROJECT_OVERVIEW.md Module Map are implemented. Additional completed features:

- **Phase 1-3**: Core MVP, Color Support, Git Diff Mode, Git-Aware Exclude
- **Phase 4**: Path-Based Rules, Inline Ignore (file/block/next), Strict Mode, Baseline (format/update/compare), SARIF Output, Progress Bar, File Hash Cache, Per-rule warn_threshold, Override with Reason, Custom Language Definition, Config Inheritance (local extends), Split Suggestions (--fix), Remote Config Support (http/https extends with caching, --no-extends flag)
- **Phase 5 (Partial)**: Language Breakdown (--group-by lang), Top-N & Metrics (--top N), Markdown Output, Directory Statistics (--group-by dir), Trend Tracking (--trend, .sloc-guard-history.json), HTML Report (--format html, summary + file list + sortable columns + status filtering), Structure Config Schema ([structure] + [[structure.rules]])

---

## Phase 5: Directory Structure Guard (New)

Focus: Enforce file and directory count limits per directory to prevent architectural mess.

### Task 5.1: Configuration Schema ✓
Completed: `StructureConfig` and `StructureRule` in `src/config/model.rs`

### Task 5.2: Structure Analyzer ✓
Completed: `src/checker/structure.rs`
- `StructureChecker` with glob-based ignore patterns and per-directory rules
- `DirStats { file_count, dir_count }` - immediate children counts
- `StructureViolation { path, violation_type, actual, limit }`
- `ViolationType::FileCount | DirCount`
- Recursive directory scanning using metadata only (no file opening)

### Task 5.3: Integration & Output
Location: `src/commands/check.rs`, `src/output/*`
```
- Update `check` command to run structure analysis
- Update OutputFormatters to display structure errors:
  - Text: Distinct error section or interleaved
  - JSON/SARIF: Add structure violations to results
  - HTML: Add "Structure" tab or section in summary
```

---

## Phase 6: Statistics Extension (Pending)

### Task 6.1: HTML Charts (Pure CSS)
Location: `src/output/html.rs`
```
- File size distribution bar chart (pure CSS)
- Language/extension breakdown pie chart
- No external dependencies
```

### Task 6.2: HTML Trend Visualization
Location: `src/output/html.rs`
```
- Integrate with .sloc-guard-history.json (if exists)
- Line chart showing SLOC over time
- Delta indicators (+/-) from previous run
```

---

## Phase 7: CI/CD Support (Pending)

### Task 7.1: GitHub Action
```
- Create reusable GitHub Action
- Input: paths, config-path, fail-on-warning
- Output: total-files, passed, failed, warnings
```

### Task 7.2: Pre-commit Hook
```
- Document .pre-commit-config.yaml setup
- Support staged files only mode
```

---

## Priority Order

| Priority | Tasks |
|----------|-------|
| **1. Structure Guard** | Phase 5 (Config, Analyzer, Integration) |
| **2. Deferred** | 6.1-6.2 HTML Charts/Trends |
| | Phase 7 CI/CD |

---

## Architecture Notes

### Dependency Flow

```
main.rs (CLI parsing + dispatch)
  -> commands/check | stats | baseline_cmd | init | config
  -> commands/common (shared: load_config, cache, scan paths)
  -> config/loader (load config)
  -> scanner (find files)
  -> language/registry (get comment syntax)
  -> counter/sloc (count lines)
  -> checker/threshold (check limits)
  -> checker/structure (NEW: check structure limits)
  -> output/* (format results)
```
