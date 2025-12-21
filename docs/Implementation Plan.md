# sloc-guard Implementation Plan

> **Doc Maintenance**: Keep concise, avoid redundancy, clean up outdated content promptly to reduce AI context usage.

## Lint

```
make ci
```

## Completed (Compressed)

All modules in PROJECT_OVERVIEW.md Module Map are implemented.

- **Phase 1-3**: Core MVP, Color Support, Git Diff Mode, Git-Aware Exclude
- **Phase 4**: Path-Based Rules, Inline Ignore, Strict Mode, Baseline, SARIF Output, Progress Bar, File Hash Cache, Per-rule warn_threshold, Override with Reason, Custom Language Definition, Config Inheritance, Split Suggestions, Remote Config.
- **Phase 5 (Partial)**: Language Breakdown, Top-N & Metrics, Markdown Output, Directory Statistics, Trend Tracking, HTML Report, Structure Guard.
- **Phase 5.5 (Refactoring & V2 Config)**: Scanner/Structure separation, `Config` V2 (auto-migration), `CheckResult` refactor, DI Context, Extension-based rule sugar, Explicit Rule Priority, Structure `warn_threshold`.
- **Phase 6 (Partial)**: CLI updates (`--max-files/dirs`, `--diff/--staged`, `--history-file`, `--update-baseline`, `--report-json`), parameter renames (`--suggest`, `--count-*`), documentation updates, 12.12 `--diff` Semantic Consistency.
- **Phase 8 (CI/CD)**: GitHub Action (cache, summary, matcher), Pre-commit Hook, Universal Docker Image, Binary Download Optimization, SARIF Guidance.
- **Phase 9**: `explain` command, `max_depth` limit, `init --detect`, Structure Allowlist Mode, Unified Directory Traversal.
- **Phase 10**: IO Abstraction, error handling cleanup.
- **Phase 11 (Partial)**: 11.6 Config Presets, 11.8 Terminology Modernization.
- **Phase 12 (Partial)**: 12.1 Structure Rule Priority, 12.2 Remove Deprecated Baseline Command, 12.3 Override Path Validation, 12.4 Consolidate State Files, 12.5 Git Scanner Fallback Warning, 12.6 max_depth Example, 12.7 Remove V1 path_rules, 12.8 FS .gitignore Support, 12.9.1 Remote Fetch Warning, 12.9.2 Offline Mode, 12.9.3 Hash Lock, 12.10 Rule Matching Overrides Extension Filter, 12.11 Relative max_depth, 12.13 --diff A..B Range Syntax.
- **Phase 13**: 13.1 Project Root Discovery, 13.2 Cache Hash Optimization, 13.3 File Locking, 13.4 Test Isolation.

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

## Phase 11: Advanced Governance (Pending)

### Task 11.1: Naming Convention Enforcement
Location: `src/config/structure.rs`, `src/checker/structure.rs`
```
- Add `file_naming_pattern` (regex) to [[structure.rules]]
- Validates filenames match pattern (e.g., PascalCase for components, `use*` for hooks)
- New violation type: NamingConvention { expected_pattern, actual_filename }
```

### Task 11.2: File Co-location Check
Location: `src/checker/structure.rs`
```
- Add `require_sibling` to [[structure.rules]]: { pattern: "*.ts", sibling: "*.spec.ts" }
- Validates paired files exist together (component + test, implementation + docs)
- New violation type: MissingSibling { file, expected_sibling_pattern }
```

### Task 11.3: Time-bound Overrides
Location: `src/config/*.rs`, `src/checker/*.rs`
```
- Add `expires = "YYYY-MM-DD"` to [[content.override]] and [[structure.override]]
- Expired overrides become violations (treat as if override doesn't exist)
- Warning mode: warn N days before expiration (configurable)
```

### Task 11.4: Baseline Ratchet
Location: `src/commands/check.rs`, `src/baseline/mod.rs`
```
- CI mode flag: --ratchet (or config: baseline.ratchet = true)
- Ratchet behavior when current violations < baseline count:
  - Default: emit warning "Baseline can be tightened: N violations removed"
  - With --ratchet=auto: auto-update baseline file silently
  - With --ratchet=strict: fail CI if baseline not updated (forces team to commit improvement)
- Prevents regression: error count can only decrease over time
- CI integration: GitHub Action output `baseline-outdated: true` for workflow conditionals
- Optional: suggest PR bot integration for automatic baseline update PRs
```

### Task 11.7: Deny Patterns
Location: `src/config/structure.rs`, `src/checker/structure.rs`
```
- Add `deny_extensions` and `deny_patterns` to [structure] and [[structure.rules]]
- Matches result in immediate violation regardless of other rules
- Use case: ban .exe/.dll, enforce migration (.js → .ts)
- New violation type: DeniedFile { pattern_or_extension }
```

---

## Priority Order

| Priority | Tasks |
|----------|-------|
| ~~**1. State File Cleanup**~~ | ~~12.7 Remove V1 path_rules~~ ✅ |
| ~~**2. Git Diff Enhancement**~~ | ~~12.13 --diff A..B Explicit Range Syntax~~ ✅ |
| **3. Governance Deep Dive** | 11.1 Naming Convention, 11.2 Co-location, 11.7 Deny Patterns |
| **4. Debt Lifecycle** | 11.3 Time-bound Overrides, 11.4 Baseline Ratchet |
| **5. Visualization** | 7.1-7.2 HTML Charts/Trends |

