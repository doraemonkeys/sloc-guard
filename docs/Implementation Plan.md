# sloc-guard Implementation Plan

> **Doc Maintenance**: Keep concise, avoid redundancy, clean up outdated content promptly to reduce AI context usage.

## Current Status

### Completed Components

| Module | Status | Description |
|--------|--------|-------------|
| `cli` | Done | CLI with check, stats, init, config commands + global options (verbose, quiet, color, no-config) |
| `config/model` | Partial | Config, DefaultConfig, RuleConfig, ExcludeConfig, FileOverride, PathRule (pending: per-rule warn_threshold) |
| `config/loader` | Done | FileConfigLoader with search order: CLI -> project .sloc-guard.toml -> $HOME/.config/sloc-guard/config.toml -> defaults |
| `language/registry` | Done | Language definitions with comment syntax (Rust, Go, Python, JS/TS, C/C++) |
| `counter/comment` | Done | CommentDetector for single/multi-line comment detection |
| `counter/sloc` | Done | SlocCounter with LineStats (total, code, comment, blank) |
| `scanner/filter` | Done | GlobFilter for extension and exclude pattern filtering |
| `scanner/mod` | Done | DirectoryScanner with walkdir integration |
| `checker/threshold` | Partial | ThresholdChecker with override > path_rules > rule > default priority (pending: per-rule skip_comments/skip_blank/warn_threshold) |
| `output/text` | Done | TextFormatter with color support (ColorMode: Auto/Always/Never), status icons, summary |
| `output/json` | Done | JsonFormatter with structured output |
| `output/stats` | Done | StatsTextFormatter and StatsJsonFormatter for stats command |
| `output/sarif` | Pending | SARIF formatter for GitHub Code Scanning |
| `output/markdown` | Pending | Markdown formatter for PR comments |
| `output/html` | Pending | HTML report with charts and trends |
| `git/diff` | Done | GitDiff with gix for --diff mode (changed files since reference) |
| `error` | Done | SlocGuardError enum with thiserror |
| `main` | Done | Command dispatch, `run_check`, `run_stats`, `run_init`, `run_config` (validate/show) |

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

---

## Completed Phases (Compressed)

| Phase | Tasks | Status |
|-------|-------|--------|
| **Phase 1: Core MVP** | 1.1 FileConfigLoader, 1.2 run_check, 1.3 run_stats, 1.4 run_init, 1.5 run_config | ✅ All Done |
| **Phase 2.1** | Color Support (TextFormatter with Auto/Always/Never) | ✅ Done |
| **Phase 3.1** | Git Diff Mode (gix-based --diff) | ✅ Done |
| **Phase 4.3** | Path-Based Rules ([[path_rules]] with glob patterns) | ✅ Done |

---

## Phase 2: Output Enhancements (P1)

### Task 2.2: Implement SARIF Output

Location: `src/output/sarif.rs`

```
- Create SarifFormatter struct
- Follow SARIF 2.1.0 spec
- Map CheckResult to SARIF result object
- Include file location, message, level
- Useful for GitHub Code Scanning integration
```

### Task 2.3: Implement Markdown Output

Location: `src/output/markdown.rs`

```
- Create MarkdownFormatter struct
- Generate table-based output
- Include summary section
- Suitable for PR comments
```

### Task 2.4: Progress Bar for Large Scans

Location: `src/output/progress.rs` (new module)

```
- Use indicatif crate for progress bar
- Show: Scanning [████████░░░░] 62% (1,234/2,000 files)
- Enable with --progress flag or auto-detect large directories
- Disable in quiet mode or non-TTY output
- Update progress during parallel file processing
```

### Task 2.5: Top-N Files Report

Location: `src/output/text.rs`, `src/output/json.rs`

```
- Add --top N flag to check command
- Show only N files with highest line count (regardless of status)
- Useful for large projects to focus on worst offenders
- Include in JSON output as separate "top_files" array
```

---

## Phase 3: Git Integration (P1)

### Task 3.2: Add Git-Aware Exclude Patterns

Location: `src/scanner/filter.rs`

```
- Respect .gitignore patterns
- Use gix dirwalk feature
- Make git-aware scanning optional (flag or auto-detect)
```

---

## Phase 4: Advanced Features (P2)

### Task 4.1: Baseline Support

Location: `src/baseline/mod.rs` (new module)

```
- Allow existing violations to be "grandfathered"
- Store baseline in .sloc-guard-baseline.json
- Command: sloc-guard baseline update
- Only fail on NEW violations
- Track file hash to detect changes
```

### Task 4.2: Warning Threshold Configuration (Partial)

Location: `src/config/model.rs`, `src/checker/threshold.rs`

```
- [x] Add warn_threshold to DefaultConfig (default 0.9)
- [ ] Allow per-rule warning thresholds (e.g., [rules.rust] warn_threshold = 0.85)
- [ ] Update ThresholdChecker to read warn_threshold from config
- [x] Support --warn-threshold CLI override
```

### Task 4.4: Override with Reason

Location: `src/config/model.rs`, `src/checker/threshold.rs`

```
- Add reason field to [[override]] section
- Document purpose of exemption (e.g., "Legacy code, scheduled for Q2 refactor")
- Show reason in verbose output and reports
- Highest priority in rule matching
```

### Task 4.5: Custom Language Definition

Location: `src/config/model.rs`, `src/language/registry.rs`

```
- Add [languages.<name>] section in config
- Allow defining: extensions, single_line_comments, multi_line_comments
- Register custom languages at config load time
- Override built-in language definitions if same extension
```

### Task 4.6: Inline Ignore Comments

Location: `src/counter/sloc.rs`, `src/checker/threshold.rs`

```
- Support inline directives: // sloc-guard:ignore-file, ignore-next, ignore-start/end
- Parse directives during line counting phase
- More flexible than [[override]] - exemption lives with code
- Support language-specific comment prefixes (# for Python, etc.)
```

### Task 4.7: Caching Mechanism

Location: `src/cache/mod.rs` (new module)

```
- Cache line counts by file hash (content-based)
- Store in .sloc-guard-cache or configurable path
- Skip counting for unchanged files
- Invalidate on config change (hash config too)
- Config option: [cache] enabled = true, path = ".sloc-guard-cache"
- CLI flag: --no-cache to bypass
```

### Task 4.8: Configuration Inheritance (extends)

Location: `src/config/loader.rs`, `src/config/model.rs`

```
- Add "extends" field to config (local paths and URLs)
- Load base config first, then merge local overrides
- Recursive extends (with cycle detection)
- CLI: --no-extends to skip inheritance
```

### Task 4.9: Strict Mode

Location: `src/cli.rs`, `src/main.rs`

```
- Add --strict flag to check command
- In strict mode: warnings also cause exit code 1
- Opposite of --warn-only
- Config option: [default] strict = true
```

---

## Phase 5: Statistics Extension (P2)

### Task 5.1: Project-Wide Statistics

```
- Breakdown by language, Top N largest files, Average file size
- Distribution histogram, Per-directory breakdown
- Group output by language/directory (--group-by lang|dir)
```

### Task 5.2: Trend Tracking

```
- Store historical stats in .sloc-guard-history.json
- Show change from previous run
```

### Task 5.3: HTML Report Generation

```
- Create HtmlFormatter struct with --report flag
- Include: dashboard, charts, top files, per-directory stats, trends
- Use embedded CSS for standalone HTML file
```

---

## Phase 6: CI/CD Support (P2)

### Task 6.1: GitHub Action

```
- Create reusable GitHub Action
- Input: paths, config-path, fail-on-warning
- Output: total-files, passed, failed, warnings
- Annotate PR with results
```

### Task 6.2: Pre-commit Hook Integration

```
- Document .pre-commit-config.yaml setup
- Provide hook entry configuration
- Support staged files only mode
```

---

## Phase 7: Future Enhancements (P3)

### Task 7.1: Function-Level Analysis

```
- Parse function/method boundaries (language-specific)
- Count lines per function/method
- Add function_max_lines to config (optional)
- Note: Requires tree-sitter or similar
```

---

## Priority Order

1. **MVP**: ✅ Complete
2. **Quick Wins**: ✅ Color, verbose, override path fix, path-based rules
3. **Short-term (High Value)**:
   - 4.6 Inline Ignore Comments (better DX than [[override]])
   - 4.9 Strict Mode (simple, high CI value)
4. **Medium-term (High Value)**:
   - 4.7 Caching Mechanism (big perf win for large projects)
   - 2.2 SARIF Output (CI/CD integration)
   - 4.1 Baseline Support (essential for large projects)
   - 4.8 Configuration Inheritance (enterprise use case)
5. **Medium Priority**: 2.4 Progress Bar, 2.5 Top-N Files, 3.2 Git-Aware Exclude
6. **Lower Priority**: 2.3 Markdown, 4.2 Per-rule warn_threshold, 4.4 Override Reason, 4.5 Custom Languages
7. **Deferred**: Phase 5, 6, 7

---

## Architecture Notes

### Dependency Flow

```
main.rs
  -> cli (parse args)
  -> config/loader (load config)
  -> scanner (find files)
  -> language/registry (get comment syntax)
  -> counter/sloc (count lines)
  -> checker/threshold (check limits)
  -> output/* (format results)
```
