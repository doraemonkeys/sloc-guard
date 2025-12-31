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

### Task 21.3: Stats Files Subcommand ✅

Implement `stats files` (migrate `--top N` functionality):
- Default: all files sorted by code descending
- Flags: `--top N`, `--sort code|total|comment|blank|name`, `--ext rs,go`, `--format`
- No summary section appended
- Fix: `JsonStatsOutput.files` should use `Option<Vec>` + `skip_serializing_if` (avoid empty `"files": []` in summary-only JSON)

### Task 21.4: Stats Breakdown Subcommand ✅

Implement `stats breakdown` (migrate `--group-by` functionality):
- Default: by language
- Flags: `--by lang|dir`, `--depth N` (for dir mode), `--format`
- Visual progress bars in text output

### Task 21.5: Stats Trend Subcommand ✅

Convert `--trend` flag to `stats trend` subcommand:
- Read-only comparison with history
- Flags: `--since <duration>` (7d, 1w, 12h), `--format`
- Output: delta values with arrows, previous/current commit info

### Task 21.6: Stats Report Subcommand ✅

Implement `stats report` for comprehensive output:
- Combines summary + files + breakdown + trend
- Flags: `--format text|json|md|html`, `-o <path>`, `--exclude-section`, `--top`, `--breakdown-by`, `--since`
- Content controlled by `[stats.report]` config (exclude, top_count, breakdown_by, trend_since)

### Task 21.7: Snapshot Command ✅

Create standalone `snapshot` command (read/write separation):
- Records current stats to trend history
- Respects `min_interval_secs` from config (--force to override)
- Flags: `--history-file`, `--force`, `--dry-run`
- Separate from stats viewing commands

### Task 21.8: Trend Config Enhancement ✅

Extend `[trend]` config:
- `auto_snapshot_on_check`: auto-record after successful `check`
- Retention: `max_entries`, `max_age_days`, `min_interval_secs`
- Significance: `min_code_delta` threshold

### Task 21.9: Stats Report Config ✅

Add `[stats.report]` config section:
- `exclude = []` - sections to omit (summary, files, breakdown, trend)
- `top_count` - files section count
- `breakdown_by` - default grouping
- `trend_since` - default comparison period
- Validation for exclude values, breakdown_by values, and trend_since format

### Task 21.10: Stats Output Refactoring ✅

Clean up DRY violations and implicit state detection:
- Unify `FileSortOrder` enum: re-export `output::stats::FileSortOrder` from CLI, remove duplicate
- Add `StatsOutputMode` enum (`Full`, `SummaryOnly`, `FilesOnly`) to `ProjectStatistics`
- Replace fragile `files.is_empty() && top_files.is_some()` detection with explicit `output_mode` field
- Use `std::mem::take` in `with_sorted_files` instead of `.clone()` + clear

---

## Phase 22: SARIF & GitHub Action Fixes (Pending)

Fix semantic issues in SARIF output and improve GitHub Action reliability.

### Task 22.1: SARIF Structure Violation Rule IDs ✅

Add proper SARIF rules for Structure violations. Currently only 2 rules defined (`sloc-guard/line-limit-exceeded`, `sloc-guard/line-limit-warning`) but Structure violations (file count, dir count, max depth, naming, etc.) incorrectly use these SLOC rules.

- Add rule IDs per `ViolationType`: `sloc-guard/structure-file-count`, `sloc-guard/structure-dir-count`, `sloc-guard/structure-max-depth`, `sloc-guard/structure-disallowed-file`, `sloc-guard/structure-disallowed-dir`, `sloc-guard/structure-denied`, `sloc-guard/structure-naming`, `sloc-guard/structure-sibling`
- Use `violation_category()` to branch Content vs Structure in `convert_result()`
- Update `rules` array in `build_rules()` (10 rules total: 2 content + 8 structure)

### Task 22.2: SARIF Structure Violation Messages ✅

Fix message text for Structure violations. Currently shows "File has N SLOC" for all violations including structure (e.g., "Directory has 15 files" should not show as "File has 15 SLOC").

- Match on `ViolationType` to generate contextually correct messages via `format_structure_message()`
- Content messages via `format_content_message()` for SLOC violations

### Task 22.3: GitHub Action Multi-Format Efficiency ✅

Add `--write-sarif` and `--write-json` flags for single-run multi-format output:

- `--write-sarif <path>`: Write SARIF output to file in addition to primary format
- `--write-json <path>`: Write JSON output to file in addition to primary format
- Action updated to use single run: `sloc-guard check --format text --write-sarif <path> --write-json <path>`
- EXIT_CODE captured from single run

### Task 22.4: GitHub Action Reliability Fixes

Address shell and caching issues in `.github/action/action.yml`:

- Fix `latest` version cache: include resolved actual version in cache key
- Fix shell quoting: wrap `${{ inputs.paths }}` in quotes (lines 281, 298, 303, 310, 311, 317, 318)
- Fix SARIF `--format` argument construction (may conflict when JSON/Text added)
- Verify Problem Matcher regex matches actual `--format text --color never` output

### Task 22.5: Action Binary Download Format Alignment ✅

Align `action.yml` binary download with `release.yml` naming convention:

- Map runner OS/arch to platform (linux/macos/windows) and arch (x64/arm64) instead of target triples
- Archive naming: `sloc-guard-${VERSION}-${PLATFORM}-${ARCH}.${EXT}` (e.g., `sloc-guard-v0.2.1-linux-x64.tar.gz`)
- Checksum file: `checksums-sha256.txt` (was `SHA256SUMS`)
- Version handling: preserve `v` prefix consistently (e.g., `v0.2.1` not `0.2.1`)
- Update cache key to use platform/arch format

---

## Priority Order

| Priority               | Tasks                                                                                              |
| ---------------------- | -------------------------------------------------------------------------------------------------- |
| **17. SARIF & Action** | ~~22.1 SARIF Rules~~ ✅, ~~22.2 SARIF Messages~~ ✅, ~~22.3 Multi-Format~~ ✅, 22.4 Action Fixes, ~~22.5 Binary Format~~ ✅ |
| **16. Stats Restructure** | ~~21.1 CLI~~ ✅, ~~21.2 Summary~~ ✅, ~~21.3 Files~~ ✅, ~~21.4 Breakdown~~ ✅, ~~21.5 Trend~~ ✅, ~~21.6 Report~~ ✅, ~~21.7 Snapshot~~ ✅, ~~21.8~~ ✅, ~~21.9~~ ✅, ~~21.10~~ ✅ |

