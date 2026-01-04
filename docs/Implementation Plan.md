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
- **Phase 16-20**: Trend History Command, Relative Path Output, Unified Siblings Config, HTML Stats Format
- **Phase 21**: Stats Command Restructure (subcommands: summary, files, breakdown, trend, report, snapshot), Trend Config, Report Config, Output Refactoring
- **Phase 22**: SARIF & GitHub Action Fixes
- **Phase 23**: Error Propagation
- **Phase 24**: Array Merge (`$reset`), Offline Cache (FetchPolicy), Check Behavior (`warnings_as_errors`/`fail_fast`), Cache Flag Unification (`--extends-policy`)

---

## Phase 25: Config Loader Refactoring

Addresses line number precision loss during extends inheritance and improves config error context.

**Dependency Graph:**
```
25.3 (Depth Limit) ─────────────────────────────┐
                                                ↓
25.1a/b/d (ConfigSource + Errors) ←──依赖──── 25.2 (Dual-Path Loading)
                    ↓                              ↓
               25.1c (Syntax Error)          depends on 25.2
                    ↓
               25.4 (Explain Source Chain)
                    ↓
               25.5 (Loader Split) ← optional
```

**25.3 Extends Chain Depth Limit** ✅
- `MAX_EXTENDS_DEPTH` constant (10) in `loader.rs`
- Depth tracked in `load_with_extends()` / `load_remote_with_extends()` / `process_config_content()`
- Returns "Extends chain too deep" error when limit exceeded

**25.1 ConfigSource & Structured Config Errors** *(split into subtasks)*

| Subtask | Content | Dependencies |
|---------|---------|--------------|
| 25.1a ✅ | `ConfigSource` enum (File/Remote/Preset) for origin tracking | None |
| 25.1b ✅ | `CircularExtends { chain }`, `ExtendsTooDeep { depth, max, chain }`, `ExtendsResolution { path, base }` variants | 25.1a |
| 25.1c ✅ | `Syntax { origin, line, column, message }` variant - precise location for raw parse errors | 25.2 |
| 25.1d ✅ | `TypeMismatch { field, expected, actual, origin }`, `Semantic { field, message, origin, suggestion }` variants | 25.1a |

**25.2 Dual-Path Loading Strategy** ✅

- Single-file mode: when no `extends`, parse directly from raw content (preserves precise line numbers via `Syntax` error)
- Inheritance mode: when `extends` present, use source chain tracking instead of line numbers
- `span_to_line_col()` helper converts byte offset to 1-based line/column

**25.4 Explain Config Source Chain** ✅

- `explain --config` shows full configuration inheritance chain
- Display field values with their origin sources (preset → remote → local)
- Show which config contributed which settings

## Priority Order

| Priority              | Tasks                                                        |
| --------------------- | ------------------------------------------------------------ |
| **19. Config Design** | ~~24.1 Array Merge~~ ✅, ~~24.2 Offline Cache~~ ✅, ~~24.3 Check Behavior~~ ✅, ~~24.4 Cache Flag Unification~~ ✅ |
| **20. Config Loader** | ~~25.3 Depth Limit~~ ✅, ~~25.1a/b/c/d~~ ✅, ~~25.2 Dual-Path~~ ✅ → ~~25.4~~ ✅ |

