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

## Phase 7: HTML Report Visualization (Pending)

### ~~Task 7.1: SVG Chart Generation Core~~ ✅
Location: `src/output/svg/`
```
- svg module with chart primitives (Axis, Bar, Line, BarChart, HorizontalBarChart, LineChart)
- viewBox-based responsive scaling via SvgBuilder
- CSS variable integration (ChartColor::CssVar) for dark mode support
- Accessibility: <title> elements, role="img", aria-labelledby
```

### Task 7.2: File Size Distribution Histogram
Location: `src/output/html.rs`
```
- Histogram by line count ranges (0-50, 51-100, 101-200, 201-500, 500+)
- Data source: ProjectStatistics (reuse --report-json flow)
- Hover: show exact file count per range
- Empty state: graceful handling (<3 files)
```

### Task 7.3: Language Breakdown Chart
Location: `src/output/html.rs`
```
- Horizontal bar chart sorted by SLOC
- Data source: ProjectStatistics.by_language
- Hover: show language name + exact line count
- Empty state: show "No language data" message
```

### Task 7.4: Trend Line Chart
Location: `src/output/html.rs`
```
- Line chart: X=timestamp, Y=code lines (auto-scaled)
- Data source: TrendHistory passed to HtmlFormatter
- Downsample to max 30 points if history longer
- Git context: show git_ref/git_branch as data point labels
- Fallback: show "No trend data" if history unavailable
```

### Task 7.5: Chart Interactivity & Polish
Location: `src/output/html.rs`
```
- Delta indicators: ↓green (decrease=good), ↑red (increase)
- Hover tooltips via CSS :hover + <title> fallback
- @media print styles (no color-only encoding)
- Smart X-axis labels (days/weeks based on range)
```

---

## Phase 16: Trend Enhancement (Pending)

### ~~Task 16.6: History Command~~ ✅

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
| ~~**9. Trend Core**~~            | ~~16.1 Retention Policy~~ ✅, ~~16.2 Output Semantics~~ ✅, ~~16.3 Significance Threshold~~ ✅ |
| ~~**10. Content Warn Granularity**~~ | ~~17.1 Content warn_at Field~~ ✅                           |
| ~~**11. Trend Extended**~~       | ~~16.4 Flexible Comparison~~ ✅, ~~16.5 Git Context~~ ✅, ~~16.6 History Command~~ ✅ |
| **12. Visualization**            | 7.1 SVG Core ✅ → 7.2 Histogram → 7.3 Language Chart → 7.4 Trend Line → 7.5 Polish |

---

