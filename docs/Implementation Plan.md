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

---

## Completed (Compressed)

All modules in PROJECT_OVERVIEW.md Module Map are implemented. Additional completed features:

- **Phase 1-3**: Core MVP, Color Support, Git Diff Mode, Git-Aware Exclude
- **Phase 4**: Path-Based Rules, Inline Ignore (file/block/next), Strict Mode, Baseline (format/update/compare), SARIF Output, Progress Bar, File Hash Cache, Per-rule warn_threshold, Override with Reason, Custom Language Definition, Config Inheritance (local extends), Split Suggestions (--fix), Remote Config Support (http/https extends with caching, --no-extends flag)
- **Phase 5**: Language Breakdown (--group-by lang), Top-N & Metrics (--top N), Markdown Output, Directory Statistics (--group-by dir), Trend Tracking (--trend, .sloc-guard-history.json), HTML Report (--format html, summary + file list + sortable columns + status filtering)

---

## Phase 5: Statistics Extension (Pending)

### Task 5.3c: HTML Charts (Pure CSS)

Location: `src/output/html.rs`

```
- File size distribution bar chart (pure CSS)
- Language/extension breakdown pie chart
- No external dependencies
```

### Task 5.3d: HTML Trend Visualization

Location: `src/output/html.rs`

```
- Integrate with .sloc-guard-history.json (if exists)
- Line chart showing SLOC over time
- Delta indicators (+/-) from previous run
```

---

## Phase 6: CI/CD Support (Pending)

### Task 6.1: GitHub Action

```
- Create reusable GitHub Action
- Input: paths, config-path, fail-on-warning
- Output: total-files, passed, failed, warnings
```

### Task 6.2: Pre-commit Hook

```
- Document .pre-commit-config.yaml setup
- Support staged files only mode
```

---

## Priority Order

| Priority | Tasks |
|----------|-------|
| **1. Deferred** | 5.3c-d HTML Report (charts, trends) |
| | Phase 6 |

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
  -> output/* (format results)
```
