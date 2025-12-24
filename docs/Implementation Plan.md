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

### ~~Task 7.2: File Size Distribution Histogram~~ ✅
Location: `src/output/svg/histogram.rs`, `src/output/html.rs`
```
- FileSizeHistogram: vertical bar chart by SLOC ranges (0-50, 51-100, 101-200, 201-500, 500+)
- Data source: ProjectStatistics via HtmlFormatter.with_stats()
- Hover: <title> tooltips with file count per bucket
- Empty state: "Not enough files for histogram" (<3 files)
- CSS variable --color-chart-primary for theming
```

### ~~Task 7.3: Language Breakdown Chart~~ ✅
Location: `src/output/svg/language_chart.rs`, `src/output/html.rs`
```
- LanguageBreakdownChart: horizontal bar chart sorted by SLOC (descending)
- Data source: ProjectStatistics.by_language (pre-sorted)
- Hover: <title> tooltips with language name + exact line count
- Empty state: "No language data" message (when by_language is None or empty)
- CSS variable --color-chart-primary for theming
```

### ~~Task 7.4: Trend Line Chart~~ ✅
Location: `src/output/svg/trend_chart.rs`, `src/output/html.rs`
```
- TrendLineChart: line chart X=timestamp (MM/DD format), Y=code lines (auto-scaled)
- Data source: TrendHistory via HtmlFormatter.with_trend_history()
- Downsample to max 30 points (evenly sampled, preserves first/last)
- Git context: git_ref/branch in X-axis labels and tooltips
- Empty state: "No trend data" message when history empty
- CSS variable --color-chart-primary for theming
```

### ~~Task 7.5: Chart Interactivity & Polish~~ ✅
Location: `src/output/svg/trend_chart.rs`, `src/output/html_template.rs`
```
- Delta indicators: ↓green (decrease=good), ↑red (increase) with significance threshold
- Hover tooltips via CSS :hover + <title> fallback with delta info
- @media print styles: status prefixes, border fallbacks for color-only encoding
- Smart X-axis labels: MM/DD for short range, W## (week) for >1 month range
- CSS variables: --color-delta-good, --color-delta-bad, --color-delta-neutral
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
| ~~**12. Visualization**~~        | ~~7.1 SVG Core~~ ✅ → ~~7.2 Histogram~~ ✅ → ~~7.3 Language Chart~~ ✅ → ~~7.4 Trend Line~~ ✅ → ~~7.5 Polish~~ ✅ |

---

