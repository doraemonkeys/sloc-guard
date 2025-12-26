# sloc-guard Implementation Plan

> **Doc Maintenance**: Keep concise, avoid redundancy, clean up outdated content promptly to reduce AI context usage.

## Lint

```
make ci
```

## Completed (Compressed)

All modules in PROJECT_OVERVIEW.md Module Map are implemented.

- **Phase 1-3**: Core MVP, Color Support, Git Diff Mode, Git-Aware Exclude
- **Phase 4**: Path-Based Rules, Inline Ignore, Strict Mode, SARIF Output, Progress Bar, Cache, Custom Language, Config Inheritance, Split Suggestions, Remote Config
- **Phase 5**: Language Breakdown, Top-N & Metrics, Markdown Output, Directory Statistics, Trend Tracking, HTML Report, Structure Guard
- **Phase 5.5**: Config V2, CheckResult refactor, DI Context, Scanner/Structure separation
- **Phase 6**: CLI updates (diff/staged/ratchet/baseline flags), parameter renames
- **Phase 8**: GitHub Action, Pre-commit Hook, Docker Image, SARIF Guidance
- **Phase 9**: `explain` command, `max_depth`, `init --detect`, Allowlist Mode, Unified Traversal
- **Phase 10-11**: IO Abstraction, Naming Convention, Co-location Check, Baseline Ratchet, Config Presets, Deny Patterns, Content Exclude, Granular Warn Thresholds
- **Phase 12**: Rule Priority, State Consolidation, .gitignore, Remote Config (Offline/Hash Lock), --diff A..B Range
- **Phase 13-15**: Project Root Discovery, Cache Optimization, File Locking, Path Matching, CheckOptions, Scanner Split, Colored Error Output
- **Phase 18**: Unified Siblings Config (`siblings` array with Directed/Group rules)



## Phase 16: Trend Enhancement (Pending)

### ~~Task 16.6: History Command~~ ✅

---

## Phase 19: Path Handling ✅

### ~~Task 19.1: Relative Path Output~~ ✅



---

## Phase 18: Sibling Rules Redesign ✅

### ~~Task 18.1: Unified Siblings Config~~ ✅

---



## Phase 20: HTML Report Enhancement (Pending)

### ~~Task 20.1: HTML Summary Stats~~ ✅

### ~~Task 20.2: File Total Column~~ ✅

### ~~Task 20.3: Stats HTML Format~~ ✅

Implemented HTML output format for `stats` command (`--format html`):

- Summary cards showing total files, lines, code, comments, blanks, and avg code/file
- Trend delta section with colored cards when `--trend` is used
- Language breakdown table when `--group-by lang`
- Directory breakdown table when `--group-by dir`
- Top files table when `--top N`
- Inline SVG charts: language breakdown chart and trend line chart
- Follows existing `HtmlFormatter` CSS styling and HTML template

## Priority Order

| Priority                         | Tasks                                                        |
| -------------------------------- | ------------------------------------------------------------ |
| ~~**13. Sibling Rules Redesign**~~ | ~~18.1 Unified Siblings Config~~ ✅                          |
| ~~**14. Path Output Fix**~~      | ~~19.1 Relative Path Output~~ ✅                              |
| **15. HTML Report Enhancement**  | ~~20.1 HTML Summary Stats~~ ✅, ~~20.2 File Total Column~~ ✅, ~~20.3 Stats HTML Format~~ ✅ |

---



