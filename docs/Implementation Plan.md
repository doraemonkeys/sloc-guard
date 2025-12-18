# sloc-guard Implementation Plan

> **Doc Maintenance**: Keep concise, avoid redundancy, clean up outdated content promptly to reduce AI context usage.

## Quick Reference

```
Exit Codes: 0=pass, 1=threshold exceeded, 2=config error, 3=IO error
Lint: make ci
```

## Performance Notes

> **Completed optimizations**: Parallel processing (rayon), HashSet for extensions, pre-indexed rule lookup, streaming file reading for large files (>10MB), metadata-based cache validation (mtime + size check avoids file read on cache hit).
>
> **Future considerations**: When adding new features, maintain these patterns:
> - Use `par_iter()` for file processing loops
> - Prefer O(1) lookups (HashMap/HashSet) over linear searches
> - Use `BufReader` for large file handling
> - **Structure Checks**: Perform directory entry counting using metadata only (no file opening).

---

## Completed (Compressed)

All modules in PROJECT_OVERVIEW.md Module Map are implemented. Additional completed features:

- **Phase 1-3**: Core MVP, Color Support, Git Diff Mode, Git-Aware Exclude
- **Phase 4**: Path-Based Rules, Inline Ignore (file/block/next), Strict Mode, Baseline (format/update/compare), SARIF Output, Progress Bar, File Hash Cache, Per-rule warn_threshold, Override with Reason, Custom Language Definition, Config Inheritance (local extends), Split Suggestions (--fix), Remote Config Support (http/https extends with caching, --no-extends flag)
- **Phase 5 (Partial)**: Language Breakdown (--group-by lang), Top-N & Metrics (--top N), Markdown Output, Directory Statistics (--group-by dir), Trend Tracking (--trend, .sloc-guard-history.json), HTML Report (--format html, summary + file list + sortable columns + status filtering), Structure Guard (config schema + analyzer + check integration)

---

## Phase 5.5: Code Quality & Architecture Refactoring (Pending)

Focus: Address architecture flaws, Scanner/Structure visibility conflict, UX ambiguities, and CLAUDE.md violations.

### Task 5.5.1: Scanner vs Structure Visibility Conflict (Critical)
> ⚠️ **Plan Mode Required**: 跨 4+ 模块的架构重构，先制定完整计划再实施。

Location: `src/config/model.rs`, `src/scanner/*.rs`, `src/checker/structure.rs`
**Problem**: `[scanner].extensions` filters files globally → Structure checker "blind" to non-code files.
- Scenario: Dir has 100 `.txt` files, scanner only sees `.rs` → Structure reports 0 files → false pass.

**Solution**: Decouple file discovery from content filtering.
```
[scanner]          # Physical discovery - NO extension filter
  gitignore = true
  exclude = [...]  # Global excludes (node_modules, target, etc.)

[content]          # SLOC analysis scope
  extensions = ["rs", "ts", ...]  # Only these for line counting
  max_lines = 400
  ...
```
- `Scanner` returns ALL files (respecting gitignore + exclude only).
- `ThresholdChecker` filters by `content.extensions` before counting.
- `StructureChecker` sees full directory contents (uses its own `count_exclude`).

### Task 5.5.2: Override Separation (Content vs Structure)
> ⚠️ **Plan Mode Required**: 配置结构拆分 + checker 适配，涉及多模块联动。

Location: `src/config/model.rs`, `src/checker/*.rs`
**Problem**: `[[override]]` mixing file limits and directory limits causes semantic confusion.
- Same array contains two different concepts (file SLOC vs directory counts)
- Only way to distinguish is by field presence (`max_lines` vs `max_files/max_dirs`)
- Edge case: user sets `max_files` on a file path, or `max_lines` on a directory → undefined behavior
- `[[content.rules]]` vs `[[content.override]]` 语义重叠 - 用户不知道该用哪个

**Solution**: Split into `[[content.override]]` and `[[structure.override]]` with clear semantics.
```toml
# Content override (file SLOC limits)
[[content.override]]
path = "src/legacy/god_object.rs"
max_lines = 5000
reason = "Legacy core"  # reason REQUIRED

# Structure override (directory limits)
[[structure.override]]
path = "src/legacy_module"
max_files = 500
max_dirs = 100
reason = "Legacy monolith, gradual migration in progress"  # reason REQUIRED
```
- `ContentOverride { path, max_lines, reason }` - file-only
- `StructureOverride { path, max_files, max_dirs, reason }` - directory-only
- Type safety: loader validates path type matches override type
- **Semantic distinction**:
  - `[[content/structure.override]]` = **豁免** (只能放宽限制, reason 必填)
  - `[[content/structure.rules]]` = **规则** (可严可宽, 用于批量 glob 匹配)
- Loader validates: override.max_lines >= effective rule limit (error if stricter)

### Task 5.5.3: Extension-Based Rule Syntax Sugar
> ⚠️ **Plan Mode Required**: 新增配置语法 + loader 展开逻辑 + 字段一致性验证。

Location: `src/config/model.rs`, `src/config/loader.rs`
**Problem**: Removing `[rules.<ext>]` in favor of `[[content.rules]]` pattern degrades UX for common case.
- Old: `[rules.rs] max_lines = 1000` (simple, intuitive)
- New: `[[content.rules]] pattern = "**/*.rs"` (verbose, error-prone glob)

**Solution**: Support both syntaxes with full field parity.
```toml
# Shorthand (extension-based, implicit **/*.ext)
[content.languages.rs]
max_lines = 1000
warn_threshold = 0.9   # Must support ALL fields that [[content.rules]] supports
skip_comments = true
skip_blank = true

# Full pattern (for complex cases like *.test.ts)
[[content.rules]]
pattern = "**/*.test.ts"
max_lines = 1500
```
- Priority: `[[content.rules]]` > `[content.languages.<ext>]` > `[content]` defaults.
- Loader expands `[content.languages.rs]` into internal `PathRule { pattern: "**/*.rs", ... }`.
- **Field parity**: `[content.languages.X]` MUST support all fields that `[[content.rules]]` supports.

### Task 5.5.4: Structure Pattern Semantics Clarification ✅
Location: `src/checker/structure.rs`, `docs/sloc-guard.example.toml`
**Completed**: Enforced directory-only matching with clear glob semantics.
- `structure.rules` patterns ONLY match directories (by design: `dir_stats` only contains directory paths)
- Glob behavior documented:
  - `src/components/*` → matches DIRECT children only (Button/, Icon/)
  - `src/components/**` → matches ALL descendants recursively
  - `src/features` → exact match only
- Added doc comments to `get_limits()` and `check()` methods clarifying semantics
- Example TOML includes explicit documentation and usage examples

### Task 5.5.5: Naming & Semantics Polish ✅
Location: `src/config/model.rs`, `docs/sloc-guard.example.toml`
**Completed**: Renamed `structure.ignore` → `structure.count_exclude` for semantic clarity.
- "ignored" implies "invisible/not scanned" which is misleading
- `count_exclude` = "exists but doesn't count toward quota" (accurate)
```
Documentation for `scanner.exclude` vs `structure.count_exclude`:
  | Config                       | Effect                                      |
  |------------------------------|---------------------------------------------|
  | `scanner.exclude = ["*.svg"]`| Completely invisible to ALL checkers        |
  | `structure.count_exclude`    | Visible but doesn't count toward dir quota  |
```

### Task 5.5.6: Rename `common.rs` Module ✅
Location: `src/commands/context.rs` (renamed from `common.rs`)
**Completed**: Renamed to `context.rs`, updated all imports.

### Task 5.5.7: Refactor `CheckResult` to Enum ✅
Location: `src/checker/threshold.rs`
**Completed**: Refactored `CheckResult` from struct to enum with associated data.
```rust
pub enum CheckResult {
    Passed { path, stats, limit, override_reason },
    Warning { path, stats, limit, override_reason, suggestions },
    Failed { path, stats, limit, override_reason, suggestions },
    Grandfathered { path, stats, limit, override_reason },
}
```
- Removed `CheckStatus` enum (redundant with variant)
- Added accessor methods: `path()`, `stats()`, `limit()`, `override_reason()`, `suggestions()`
- Consuming transformations: `into_grandfathered()`, `with_suggestions()`
- Updated all output formatters and commands

### Task 5.5.8: Config Versioning (Partial ✅)
Location: `src/config/model.rs`, `src/config/loader.rs`
**Completed**: Added `version` field to config schema with validation.
- `CONFIG_VERSION` constant = "1"
- `version` field in `Config` struct (optional, defaults to None)
- Loader validates version on load: unsupported version → error with message
- Missing version allowed (for backward compatibility)

**Remaining** (blocked on Task 5.5.1/5.5.2):
- Migration path: v1 config auto-converted to v2 internally
- Warn when `version` is missing (deprecation notice)

### Task 5.5.9: Rule Priority Chain Documentation & Enforcement
> ⚠️ **Plan Mode Required**: 规则优先级逻辑重构，跨 loader/checker 模块。

Location: `src/checker/threshold.rs`, `src/config/loader.rs`, `docs/sloc-guard.example.toml`
**Problem**: Multiple rules can match same file, priority unclear.
- `[[content.rules]] pattern = "tests/**"` vs `[[content.rules]] pattern = "**/*.test.ts"`
- Which wins for `tests/foo.test.ts`?
- `[content.languages.rs]` 和 `[[content.rules]] pattern = "**/*.rs"` 本质相同却可能有不同结果

**Solution**: Define and enforce explicit priority chain.
```
Content Rule Priority (high → low):
1. [[content.override]] - exact path match
2. [[content.rules]] - LAST declared match wins (later rules override earlier)
3. [content.languages.<ext>] - extension shorthand
4. [content] defaults

Structure Rule Priority (high → low):
1. [[structure.override]] - exact path match
2. [[structure.rules]] - LAST declared match wins
3. [structure] defaults
```
- **Implementation**: Loader expands `[content.languages.X]` into internal rules and **inserts at HEAD** of rule chain
  - This ensures explicit `[[content.rules]]` always override language shorthand
  - User writes: `[content.languages.rs]` + `[[content.rules]] pattern="**/*.rs"`
  - Internal: rules list = `[expanded_rs_rule, explicit_rs_rule]` → explicit wins (LAST match)
- Add comments in example TOML documenting priority chain
- Consider: warn on overlapping rules at config load time (optional strict mode)

### Task 5.5.10: Structure warn_threshold Symmetry ✅
Location: `src/config/model.rs`, `src/checker/structure.rs`
**Completed**: Added `warn_threshold` to `StructureConfig` and `StructureRule`.
- Content: `max_lines = 400, warn_threshold = 0.8` → warns at 320
- Structure: `max_files = 50, warn_threshold = 0.9` → warns at 45
```toml
[structure]
max_files = 50
max_dirs = 10
warn_threshold = 0.9  # Warn at 45 files / 9 dirs

[[structure.rules]]
pattern = "src/components/*"
max_files = 5
warn_threshold = 0.8  # Override threshold per rule
```
- `StructureConfig { warn_threshold: Option<f64> }`
- `StructureRule { warn_threshold: Option<f64> }`
- `StructureViolation { is_warning: bool }` - distinguishes warnings from failures

### Task 5.5.11: "Unlimited" Special Value Semantics ✅
Location: `src/config/model.rs`, `src/checker/structure.rs`, `docs/sloc-guard.example.toml`
**Completed**: Use `-1` (UNLIMITED) to express "no limit" for a specific field.
- `max_dirs = -1` means unlimited (skip check for this field)
- Changed `max_files`/`max_dirs` from `Option<usize>` to `Option<i64>`
- Added `UNLIMITED` constant (`-1`)
- Validation rejects values < `-1`
- Documentation updated in example TOML

### Task 5.5.12: Add `extends` Examples to Documentation ✅
Location: `docs/sloc-guard.example.toml`
**Completed**: Added extends examples (local/remote inheritance, override values, `--no-extends` flag).

---

## Phase 6: Statistics Extension (Pending)

### Task 6.1: HTML Charts (Pure CSS)
Location: `src/output/html.rs`
```
- File size distribution bar chart (pure CSS)
- Language/extension breakdown pie chart
- No external dependencies
```

### Task 6.2: HTML Trend Visualization
Location: `src/output/html.rs`
```
- Integrate with .sloc-guard-history.json (if exists)
- Line chart showing SLOC over time
- Delta indicators (+/-) from previous run
```

---

## Phase 7: CI/CD Support (Pending)

### Task 7.1: GitHub Action
```
- Create reusable GitHub Action
- Input: paths, config-path, fail-on-warning
- Output: total-files, passed, failed, warnings
```

### Task 7.2: Pre-commit Hook
```
- Document .pre-commit-config.yaml setup
- Support staged files only mode
```

---

## Priority Order

| Priority | Tasks |
|----------|-------|
| **1. Critical Architecture** | 5.5.1 Scanner/Structure Visibility, 5.5.2 Override Separation |
| **2. UX & Semantics** | 5.5.3 Extension Syntax Sugar, ~~5.5.4 Pattern Semantics~~, ~~5.5.5 Naming~~, 5.5.9 Priority Chain, ~~5.5.10 Structure warn_threshold~~, ~~5.5.11 Unlimited Value~~ |
| **3. Documentation** | ~~5.5.12 extends Examples~~ |
| **4. Code Quality** | ~~5.5.6 Rename common.rs~~, ~~5.5.7 CheckResult Enum~~, ~5.5.8 Versioning~ (partial) |
| **5. Deferred** | 6.1-6.2 HTML Charts/Trends, Phase 7 CI/CD |

---

## Architecture Notes

### Dependency Flow

```
main.rs (CLI parsing + dispatch)
  -> commands/check | stats | baseline_cmd | init | config
  -> context (shared: load_config, cache)
  -> config/loader (load config)
  -> scanner (find files)
  -> language/registry (get comment syntax)
  -> counter/sloc (count lines)
  -> checker/threshold (check limits)
  -> checker/structure (check structure limits)
  -> output/* (format results)
```
