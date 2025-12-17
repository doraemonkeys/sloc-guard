# sloc-guard Implementation Plan

> **Doc Maintenance**: Keep concise, avoid redundancy, clean up outdated content promptly to reduce AI context usage.

## Current Status

### Completed Components

| Module | Status | Description |
|--------|--------|-------------|
| `cli` | Done | CLI with check, stats, init, config commands + global options (verbose, quiet, color, no-config) |
| `config/model` | Partial | Config, DefaultConfig, RuleConfig, ExcludeConfig, FileOverride (pending: path_rules, per-rule warn_threshold) |
| `config/loader` | Done | FileConfigLoader with search order: CLI -> project .sloc-guard.toml -> $HOME/.config/sloc-guard/config.toml -> defaults |
| `language/registry` | Done | Language definitions with comment syntax (Rust, Go, Python, JS/TS, C/C++) |
| `counter/comment` | Done | CommentDetector for single/multi-line comment detection |
| `counter/sloc` | Done | SlocCounter with LineStats (total, code, comment, blank) |
| `scanner/filter` | Done | GlobFilter for extension and exclude pattern filtering |
| `scanner/mod` | Done | DirectoryScanner with walkdir integration |
| `checker/threshold` | Partial | ThresholdChecker with override > rule > default priority (pending: path_rules, per-rule skip_comments/skip_blank/warn_threshold) |
| `output/text` | Done | TextFormatter with color support (ColorMode: Auto/Always/Never), status icons, summary |
| `output/json` | Done | JsonFormatter with structured output |
| `output/stats` | Done | StatsTextFormatter and StatsJsonFormatter for stats command |
| `output/sarif` | Pending | SARIF formatter for GitHub Code Scanning |
| `output/markdown` | Pending | Markdown formatter for PR comments |
| `output/html` | Pending | HTML report with charts and trends |
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

## Phase 1: Core MVP (P0)

### Task 1.1: Implement FileConfigLoader ✅

Location: `src/config/loader.rs`

```
- [x] Implement concrete FileConfigLoader struct
- [x] Load from .sloc-guard.toml in current directory
- [x] Load from $HOME/.config/sloc-guard/config.toml as fallback
- [x] Return Config::default() if no config found
- [x] Add tests for each scenario
```

### Task 1.2: Implement run_check Command ✅

Location: `src/main.rs`

```
- [x] Load configuration (from file or defaults, respect --no-config)
- [x] Apply CLI argument overrides:
  - --max-lines, --ext, --exclude, --include
  - --no-skip-comments, --no-skip-blank
  - --warn-threshold
- [x] Create GlobFilter from config + CLI args
- [x] Scan directories with DirectoryScanner
- [x] For each file:
  - Detect language from extension
  - Count lines with SlocCounter
  - Check against threshold
- [x] Collect results
- [x] Format output (text/json based on --format; sarif/markdown return error as not implemented)
- [x] Write to --output file if specified
- [x] Return appropriate exit code (0/1/2, or 0 if --warn-only)
- [x] Add tests for all functions (20 tests, 82.38% coverage)
```

### Task 1.3: Implement run_stats Command ✅

Location: `src/main.rs`, `src/output/stats.rs`

```
- [x] Similar flow to check but without threshold checking
- [x] Load config for exclude patterns (respect --no-config)
- [x] Support --config, --ext, --exclude, --include options
- [x] Just count and display statistics
- [x] Support --format (text/json) and --output options
- [x] Add FileStatistics, ProjectStatistics types
- [x] Add StatsTextFormatter and StatsJsonFormatter
- [x] Add tests (12 tests for main, 9 tests for stats module)
```

### Task 1.4: Implement run_init Command ✅

Location: `src/main.rs`

```
- [x] Generate default .sloc-guard.toml
- [x] Check if file exists (error unless --force)
- [x] Write template config with comments
- [x] Add tests (8 tests for init command)
```

### Task 1.5: Implement run_config Commands ✅

Location: `src/main.rs`

```
- [x] config validate:
  - [x] Parse specified config file (or find default)
  - [x] Validate TOML syntax
  - [x] Validate semantic correctness (valid glob patterns, threshold in range, rule validity)
  - [x] Output validation errors with context
  - [x] Return EXIT_CONFIG_ERROR on failure
- [x] config show:
  - [x] Load and merge configuration (file + defaults)
  - [x] Support --format (text/json)
  - [x] Show effective configuration
- [x] Add tests (18 tests for config commands, 83.68% coverage)
```

---

### Known Gaps (CLI defined but not implemented)

| Feature | CLI Location | Status |
|---------|--------------|--------|
| `--diff` | `CheckArgs.diff` | gix imported, logic not wired |
| `path_rules` | Config template only | Model struct missing |

---

## Phase 2: Output Enhancements (P1)

### Task 2.1: Add Color Support to TextFormatter ✅

Location: `src/output/text.rs`

```
- [x] Add ColorMode enum (Auto/Always/Never)
- [x] Red for FAILED, yellow for WARNING, green for PASSED
- [x] Detect terminal capability (isatty) in Auto mode
- [x] Respect NO_COLOR environment variable
- [x] Wire --color CLI flag to TextFormatter
- [x] Add tests for colored/non-colored output
```

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

---

## Phase 3: Git Integration (P1)

### Task 3.1: Implement Diff Mode

Location: `src/git/diff.rs` (new module)

```
- Use gix crate (already in dependencies)
- Parse --diff argument (branch name or commit hash)
- Get list of changed files from diff
- Filter scanner results to only changed files
- Handle case where git repo not found
```

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

### Task 4.2: Warning Threshold Configuration

Location: `src/config/model.rs`, `src/checker/threshold.rs`

```
- [x] Add warn_threshold to DefaultConfig (default 0.9)
- [ ] Allow per-rule warning thresholds (e.g., [rules.rust] warn_threshold = 0.85)
- [ ] Update ThresholdChecker to read warn_threshold from config
- [x] Support --warn-threshold CLI override (implemented in Task 1.2)
```

### Task 4.3: Path-Based Rules

Location: `src/config/model.rs`, `src/checker/threshold.rs`

```
- Add [[path_rules]] section support in config
- Support path patterns (e.g., "src/generated/**")
- Higher priority than extension-based rules, lower than override
- Use glob matching for path patterns
- Support warn_threshold per path rule
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
- Allow defining:
  - extensions: ["ext1", "ext2"]
  - single_line_comments: ["//", "#"]
  - multi_line_comments: [["/*", "*/"], ["<!--", "-->"]]
- Register custom languages at config load time
- Override built-in language definitions if same extension
```

---

## Phase 5: Statistics Extension (P2)

### Task 5.1: Project-Wide Statistics

Location: `src/stats/mod.rs` (new module)

```
- Total SLOC across project
- Breakdown by language
- Top N largest files
- Average file size
- Distribution histogram
- Per-directory breakdown
```

### Task 5.2: Trend Tracking

Location: `src/stats/trend.rs`

```
- Store historical stats in .sloc-guard-history.json
- Show change from previous run
- Useful for monitoring code growth
```

### Task 5.3: HTML Report Generation

Location: `src/output/html.rs` (new module)

```
- Create HtmlFormatter struct
- Support --report flag to generate HTML file
- Include:
  - Summary dashboard with key metrics
  - Interactive charts (file size distribution, language breakdown)
  - Top N largest files table
  - Per-directory statistics
  - Trend visualization (if history available)
- Use embedded CSS for standalone HTML file
```

---

## Phase 6: CI/CD Support (P2)

### Task 6.1: GitHub Action

Location: `.github/action.yml`

```
- Create reusable GitHub Action
- Input: paths, config-path, fail-on-warning
- Output: total-files, passed, failed, warnings
- Annotate PR with results
```

### Task 6.2: Pre-commit Hook Integration

Location: `docs/pre-commit.md`, config examples

```
- Document .pre-commit-config.yaml setup
- Provide hook entry configuration
- Support staged files only mode
```

---

## Phase 7: Future Enhancements (P3)

### Task 7.1: Function-Level Analysis

Location: `src/counter/function.rs` (new module)

```
- Parse function/method boundaries (language-specific)
- Count lines per function/method
- Add function_max_lines to config (optional)
- Report functions exceeding limit
- Support languages: Rust, Go, Python, JavaScript/TypeScript
- Note: Requires language-specific parsing, consider tree-sitter
```

---

## Priority Order

1. **Immediate (MVP)**: ~~1.1 -> 1.2 -> 1.3 -> 1.4 -> 1.5~~ ✅
2. **Quick Wins**:
   - ~~2.1 Color Support~~ ✅
   - ~~Implement `--verbose` real output~~ ✅
   - ~~Fix override path matching logic (too loose with `contains`)~~ ✅
3. **Short-term**:
   - 3.1 Git Diff Mode (`--diff`, gix already imported)
   - 4.3 Path-Based Rules (config template mentions it, model missing)
4. **Medium-term (2-3 weeks)**:
   - 2.2 SARIF Output (high CI/CD integration value)
   - 4.1 Baseline Support (essential for large projects)
   - 3.2 Git-Aware Exclude (.gitignore respect)
5. **Lower Priority**:
   - 2.3 Markdown Output
   - 4.2 Per-rule warn_threshold
   - 4.4 Override with Reason display
   - 4.5 Custom Language Definition
6. **Deferred**:
   - 5.1 -> 5.2 -> 5.3 (Statistics Extension)
   - 6.1 -> 6.2 (CI/CD Support)
   - 7.1 (Function-Level Analysis)

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
