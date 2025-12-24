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
- **Phase 15**: 15.1 Colored Error Output, 15.2 Structured Error Suggestions, 15.3 Error Context Enrichment.

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

## Phase 16: Trend Enhancement (Pending)

### Task 16.1: Retention Policy
Location: `src/stats/trend.rs`, `src/config/model.rs`
```
- Add TrendConfig struct: max_entries, max_age_days, min_interval_secs
- Auto-cleanup old entries on save (prevent infinite history growth)
- Configurable via [trend] section (top-level, like [baseline])
```

### Task 16.2: Output Time Semantics
Location: `src/output/stats_text.rs`
```
- Display relative time ("2 hours ago") instead of raw timestamp
- Show percentage change for each metric
- Add trend arrows (↑↓~) with color coding
```

### Task 16.3: Significance Threshold
Location: `src/stats/trend.rs`
```
- Add is_significant() method with configurable threshold
- Skip trend output when delta is trivial (reduce noise)
- Default: code_delta > 10 || files_delta > 0
```

### Task 16.4: Flexible Time Comparison
Location: `src/stats/trend.rs`, `src/cli.rs`
```
- Add --since <duration> flag (e.g., 7d, 30d)
- compute_delta_since(Duration) method
- Find nearest entry before specified time point
```

### Task 16.5: Git Context
Location: `src/stats/trend.rs`
```
- Add optional git_ref (commit hash) and git_branch to TrendEntry
- Populate via gix when in git repo (optional dependency)
- Output: "Changes since commit a1b2c3d (2 hours ago)"
```

### Task 16.6: History Command
Location: `src/cli.rs`, `src/commands/stats.rs`
```
- Add `stats history` subcommand
- List recent entries (--limit N, default 10)
- Support --format json for machine parsing
```

---

## Priority Order

| Priority                         | Tasks                                                        |
| -------------------------------- | ------------------------------------------------------------ |
| ~~**1. State File Cleanup**~~    | ~~12.7 Remove V1 path_rules~~ ✅                              |
| ~~**2. Git Diff Enhancement**~~  | ~~12.13 --diff A..B Explicit Range Syntax~~ ✅                |
| ~~**3. Code Quality**~~          | ~~14.1 Extract Path Matching~~ ✅, ~~14.2 CheckOptions Struct~~ ✅, ~~14.3 Scanner Module Split~~ ✅ |
| ~~**4. Structure Naming**~~      | ~~11.9 pattern→scope~~ ✅, ~~11.12 deny_file_patterns→deny_files + deny_dirs~~ ✅ |
| ~~**5. Governance Refinement**~~ | ~~11.10 Content Exclude~~ ✅, ~~11.11 Granular Warn~~ ✅, ~~11.13 Allowlist Mode~~ ✅ |
| ~~**6. Config Simplification**~~ | ~~11.14 Unify Rule and Override~~ ✅, ~~11.15 Remove Language Shorthand~~ ✅ |
| ~~**7. Debt Lifecycle**~~        | ~~11.4 Baseline Ratchet~~ ✅                                  |
| ~~**8. Error UX**~~              | ~~15.1 Colored Error Output~~ ✅, ~~15.2 Structured Error Suggestions~~ ✅, ~~15.3 Error Context Enrichment~~ ✅ |
| **9. Trend Core**                | 16.1 Retention Policy, 16.2 Output Semantics, 16.3 Significance Threshold |
| **10. Trend Extended**           | 16.4 Flexible Comparison, 16.5 Git Context, 16.6 History Command |
| **11. Visualization**            | 7.1-7.2 HTML Charts/Trends                                   |

---

