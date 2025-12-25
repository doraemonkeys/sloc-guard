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
- **Phase 18**: 18.1 Unified Siblings Config (replaced `file_pattern`/`require_sibling` with `siblings` array, added `SiblingRule::Directed`/`Group`, `GroupIncomplete` violation).



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
| ~~**13. Sibling Rules Redesign**~~ | ~~18.1 Unified Siblings Config~~ ✅                          |
| **14. Path Output Fix**          | 19.1 Relative Path Output                                    |
| **15. HTML Report Enhancement**  | 20.1 HTML Summary Stats, 20.2 File Total Column, 20.3 Stats HTML Format |

---

## Phase 20: HTML Report Enhancement (Pending)

### Task 20.1: HTML Summary Stats

Add aggregate statistics to HTML report summary section:
- Total lines, total code, total comments, total blanks
- Currently only shows file counts (Passed/Warning/Failed/Grandfathered)

### Task 20.2: File Total Column

Add "Total" column to HTML file table showing actual file line count (`stats.total`).

### Task 20.3: Stats HTML Format

Implement HTML output format for `stats` command (`--format html`):
- Display project statistics in styled HTML table (total files, lines, code/comment/blank counts)
- Language breakdown table with percentages
- Directory breakdown if `--group-by dir`
- Top files list if `--top N`
- Embed SVG charts inline (language pie chart, optional trend line if `--trend`)
- Follow existing `HtmlFormatter` styling patterns

---

## Phase 19: Path Handling (Pending)

### Task 19.1: Relative Path Output

**Problem**: On Windows, `canonicalize()` returns extended-length paths (`\\?\C:\...`), and scanners produce absolute paths. Output formatters display these directly via `path.display()`.

**Requirements**:
1. All formatters output paths relative to project root (or current working directory)
2. Eliminate `\\?\` prefix by avoiding `canonicalize()` or normalizing paths
3. Path conversion happens at output stage (not in scanner/checker internals)

**Affected Modules**:
- `src/scanner/gitignore.rs`: Uses `canonicalize()` → use `dunce::canonicalize()` or normalize manually
- `src/output/text.rs`, `json.rs`, `sarif.rs`, `markdown.rs`, `html.rs`: `path.display()` → convert to relative path



---

## Phase 18: Sibling Rules Redesign ✅

### ~~Task 18.1: Unified Siblings Config~~ ✅

---

