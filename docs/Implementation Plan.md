# sloc-guard Implementation Plan

> **Doc Maintenance**: Keep concise, avoid redundancy, clean up outdated content promptly to reduce AI context usage.

## Current Status

### Completed Components

| Module | Status | Description |
|--------|--------|-------------|
| `cli` | Done | CLI with check (--baseline, --no-cache), stats (--no-cache), init, config, baseline commands + global options (verbose, quiet, color, no-config) |
| `config/model` | Partial | Config, DefaultConfig, RuleConfig, ExcludeConfig, FileOverride, PathRule, strict (pending: per-rule warn_threshold) |
| `config/loader` | Done | FileConfigLoader with search order: CLI -> project .sloc-guard.toml -> $HOME/.config/sloc-guard/config.toml -> defaults |
| `language/registry` | Done | Language definitions with comment syntax (Rust, Go, Python, JS/TS, C/C++) |
| `counter/comment` | Done | CommentDetector for single/multi-line comment detection |
| `counter/sloc` | Done | SlocCounter with LineStats, CountResult, inline ignore-file directive |
| `scanner/filter` | Done | GlobFilter for extension and exclude pattern filtering |
| `scanner/mod` | Done | DirectoryScanner with walkdir integration |
| `checker/threshold` | Done | ThresholdChecker with override > path_rules > rule > default priority, CheckStatus: Passed/Warning/Failed/Grandfathered |
| `output/text` | Done | TextFormatter with color support (ColorMode: Auto/Always/Never), status icons, summary, grandfathered count |
| `output/json` | Done | JsonFormatter with structured output including grandfathered count |
| `output/sarif` | Done | SarifFormatter with SARIF 2.1.0 output for GitHub Code Scanning |
| `output/stats` | Done | StatsTextFormatter and StatsJsonFormatter for stats command |
| `output/progress` | Done | ScanProgress with indicatif, disabled in quiet mode or non-TTY |
| `git/diff` | Done | GitDiff with gix for --diff mode (changed files since reference) |
| `baseline` | Done | Baseline, BaselineEntry, compute_file_hash, `baseline update` command, `--baseline` flag for check |
| `cache` | Done | Cache, CacheEntry, CachedLineStats, compute_config_hash for file hash caching |
| `error` | Done | SlocGuardError enum with thiserror |
| `commands/config` | Done | `run_config`, `validate_config_semantics`, `format_config_text` |
| `commands/init` | Done | `run_init`, `generate_config_template` |
| `main` | Done | Command dispatch, `run_check`, `run_stats`, `run_baseline` |

---

### Exit Codes

| Code | Constant | Description |
|------|----------|-------------|
| 0 | `EXIT_SUCCESS` | All checks passed (or `--warn-only` mode) |
| 1 | `EXIT_FAILURE` | One or more files exceeded threshold |
| 2 | `EXIT_CONFIG_ERROR` | Configuration file error (syntax or semantic) |
| 3 | `EXIT_IO_ERROR` | File system error (permission denied, not found) |

Note: When `--warn-only` is set, exit code 1 is converted to 0.

---

lint:
```
make ci
```

## Performance Notes

> **Completed optimizations**: Parallel processing (rayon), HashSet for extensions, pre-indexed rule lookup, streaming file reading for large files (>10MB).
>
> **Future considerations**: When adding new features, maintain these patterns:
> - Use `par_iter()` for file processing loops
> - Prefer O(1) lookups (HashMap/HashSet) over linear searches
> - Use `BufReader` for large file handling
>
> **Pending optimizations**:
> - [ ] Merge file read and hash computation: Currently on cache miss, file is read twice (once for hash, once for line counting). Combine into single read pass.

---

## Completed Phases (Compressed)

| Phase | Tasks | Status |
|-------|-------|--------|
| **Phase 1: Core MVP** | 1.1 FileConfigLoader, 1.2 run_check, 1.3 run_stats, 1.4 run_init, 1.5 run_config | ✅ All Done |
| **Phase 2.1** | Color Support (TextFormatter with Auto/Always/Never) | ✅ Done |
| **Phase 3.1** | Git Diff Mode (gix-based --diff) | ✅ Done |
| **Phase 4.3** | Path-Based Rules ([[path_rules]] with glob patterns) | ✅ Done |
| **Phase 4.6a** | Inline Ignore (// sloc-guard:ignore-file in first 10 lines) | ✅ Done |
| **Phase 4.9** | Strict Mode (--strict flag, config option) | ✅ Done |
| **Phase 4.1a** | Baseline File Format (Baseline, BaselineEntry, SHA-256 hash) | ✅ Done |
| **Phase 4.1b** | Baseline Update Command (`baseline update` with --output) | ✅ Done |
| **Phase 4.1c** | Baseline Compare (`--baseline` flag, grandfathered status) | ✅ Done |
| **Phase 2.2** | SARIF Output (SarifFormatter with 2.1.0 spec, GitHub Code Scanning) | ✅ Done |
| **Phase 2.4** | Progress Bar (ScanProgress with indicatif, auto-disabled in quiet/non-TTY) | ✅ Done |
| **Phase 4.7a** | File Hash Cache (Cache, CacheEntry, compute_config_hash) | ✅ Done |
| **Phase 4.7b** | Cache Integration (--no-cache flag, cache in check/stats commands) | ✅ Done |

---

## Phase 2: Output Enhancements (P1)

### Task 2.3: Markdown Output

Location: `src/output/markdown.rs`

```
- Create MarkdownFormatter struct
- Generate table-based output
- Include summary section
```

---

## Phase 3: Git Integration (P1)

### Task 3.2: Git-Aware Exclude

Location: `src/scanner/filter.rs`

```
- Respect .gitignore patterns via gix dirwalk
- Make git-aware scanning optional (--no-gitignore flag)
```

---

## Phase 4: Advanced Features (P2)

### Task 4.2: Per-rule warn_threshold (Partial)

Location: `src/config/model.rs`, `src/checker/threshold.rs`

```
- [x] Add warn_threshold to DefaultConfig (default 0.9)
- [ ] Allow per-rule: [rules.rust] warn_threshold = 0.85
- [x] Support --warn-threshold CLI override
```

### Task 4.4: Override with Reason

Location: `src/config/model.rs`

```
- Add optional reason field to [[override]]
- Show reason in verbose output
```

### Task 4.5: Custom Language Definition

Location: `src/config/model.rs`, `src/language/registry.rs`

```
- Add [languages.<name>] section in config
- Allow: extensions, single_line_comments, multi_line_comments
- Override built-in if same extension
```

### Task 4.6b: Inline Ignore (block/next)

Location: `src/counter/sloc.rs`

```
- Support: // sloc-guard:ignore-next N
- Support: // sloc-guard:ignore-start / ignore-end
- Exclude matched lines from count
```

### Task 4.8a: Config Inheritance (local)

Location: `src/config/loader.rs`

```
- Add "extends" field to config (local paths only)
- Load base config first, merge local overrides
- Cycle detection for recursive extends
```

### Task 4.8b: Config Inheritance (URL)

Location: `src/config/loader.rs`

```
- Support extends = "https://..."
- Add --no-extends CLI flag
- Cache downloaded configs
```

---

## Phase 5: Statistics Extension (P2)

### Task 5.1a: Language Breakdown

Location: `src/output/stats.rs`

```
- Group SLOC by language/extension
- Add --group-by lang option to stats command
```

### Task 5.1b: Top-N & Metrics

Location: `src/output/stats.rs`

```
- Add --top N flag to stats command
- Show top N largest files
- Show average file size
```

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

## Phase 6: CI/CD Support (P2)

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

## Phase 7: Future Enhancements (P3)

### Task 7.1a: Function Analysis (Rust)

```
- Use tree-sitter-rust to parse function boundaries
- Count lines per function
- Add function_max_lines to config
```

### Task 7.1b: Function Analysis (Other Languages)

```
- Extend to Go, Python, JS/TS as needed
- Separate tree-sitter grammars per language
```

---

## Priority Order

| Priority | Tasks | Effort |
|----------|-------|--------|
| **1. Short-term** | 5.1a Language Breakdown | ~2h |
| | 5.1b Top-N & Metrics | ~2h |
| **2. Medium** | 3.2 Git-Aware Exclude | ~3h |
| | 2.3 Markdown Output | ~2h |
| | 4.2 Per-rule warn_threshold | ~1h |
| | 4.4 Override Reason | ~1h |
| **3. Lower** | 4.5 Custom Languages | ~3h |
| | 4.6b Inline Ignore (block/next) | ~2h |
| | 4.8a Config Inheritance (local) | ~2h |
| **4. Deferred** | 4.8b Config Inheritance (URL) | ~2h |
| | 5.1c Directory Statistics | ~2h |
| | 5.2 Trend Tracking | ~3h |
| | 5.3a HTML Structure + Summary | ~2h |
| | 5.3b HTML File List | ~2h |
| | 5.3c HTML Charts (Pure CSS) | ~2h |
| | 5.3d HTML Trend Visualization | ~2h |
| | Phase 6, 7 | TBD |

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
