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

---

## Phase 21: Stats Command Restructure (Pending)

Refactor `stats` into explicit subcommands following design principles:
- Subcommand = data scope (noun)
- Flag = modifier/filter (verb/adjective)
- Read/write separation
- Bare `stats` requires subcommand

### Task 21.1: CLI Subcommand Structure ✅

Restructure CLI to enforce subcommand requirement:
- `stats summary` - project-level totals only
- `stats files` - file list with `--top`, `--sort`, `--ext`
- `stats breakdown` - grouped stats with `--by lang|dir`, `--depth`
- `stats trend` - delta comparison with `--since`
- `stats history` - list entries with `--limit` (existing, verify compatibility)
- `stats report` - comprehensive output with `-o` for file output
- Bare `stats` → error with subcommand list

### Task 21.2: Stats Summary Subcommand ✅

Implement `stats summary`:
- Output: Files, Code, Comments, Blank, Average metrics
- Flags: `--format text|json|md`
- No file list, no breakdown—summary only

### Task 21.3: Stats Files Subcommand

Implement `stats files` (migrate `--top N` functionality):
- Default: all files sorted by code descending
- Flags: `--top N`, `--sort code|total|comment|blank|name`, `--ext rs,go`, `--format`
- No summary section appended

### Task 21.4: Stats Breakdown Subcommand

Implement `stats breakdown` (migrate `--group-by` functionality):
- Default: by language
- Flags: `--by lang|dir`, `--depth N` (for dir mode), `--format`
- Visual progress bars in text output

### Task 21.5: Stats Trend Subcommand

Convert `--trend` flag to `stats trend` subcommand:
- Read-only comparison with history
- Flags: `--since <duration>` (7d, 1w, 12h), `--format`
- Output: delta values with arrows, previous/current commit info

### Task 21.6: Stats Report Subcommand

Implement `stats report` for comprehensive output:
- Combines summary + files + breakdown + trend
- Flags: `--format text|json|md|html`, `-o <path>`
- Content controlled by `[stats.report]` config (exclude list)

### Task 21.7: Snapshot Command ✅

Create standalone `snapshot` command (read/write separation):
- Records current stats to trend history
- Respects `min_interval_secs` from config (--force to override)
- Flags: `--history-file`, `--force`, `--dry-run`
- Separate from stats viewing commands

### Task 21.8: Trend Config Enhancement

Extend `[trend]` config:
- `auto_snapshot_on_check`: auto-record after successful `check`
- Retention: `max_entries`, `max_age_days`, `min_interval_secs`
- Significance: `min_code_delta` threshold

### Task 21.9: Stats Report Config

Add `[stats.report]` config section:
- `exclude = []` - sections to omit (summary, files, breakdown, trend)
- `top_count` - files section count
- `breakdown_by` - default grouping
- `trend_since` - default comparison period

---

## Priority Order

| Priority               | Tasks                                                                                              |
| ---------------------- | -------------------------------------------------------------------------------------------------- |
| **16. Stats Restructure** | ~~21.1 CLI~~ ✅, ~~21.2 Summary~~ ✅, 21.3 Files, 21.4 Breakdown, 21.5 Trend, 21.6 Report, ~~21.7 Snapshot~~ ✅, 21.8-9 Config |

