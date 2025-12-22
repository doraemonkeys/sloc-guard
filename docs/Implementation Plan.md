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
- **Phase 11 (Partial)**: 11.1 Naming Convention Enforcement, 11.2 File Co-location Check, 11.6 Config Presets, 11.7 Deny Patterns, 11.8 Deny File Patterns, 11.8 Terminology Modernization, 11.9 Rename pattern→scope.
- **Phase 12**: Structure Rule Priority, State File Consolidation, .gitignore Support, Remote Config (Fetch Warning, Offline Mode, Hash Lock), Rule Matching Override, Relative max_depth, --diff A..B Range.
- **Phase 13**: 13.1 Project Root Discovery, 13.2 Cache Hash Optimization, 13.3 File Locking, 13.4 Test Isolation.
- **Phase 14**: 14.1 Extract Path Matching Utility, 14.2 CheckOptions Struct, 14.3 Scanner Module Split.

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

> **Performance Note**: Introducing allowlists and complex scope matching may increase computational cost. Ensure **`globset` compilation reuse** during implementation to avoid performance regression.

---

### Task 11.10: Content Exclude Patterns

Add logical exclusion for content checking that doesn't affect structure visibility.

**Behavior**:
- Files matching `content.exclude` patterns skip SLOC counting but remain visible for structure checks
- Different from `scanner.exclude` (physical exclusion) - this is content-only exclusion

**Config Example**:
```toml
[content]
exclude = ["**/*.generated.ts", "**/*.pb.go"]  # Skip SLOC, still count in structure
```

**Use Case**: Scan all `.ts` files but skip SLOC for generated files while still counting them in `max_files`.

---

### Task 11.11: Granular Warn Thresholds

Add per-metric warning thresholds for structure limits.

**Behavior**:
- Absolute thresholds (`warn_files_at`, `warn_dirs_at`) take precedence over percentages
- Percentage thresholds (`warn_files_threshold`, `warn_dirs_threshold`) override global `warn_threshold`
- Fallback chain: per-metric absolute → per-metric percentage → global `warn_threshold` → default 0.8
- Apply to both `[structure]` and `[[structure.rules]]`

**Config Example**:
```toml
[structure]
max_files = 50
max_dirs = 10
warn_files_at = 45           # Warn at 45 files (absolute)
warn_dirs_threshold = 0.5    # Warn at 50% dir usage
```

**Rationale**: Single percentage thresholds fail for small values (0.8 × 5 = 4) and can't differentiate metrics.

---

### Task 11.12: Rename deny_file_patterns → deny_files, Add deny_dirs

Unify file/directory deny patterns with consistent naming.

**Behavior**:
- Rename `deny_file_patterns` → `deny_files` (global and rule-level)
- Add `deny_dirs` for directory basename patterns (global and rule-level)
- Both fields match basenames only (no path separators)
- Alias old name for backward compatibility

**Config Example**:
```toml
[structure]
deny_files = ["*.bak", "secrets.*", ".DS_Store"]  # File basename patterns
deny_dirs = ["__pycache__", "node_modules", ".git"]  # Directory basename patterns
deny_extensions = ["dll", "exe"]  # Extension-only patterns

[[structure.rules]]
scope = "src/**"
deny_files = ["util.rs", "helper.rs"]  # Scoped file deny
deny_dirs = ["temp_*"]  # Scoped directory deny
```

---

### Task 11.13: Structure Allowlist Mode

Add allowlist-based filtering as an alternative to deny-based filtering.

**Behavior**:
- Add `allow_files`, `allow_dirs`, `allow_extensions` fields (global and rule-level)
- Rule-level mutual exclusion: a rule can only use allow-mode OR deny-mode, not both
- Global-level mutual exclusion: global config can only use allow-mode OR deny-mode
- Allow-mode semantics: only matching items are permitted; everything else is denied

**Config Example**:
```toml
# Deny mode (default)
[structure]
deny_files = ["*.bak", "secrets.*"]
deny_dirs = ["__pycache__", "node_modules"]

# Allow mode (mutually exclusive with deny at same level)
# [structure]
# allow_files = ["README.md", "LICENSE"]
# allow_dirs = ["src", "tests"]
# allow_extensions = ["rs", "go", "py"]

[[structure.rules]]
scope = "src/generated"
max_files = -1
allow_extensions = ["rs"]  # Only allow .rs files here
```

**Validation**: Error if both allow and deny fields are set at the same level.

---

### Task 11.3: Time-bound Overrides

Add expiration dates to overrides for technical debt management.

**Behavior**:
- Add `expires = "YYYY-MM-DD"` to `[[content.override]]` and `[[structure.override]]`
- Expired overrides become violations (treated as if override doesn't exist)
- Optional: warning N days before expiration (configurable via `warn_expiry_days`)

**Config Example**:
```toml
[[content.override]]
path = "src/legacy/big_file.rs"
max_lines = 2000
reason = "Core legacy logic, too risky to split."
expires = "2025-12-31"
```

---

### Task 11.4: Baseline Ratchet

Enforce that violation count can only decrease over time.

**Behavior**:
- Add `--ratchet` flag or `baseline.ratchet = true` config
- When current violations < baseline count:
  - Default: emit warning "Baseline can be tightened"
  - `--ratchet=auto`: auto-update baseline silently
  - `--ratchet=strict`: fail CI if baseline not updated
- GitHub Action output: `baseline-outdated: true` for workflow conditionals

---

## Priority Order

| Priority | Tasks |
|----------|-------|
| ~~**1. State File Cleanup**~~ | ~~12.7 Remove V1 path_rules~~ ✅ |
| ~~**2. Git Diff Enhancement**~~ | ~~12.13 --diff A..B Explicit Range Syntax~~ ✅ |
| ~~**3. Code Quality**~~ | ~~14.1 Extract Path Matching~~ ✅, ~~14.2 CheckOptions Struct~~ ✅, ~~14.3 Scanner Module Split~~ ✅ |
| **4. Structure Naming** | ~~11.9 pattern→scope~~ ✅, 11.12 deny_file_patterns→deny_files + deny_dirs |
| **5. Governance Refinement** | 11.10 Content Exclude, 11.11 Granular Warn, 11.13 Allowlist Mode |
| **6. Debt Lifecycle** | 11.3 Time-bound Overrides, 11.4 Baseline Ratchet |
| **7. Visualization** | 7.1-7.2 HTML Charts/Trends |

