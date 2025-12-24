# sloc-guard Implementation Plan

> **Doc Maintenance**: Keep concise, avoid redundancy, clean up outdated content promptly to reduce AI context usage.

## Lint

```
make ci
```

## Completed (Compressed)

All modules in PROJECT_OVERVIEW.md Module Map are implemented.

- **Phase 1-3**: Core MVP, Color Support, Git Diff Mode, Git-Aware Exclude
- **Phase 4**: Path-Based Rules, Inline Ignore, Strict Mode, SARIF Output, Progress Bar, File Hash Cache, Per-rule warn_threshold, Custom Language Definition, Config Inheritance, Split Suggestions, Remote Config.
- **Phase 5 (Partial)**: Language Breakdown, Top-N & Metrics, Markdown Output, Directory Statistics, Trend Tracking, HTML Report, Structure Guard.
- **Phase 5.5 (Refactoring & V2 Config)**: Scanner/Structure separation, `Config` V2 (auto-migration), `CheckResult` refactor, DI Context, Extension-based rule sugar, Explicit Rule Priority, Structure `warn_threshold`.
- **Phase 6 (Partial)**: CLI updates (`--max-files/dirs`, `--diff/--staged`, `--history-file`, `--update-baseline`, `--report-json`), parameter renames (`--suggest`, `--count-*`), documentation updates.
- **Phase 8 (CI/CD)**: GitHub Action (cache, summary, matcher), Pre-commit Hook, Universal Docker Image, SARIF Guidance.
- **Phase 9**: `explain` command, `max_depth` limit, `init --detect`, Structure Allowlist Mode, Unified Directory Traversal.
- **Phase 10**: IO Abstraction, error handling cleanup.
- **Phase 11**: 11.1 Naming Convention Enforcement, 11.2 File Co-location Check, 11.4 Baseline Ratchet, 11.6 Config Presets, 11.7 Deny Patterns, 11.8 Terminology Modernization, 11.9 Rename pattern→scope, 11.10 Content Exclude Patterns, 11.11 Granular Warn Thresholds, 11.12 deny_files + deny_dirs, 11.13 Structure Allowlist Mode, 11.14 Unify Rule and Override, 11.15 Remove Language Shorthand.
- **Phase 12**: Structure Rule Priority, State File Consolidation, .gitignore Support, Remote Config (Fetch Warning, Offline Mode, Hash Lock), Rule Matching Override, Relative max_depth, --diff A..B Range.
- **Phase 13**: 13.1 Project Root Discovery, 13.2 Cache Hash Optimization, 13.3 File Locking, 13.4 Test Isolation.
- **Phase 14**: 14.1 Extract Path Matching Utility, 14.2 CheckOptions Struct, 14.3 Scanner Module Split.

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

## Phase 11: Advanced Governance (Completed)

> **Performance Note**: Introducing allowlists and complex scope matching may increase computational cost. Ensure **`globset` compilation reuse** during implementation to avoid performance regression.

---

## Priority Order

| Priority | Tasks |
|----------|-------|
| ~~**1. State File Cleanup**~~ | ~~12.7 Remove V1 path_rules~~ ✅ |
| ~~**2. Git Diff Enhancement**~~ | ~~12.13 --diff A..B Explicit Range Syntax~~ ✅ |
| ~~**3. Code Quality**~~ | ~~14.1 Extract Path Matching~~ ✅, ~~14.2 CheckOptions Struct~~ ✅, ~~14.3 Scanner Module Split~~ ✅ |
| ~~**4. Structure Naming**~~ | ~~11.9 pattern→scope~~ ✅, ~~11.12 deny_file_patterns→deny_files + deny_dirs~~ ✅ |
| ~~**5. Governance Refinement**~~ | ~~11.10 Content Exclude~~ ✅, ~~11.11 Granular Warn~~ ✅, ~~11.13 Allowlist Mode~~ ✅ |
| ~~**6. Config Simplification**~~ | ~~11.14 Unify Rule and Override~~ ✅, ~~11.15 Remove Language Shorthand~~ ✅ |
| ~~**7. Debt Lifecycle**~~ | ~~11.4 Baseline Ratchet~~ ✅ |
| **8. Visualization** | 7.1-7.2 HTML Charts/Trends |
| **9. Error UX** | 15.1-15.3 Error Message Enhancement |

---

## Phase 15: Error Message Enhancement (Pending)

### Task 15.1: Colored Error Output
Location: `src/error.rs`, `src/commands/check/runner.rs`
```
- Add colored crate for visual distinction (red=error, yellow=warning)
- Create unified print_error() helper with consistent formatting
- Format: ✖ Error Type / × Detail / help: Suggestion
```

### Task 15.2: Structured Error Suggestions
Location: `src/error.rs`
```
- Add suggestion() method to SlocGuardError
- Provide actionable hints per error variant:
  - GitRepoNotFound → "Run 'git init' or run inside a git repository"
  - Config → Include expected format/range
  - FileRead → Show underlying reason (not found / permission denied)
  - InvalidPattern → Show globset parse error details
```

### Task 15.3: Error Context Enrichment
Location: `src/error.rs`
```
- FileRead: Display source error kind
- InvalidPattern: Include globset error message
- IO: Add path/operation context where available
```
