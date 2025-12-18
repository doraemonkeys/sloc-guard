# sloc-guard Implementation Plan

> **Doc Maintenance**: Keep concise, avoid redundancy, clean up outdated content promptly to reduce AI context usage.

## Quick Reference

```
Exit Codes: 0=pass, 1=threshold exceeded, 2=config error, 3=IO error
Lint: make ci
```

## Performance Notes

> **Completed optimizations**: Parallel processing (rayon), HashSet for extensions, pre-indexed rule lookup, streaming file reading for large files (>10MB), merged file read and hash computation (single read pass on cache miss).
>
> **Future considerations**: When adding new features, maintain these patterns:
> - Use `par_iter()` for file processing loops
> - Prefer O(1) lookups (HashMap/HashSet) over linear searches
> - Use `BufReader` for large file handling

---

## Completed (Compressed)

All modules in PROJECT_OVERVIEW.md Module Map are implemented. Additional completed features:

- **Phase 1-3**: Core MVP, Color Support, Git Diff Mode, Git-Aware Exclude
- **Phase 4**: Path-Based Rules, Inline Ignore (file/block/next), Strict Mode, Baseline (format/update/compare), SARIF Output, Progress Bar, File Hash Cache, Per-rule warn_threshold, Override with Reason, Custom Language Definition, Config Inheritance (local extends)
- **Phase 5**: Language Breakdown (--group-by lang), Top-N & Metrics (--top N), Markdown Output

---

## Phase 4: Advanced Features (Pending)

### Task 4.8b-1: Remote Config Fetch

Location: `src/config/loader.rs`, `Cargo.toml`

```
- Add reqwest dependency (blocking feature)
- Implement fetch_remote_config(url) → Result<String>
- Error handling: timeout, 404, invalid URL
```

### Task 4.8b-2: Remote Config Cache

Location: `src/config/cache.rs` (new)

```
- Cache path: ~/.cache/sloc-guard/configs/
- Hash URL as filename
- Check cache freshness (e.g., 1 hour TTL)
- Integrate with fetch_remote_config
```

### Task 4.8b-3: --no-extends Flag

Location: `src/cli.rs`, `src/config/loader.rs`

```
- Add --no-extends global CLI flag
- Skip extends resolution when flag set
- Update config loading logic
```

### Task 4.10: Split Suggestions (--fix)

Location: `src/analyzer/` (new), `src/cli.rs`, `src/output/*.rs`

```
- Add --fix flag to check command
- Analyze code structure to identify function/class boundaries
- When file exceeds threshold, suggest split points:
  - Detect function definitions (fn, def, function, etc.)
  - Group consecutive functions by logical cohesion
  - Estimate line count per split chunk
- Output format: function name, line range, suggested new file
- Language-specific parsers: Rust, Go, Python, JS/TS, C/C++
- Text output: show suggestions inline with failed files
- JSON/SARIF: include "suggestions" array in results
```

---

## Phase 5: Statistics Extension (Pending)

### Task 5.1c: Directory Statistics

Location: `src/output/stats.rs`

```
- Per-directory breakdown
- Add --group-by dir option
```

### Task 5.2: Trend Tracking

Location: `src/stats/trend.rs`

```
- Store historical stats in .sloc-guard-history.json
- Show delta from previous run
```

### Task 5.3a: HTML Structure + Summary

Location: `src/output/html.rs`

```
- Create HtmlFormatter with --report flag
- HTML skeleton with summary table (total files, passed, failed, warnings)
- Embedded CSS for standalone file
```

### Task 5.3b: HTML File List

Location: `src/output/html.rs`

```
- File results table (path, lines, limit, status)
- Sortable columns (client-side JS optional)
- Status filtering (show all/failed/warning only)
```

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
| **1. Short-term** | 4.10 Split Suggestions (--fix) |
| **2. Deferred** | 4.8b-1 Remote Config Fetch |
| | 4.8b-2 Remote Config Cache |
| | 4.8b-3 --no-extends Flag |
| | 5.1c Directory Statistics |
| | 5.2 Trend Tracking |
| | 5.3a-d HTML Report |
| | Phase 6 |

---

## Architecture Notes

### Dependency Flow

```
main.rs
  -> cli (parse args)
  -> commands/* (init, config)
  -> config/loader (load config)
  -> scanner (find files)
  -> language/registry (get comment syntax)
  -> counter/sloc (count lines)
  -> checker/threshold (check limits)
  -> output/* (format results)
```
