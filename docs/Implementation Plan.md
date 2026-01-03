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

---

## Phase 24: Config Design Improvements

Addresses counter-intuitive behaviors in config inheritance and ambiguous semantics.

**24.1 Array Merge with `$reset` Marker** ✅
- Arrays default to append (parent + child) during `merge_toml_values()`
- `$reset` as first element clears parent array, uses remaining child elements
- `validate_reset_positions()` errors if `$reset` is not first
- `strip_reset_markers()` removes markers from final config (handles no-extends case)

**24.2 Offline Cache Strategy Separation** ✅
- `FetchPolicy` enum: `Normal` (TTL-controlled), `Offline` (ignore TTL), `ForceRefresh` (skip cache)
- `--offline` mode ignores TTL, uses any existing cache
- Remote cache moved to state directory (`.git/sloc-guard/remote-configs/` or `.sloc-guard/remote-configs/`)
- SHA256 hash lock validates content regardless of TTL

**24.3 Check Behavior Configuration** ✅
- Removed ambiguous `content.strict` field (deprecated)
- Add `[check]` section with `warnings_as_errors` (treat warnings as failures) and `fail_fast` (stop on first failure)
- Add CLI flags `--warnings-as-errors` and `--fail-fast` (`--strict` kept as hidden deprecated alias)
- `fail_fast` implements short-circuit processing with `AtomicBool` for parallel file processing

**24.4 Cache Flag Unification**
- Problem: Two separate cache mechanisms use confusingly similar flags (`--no-cache` for SLOC, `--offline` for remote config)
- Remove: `--offline` (global), `--no-cache` (check/stats)
- Add: `--no-sloc-cache` - disable SLOC counting cache (replaces `--no-cache`)
- Add: `--extends-policy=<mode>` (global) - remote config fetch strategy
  - `normal` (default): 1h TTL, fetch on miss/expire
  - `offline`: use cached only, ignore TTL, error on miss
  - `refresh`: skip cache, always fetch fresh
- Maps to existing `FetchPolicy` enum: `Normal`, `Offline`, `ForceRefresh`

---

## Priority Order

| Priority               | Tasks                                                         |
| ---------------------- | ------------------------------------------------------------- |
| **19. Config Design**  | ~~24.1 Array Merge~~ ✅, ~~24.2 Offline Cache~~ ✅, ~~24.3 Check Behavior~~ ✅, 24.4 Cache Flag Unification |

