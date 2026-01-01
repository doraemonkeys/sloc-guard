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

---

## Phase 23: Error Propagation & Fail-Fast

Silent failures mask configuration/environment issues. This phase surfaces errors early.

**23.1 ThresholdChecker::new() returns Result** ✅
- `build_content_exclude()` and `build_path_rules()` return `Result<_, SlocGuardError::InvalidPattern>`
- First invalid glob pattern fails immediately (no batch collection - matches Rust conventions)

**23.2 StructureChecker propagation** ✅
- `CheckContext::from_config()` propagates `StructureChecker::new()` errors instead of `.ok()`
- Invalid glob in structure rules surfaces immediately

**23.3 Cycle detection canonicalize must fail** ✅
- `loader.rs` cycle detection: `canonicalize()` returns `Result`, propagates error
- Fallback to original path could cause infinite recursion

**23.4 current_dir() propagation** ✅
- `commands/context.rs` and `commands/config.rs`: return `SlocGuardError::Io` on failure
- Silently setting `project_root = None` breaks downstream features

**23.5 save_cache returns Result** ✅
- `save_cache()` returns `io::Result<()>`, caller uses `let _ =` for explicit ignore
- No behavioral change, just explicit intent in code

**23.6 GlobSet build propagation** ✅
- `builder.build()` errors propagate instead of returning `GlobSet::empty()`
- Empty set means all user rules silently fail

---

## Phase 24: Config Design Improvements

Addresses counter-intuitive behaviors in config inheritance and ambiguous semantics.

**24.1 Array Merge with `$reset` Marker**
- Change `merge_toml_values()`: arrays default to append (parent + child)
- `$reset` as first element clears parent array, uses remaining child elements
- Validate `$reset` position in `Config::validate()` (must be first or error)
- Affects: `scanner.exclude`, `content.rules`, `structure.rules`

**24.2 Offline Cache Strategy Separation**
- Add `FetchPolicy` enum: `Normal` (TTL-controlled), `Offline` (ignore TTL), `ForceRefresh` (skip cache)
- `--offline` mode ignores TTL, uses any existing cache
- Move remote cache to state directory (`.git/sloc-guard/remote-configs/` or `.sloc-guard/remote-configs/`)
- SHA256 hash lock: if set, validate content regardless of TTL

**24.3 Check Behavior Configuration**
- Remove ambiguous `strict` field
- Add `[check]` section with `warnings_as_errors` (treat warnings as failures) and `fail_fast` (stop on first failure)
- Add CLI flags `--warnings-as-errors` and `--fail-fast`
- `fail_fast` implements short-circuit processing for performance

---

## Priority Order

| Priority               | Tasks                                                                                              |
| ---------------------- | -------------------------------------------------------------------------------------------------- |
| **19. Config Design**  | 24.1 Array Merge, 24.2 Offline Cache, 24.3 Check Behavior |
| **18. Error Handling** | ~~23.1 ThresholdChecker~~ ✅, ~~23.2 StructureChecker~~ ✅, ~~23.3 Canonicalize~~ ✅, ~~23.4 current_dir~~ ✅, ~~23.5 save_cache~~ ✅, ~~23.6 GlobSet~~ ✅ |
| **17. SARIF & Action** | ~~22.1 SARIF Rules~~ ✅, ~~22.2 SARIF Messages~~ ✅, ~~22.3 Multi-Format~~ ✅, ~~22.4 Action Fixes~~ ✅, ~~22.5 Binary Format~~ ✅ |
| **16. Stats Restructure** | ~~21.1 CLI~~ ✅, ~~21.2 Summary~~ ✅, ~~21.3 Files~~ ✅, ~~21.4 Breakdown~~ ✅, ~~21.5 Trend~~ ✅, ~~21.6 Report~~ ✅, ~~21.7 Snapshot~~ ✅, ~~21.8~~ ✅, ~~21.9~~ ✅, ~~21.10~~ ✅ |

