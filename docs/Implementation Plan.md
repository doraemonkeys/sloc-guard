# sloc-guard Implementation Plan

> **Doc Maintenance**: Keep concise, avoid redundancy, clean up outdated content promptly to reduce AI context usage.

## Quick Reference

```
Exit Codes: 0=pass, 1=threshold exceeded, 2=config error, 3=IO error
Lint: make ci
```

## Performance Notes

> **Completed optimizations**: Parallel processing (rayon), HashSet for extensions, pre-indexed rule lookup, streaming file reading for large files (>10MB), metadata-based cache validation (mtime + size check avoids file read on cache hit), unified directory traversal (single WalkDir pass for both file discovery and structure checking).
>
> **Future considerations**: When adding new features, maintain these patterns:
> - Use `par_iter()` for file processing loops
> - Prefer O(1) lookups (HashMap/HashSet) over linear searches
> - Use `BufReader` for large file handling
> - **Structure Checks**: Perform directory entry counting using metadata only (no file opening).

---

## Completed (Compressed)

All modules in PROJECT_OVERVIEW.md Module Map are implemented.

- **Phase 1-3**: Core MVP, Color Support, Git Diff Mode, Git-Aware Exclude
- **Phase 4**: Path-Based Rules, Inline Ignore, Strict Mode, Baseline, SARIF Output, Progress Bar, File Hash Cache, Per-rule warn_threshold, Override with Reason, Custom Language Definition, Config Inheritance, Split Suggestions, Remote Config.
- **Phase 5 (Partial)**: Language Breakdown, Top-N & Metrics, Markdown Output, Directory Statistics, Trend Tracking, HTML Report, Structure Guard.
- **Phase 5.5 (Refactoring & V2 Config)**:
  - **Architecture**: Scanner/Structure separation, `ScannerConfig` vs `ContentConfig`, `CheckResult` enum refactor, Dependency Injection (Context) implementation.
  - **Config V2**: Separated `[[content.override]]` vs `[[structure.override]]` (with required reason), Versioning (auto-migration v1→v2), `warn_threshold` for structure, Unlimited (`-1`) limits.
  - **UX**: Extension-based rule sugar (`[content.languages.rs]`), Explicit Rule Priority (Override > Rule > Lang > Default), Structure pattern semantics clarification, Renamed `structure.count_exclude`.
- **Phase 6 (Partial)**:
  - **6.1**: `--max-files`, `--max-dirs` CLI params for `check` command. Requires explicit `<PATH>` argument, overrides `[structure]` defaults (not rules).
  - **6.2**: `--diff` optional parameter. Defaults to `HEAD` when provided without value (`--diff` same as `--diff HEAD`).
  - **6.3**: `--history-file` parameter for `stats` command. Custom path for trend history file (default: `.sloc-guard-history.json`).
  - **6.4**: Documentation Clarification - CLI help text updates (paths vs --include, CLI override scope, --diff behavior), README.md creation.
  - **6.5.1**: `--update-baseline[=MODE]` for `check` command. Modes: `all`(default), `content`, `structure`, `new`. Baseline V2 format (tagged enum for content/structure entries). V1 auto-migration. Deprecates `baseline update` subcommand.
  - **6.6**: `--report-json <PATH>` for `check` command. Outputs `ProjectStatistics` JSON alongside check results (avoids separate stats run in CI).
  - **6.9.1**: CLI parameter renames: `--fix` → `--suggest`, `--no-skip-comments` → `--count-comments`, `--no-skip-blank` → `--count-blank`.
  - **6.9.2**: `config show` format enum - `ConfigOutputFormat` (Text, Json) replaces String parameter.
- **Phase 9 (Partial)**:
  - **9.1**: `explain` command - shows which rules/overrides apply to a path, displays rule chain with match status.
  - **9.2**: `max_depth` - limits directory nesting depth in `[structure]`, `[[structure.rules]]`, and `[[structure.override]]`. CLI `--max-depth` parameter. `StructureChecker` tracks depth during traversal.
  - **9.3**: `init --detect` - Auto-detect project type (Cargo.toml→Rust, package.json→Node, go.mod→Go, etc.). Generates language-appropriate V2 config with suitable `max_lines`, extensions, and exclude patterns. Monorepo support: detects subprojects and generates scoped `[[content.rules]]`.
  - **9.4**: Structure Whitelist Mode - `allow_extensions` / `allow_patterns` on `[[structure.rules]]`. Files not matching whitelist are `DisallowedFile` violations. Stricter than `count_exclude`. OR logic (extension OR pattern match).
  - **9.5**: Unified Directory Traversal - `scan_with_structure()` method on `FileScanner` trait. Single WalkDir pass collects files AND directory statistics. Eliminates redundant I/O from separate scanner and structure checker traversals.
- **Phase 10**: IO Abstraction for Pure Unit Testing, Replace unwrap() with expect().
- **Phase 11 (Partial)**:
  - **11.6**: Configuration Presets - `extends = "preset:<name>"` syntax. Built-in presets: rust-strict, node-strict, python-strict, monorepo-base. Presets define ecosystem-specific defaults (extensions, max_lines, exclude patterns, structure limits). Lower priority than explicit config (child overrides preset).
- **Phase 8 (Partial)**:
  - **8.1.1**: Core GitHub Action - `.github/action/action.yml` composite action. Inputs: paths, config-path, fail-on-warning, version, cache, sarif-output, baseline, diff. Outputs: total-files, passed, failed, warnings, sarif-file. Installation via cargo install from action repository. Cache integration for cargo registry, binary, and results.
  - **8.1.2**: Problem Matchers and Job Summary - `problem-matcher.json` for PR annotations (FAILED/WARNING patterns). Job Summary via `$GITHUB_STEP_SUMMARY` with status, file counts, grandfathered count. New output: `grandfathered`.
  - **8.1.3**: Binary Download Optimization - Download pre-built binaries from GitHub Releases (x86_64/ARM64 Linux, macOS, Windows). SHA256 checksum verification. Exponential backoff retry (3 retries). Fallback to cargo install if binary unavailable.
  - **8.2**: Pre-commit Hook - `.pre-commit-hooks.yaml` with `language: script`. Wrapper script `scripts/install-sloc-guard.sh` (OS/Arch detection, binary download with checksum, caching at `~/.cache/sloc-guard/`). New `--files` CLI parameter for pure incremental mode (skips directory scan, processes only listed files, disables structure checks).
  - **8.3**: Universal Docker Image - Multi-stage `Dockerfile` (rust:alpine builder → alpine:3.21 runtime, ~10MB). Multi-arch support (linux/amd64, linux/arm64) via `.github/workflows/docker.yml`. Publish to ghcr.io on release tags. CI platform examples in README (GitLab CI, Jenkins, Azure Pipelines, CircleCI).
  - **8.4**: Diff Mode Enhancement - `--staged` parameter for staging area only (mutually exclusive with `--diff`). Clarified semantics: `--diff HEAD` = uncommitted (staged + unstaged), `--staged` = staged only, `--diff origin/main` = PR changes.
  - **8.5**: SARIF Auto-Upload Guidance - README documentation with GitHub Action examples. SARIF output/upload to Security tab via `github/codeql-action/upload-sarif@v3`. Action inputs/outputs reference. Docker and CI platform examples (GitLab CI, Azure Pipelines).

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

## Phase 8: CI/CD Support (Pending)

### Task 8.1: GitHub Action

#### 8.1.1: Core Action with Cache ✅
(Completed - see Completed section)

#### 8.1.2: Problem Matchers and Job Summary ✅
(Completed - see Completed section)

#### 8.1.3: Binary Download Optimization ✅
(Completed - see Completed section)

### Task 8.2: Pre-commit Hook ✅
(Completed - see Completed section)

### Task 8.3: Universal Docker Image ✅
(Completed - see Completed section)

### Task 8.4: Diff Mode Enhancement ✅
(Completed - see Completed section)

### Task 8.5: SARIF Auto-Upload Guidance ✅
(Completed - see Completed section)

---

## Phase 9: Advanced Features (Pending)

(All Phase 9 tasks completed - see Completed section above)

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

### Task 11.8: Terminology Modernization
```
- Rename internal: "whitelist" → "allowlist" in code, docs, config field names
- Config: allow_extensions/allow_patterns already named correctly
- Update CLI help, error messages, documentation
- No functional change, pure naming cleanup
```

---

## Priority Order

| Priority | Tasks |
|----------|-------|
| ~~**1. Code Quality**~~ | ~~10.1 IO Abstraction, 10.2 expect() cleanup~~ ✅ |
| ~~**2. Structure Enhancements**~~ | ~~9.2 max_depth, 9.4 whitelist mode~~ ✅ |
| ~~**3. Performance**~~ | ~~9.5 Eliminate Redundant Directory Traversal~~ ✅ |
| **4. UX Improvements** | ~~9.3 Smart init~~ ✅, ~~11.6 Presets~~ ✅ |
| **5. CI/CD** | ~~8.1.1 Core Action~~ ✅, ~~8.1.2-8.1.3 GitHub Action~~ ✅, ~~8.2 Pre-commit Hook~~ ✅, ~~8.3 Docker Image~~ ✅, ~~8.4 Diff Mode Enhancement~~ ✅, ~~8.5 SARIF Guidance~~ ✅ |
| **6. Cleanup** | 11.8 Terminology Modernization |
| **7. Governance Deep Dive** | 11.1 Naming Convention, 11.2 Co-location, 11.7 Deny Patterns |
| **8. Debt Lifecycle** | 11.3 Time-bound Overrides, 11.4 Baseline Ratchet |
| **9. Visualization** | 7.1-7.2 HTML Charts/Trends |

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
