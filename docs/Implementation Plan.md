# sloc-guard Implementation Plan


## Current Status

### Completed Components

| Module | Status | Description |
|--------|--------|-------------|
| `cli` | Done | CLI with check, stats, init, config commands + global options (verbose, quiet, color, no-config) |
| `config/model` | Done | Configuration data structures with include_paths, warn_threshold |
| `config/loader` | Stub | Only ConfigLoader trait defined, FileConfigLoader not implemented |
| `language/registry` | Done | Language definitions with comment syntax (Rust, Go, Python, JS/TS, C/C++) |
| `counter/comment` | Done | CommentDetector for single/multi-line comment detection |
| `counter/sloc` | Done | SlocCounter with LineStats (total, code, comment, blank) |
| `scanner/filter` | Done | GlobFilter for extension and exclude pattern filtering |
| `scanner/mod` | Done | DirectoryScanner with walkdir integration |
| `checker/threshold` | Done | ThresholdChecker with rule priority (override > rule > default) |
| `output/text` | Done | TextFormatter with status icons and summary |
| `output/json` | Done | JsonFormatter with structured output |
| `output/sarif` | Pending | SARIF formatter for GitHub Code Scanning |
| `output/markdown` | Pending | Markdown formatter for PR comments |
| `error` | Done | SlocGuardError enum with thiserror |
| `main` | Stub | Command dispatch done, all handlers are TODO stubs |

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

## Phase 1: Core MVP (P0)

### Task 1.1: Implement FileConfigLoader

Location: `src/config/loader.rs`

```
- Implement concrete FileConfigLoader struct
- Load from .sloc-guard.toml in current directory
- Load from $HOME/.config/sloc-guard/config.toml as fallback
- Return Config::default() if no config found
- Add tests for each scenario
```

### Task 1.2: Implement run_check Command

Location: `src/main.rs`

```
- Load configuration (from file or defaults, respect --no-config)
- Apply CLI argument overrides:
  - --max-lines, --ext, --exclude, --include
  - --no-skip-comments, --no-skip-blank
  - --warn-threshold
- Create GlobFilter from config + CLI args
- Scan directories with DirectoryScanner
- For each file:
  - Detect language from extension
  - Count lines with SlocCounter
  - Check against threshold
- Collect results
- Format output (text/json/sarif/markdown based on --format)
- Write to --output file if specified
- Return appropriate exit code (0/1/2, or 0 if --warn-only)
```

### Task 1.3: Implement run_stats Command

Location: `src/main.rs`

```
- Similar flow to check but without threshold checking
- Load config for exclude patterns (respect --no-config)
- Support --config, --ext, --exclude, --include options
- Just count and display statistics
- Support --format (text/json) and --output options
```

### Task 1.4: Implement run_init Command

Location: `src/main.rs`

```
- Generate default .sloc-guard.toml
- Check if file exists (error unless --force)
- Write template config with comments
```

### Task 1.5: Implement run_config Commands

Location: `src/main.rs`

```
- config validate:
  - Parse specified config file (or find default)
  - Validate TOML syntax
  - Validate semantic correctness (valid glob patterns, threshold in range)
  - Output validation errors with context
  - Return EXIT_CONFIG_ERROR on failure

- config show:
  - Load and merge configuration (file + defaults)
  - Support --format (text/json)
  - Show effective configuration
  - Indicate source of each value (file/default) in verbose mode
```

---

## Phase 2: Output Enhancements (P1)

### Task 2.1: Add Color Support to TextFormatter

Location: `src/output/text.rs`

```
- Use colored crate (already in dependencies)
- Red for FAILED, yellow for WARNING, green for PASSED
- Detect terminal capability (isatty)
- Respect NO_COLOR environment variable
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

### Task 4.2: Warning Threshold Configuration (Partial)

Location: `src/config/model.rs`, `src/checker/threshold.rs`

```
- [x] Add warn_threshold to DefaultConfig (default 0.9)
- [ ] Allow per-rule warning thresholds
- [ ] Update ThresholdChecker to read warn_threshold from config
- [ ] Support --warn-threshold CLI override
```

### Task 4.3: Path-Based Rules

Location: `src/config/model.rs`, `src/checker/threshold.rs`

```
- Support path patterns in rules (e.g., "src/generated/**")
- Higher priority than extension-based rules
- Use glob matching
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
```

### Task 5.2: Trend Tracking

Location: `src/stats/trend.rs`

```
- Store historical stats in .sloc-guard-history.json
- Show change from previous run
- Useful for monitoring code growth
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

## Priority Order

1. **Immediate (MVP)**: 1.1 -> 1.2 -> 1.3 -> 1.4 -> 1.5
2. **Short-term**: 2.1 -> 3.1 -> 3.2
3. **Medium-term**: 2.2 -> 2.3 -> 4.1 -> 4.2 -> 4.3
4. **Long-term**: 5.1 -> 5.2 -> 6.1 -> 6.2

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

### Design Principles

- All components use traits for testability
- Dependency injection via constructor parameters
- No global state
- Error handling via Result<T, SlocGuardError>

### Testing Strategy

- Unit tests in each module
- Integration tests in `tests/` directory
- Use tempfile for filesystem tests
- Use assert_cmd for CLI tests
