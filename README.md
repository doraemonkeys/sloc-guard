# sloc-guard

**Guard your codebase against complexity.**

`sloc-guard` is a high-performance command-line tool that enforces limits on **Source Lines of Code (SLOC)** and **Directory Structure**. Unlike passive counters that just tell you how big your project is, `sloc-guard` actively prevents code bloat and architectural decay by failing your build when thresholds are exceeded.

[![Crates.io](https://img.shields.io/crates/v/sloc-guard.svg)](https://crates.io/crates/sloc-guard)
[![Downloads](https://img.shields.io/crates/d/sloc-guard.svg)](https://crates.io/crates/sloc-guard)
[![License](https://img.shields.io/crates/l/sloc-guard.svg)](LICENSE)
[![CI](https://github.com/doraemonkeys/sloc-guard/actions/workflows/ci.yml/badge.svg)](https://github.com/doraemonkeys/sloc-guard/actions/workflows/ci.yml)
[![Test Coverage](https://img.shields.io/badge/coverage-90%25%2B-brightgreen.svg)](.github/workflows/ci.yml)
[![Rust](https://img.shields.io/badge/rust-2024%20edition-orange.svg)](https://www.rust-lang.org/)
[![PRs Welcome](https://img.shields.io/badge/PRs-welcome-brightgreen.svg)](https://github.com/doraemonkeys/sloc-guard/pulls)

---

## ❓ Why sloc-guard?

Large files and messy directory structures are silent killers of codebase maintainability. By the time you notice, the damage is done.

**sloc-guard** enforces limits *before* code is merged:
- 🎯 **SLOC Limits** — Prevent files from exceeding line count thresholds (comments and blanks excluded by default)
- 📁 **Structure Guards** — Enforce directory organization (max files/dirs, naming conventions, sibling rules)
- 🔄 **Git-Aware** — Check only changed files (`--diff`, `--staged`) for fast CI integration
- 📊 **Trend Tracking** — Monitor codebase growth over time with historical snapshots

### How is it different from other tools?

| Feature | sloc-guard | cloc | tokei | SCC |
|---------|------------|------|-------|-----|
| **Enforce limits** | ✅ | ❌ | ❌ | ❌ |
| **Directory structure rules** | ✅ | ❌ | ❌ | ❌ |
| **Path-based rule overrides** | ✅ | ❌ | ❌ | ❌ |
| **Git diff mode** | ✅ | ❌ | ❌ | ❌ |
| **Baseline grandfathering** | ✅ | ❌ | ❌ | ❌ |
| **Trend tracking** | ✅ | ❌ | ❌ | ❌ |
| **Split suggestions** | ✅ | ❌ | ❌ | ❌ |
| **SARIF output** | ✅ | ❌ | ❌ | ❌ |
| **Remote config inheritance** | ✅ | ❌ | ❌ | ❌ |

> Other tools *count* lines. sloc-guard *enforces* them.

### 🤖 Perfect for AI-Assisted Development

In the age of AI coding assistants, telling AI "don't write spaghetti code" is futile — it has no memory of your preferences. But **hard constraints work**: when sloc-guard fails the build, AI automatically responds by refactoring and splitting code.

Instead of endlessly reminding AI to keep files small, just let it hit the wall and fix itself.

---

## Quick Start

### Installation

**From crates.io:**

```bash
cargo install sloc-guard
```

**From source**

```bash
cargo install --git https://github.com/doraemonkeys/sloc-guard
```

**Or download pre-built binary from [GitHub Releases](https://github.com/doraemonkeys/sloc-guard/releases).**

### 30-Second Setup

```bash
# 1. Initialize config with project type detection
sloc-guard init --detect

# 2. Check your codebase
sloc-guard check

# 3. See project statistics
sloc-guard stats summary
```

That's it! sloc-guard will enforce a 600-line limit per file by default.

- HTML report

```bash
# (Optional) Generate a visual HTML report
sloc-guard check -f html -o output.html
# OR
sloc-guard stats report -f html -o report.html
```


### Configuration

Create `.sloc-guard.toml` in your project root:

```toml
version = "2"

# Optional: inherit from presets or remote configs
# extends = "preset:rust-strict"

[scanner]
gitignore = true                             # Respect .gitignore (default: true)
exclude = [".git/**", "vendor/**", "dist/**"] # Exclude from scanning entirely

[content]
extensions = ["rs", "go", "py", "js", "ts"]  # Files to check
max_lines = 500                              # Max lines per file
warn_threshold = 0.8                         # Warn at 80% (400 lines)
warn_at = 450                                # Absolute threshold (takes precedence over warn_threshold)
skip_comments = true                         # Don't count comments (default: true)
skip_blank = true                            # Don't count blank lines (default: true)
exclude = ["**/*_test.go"]                   # Skip SLOC check (still visible to structure rules)

[structure]
max_files = 30                               # Max files per directory
max_dirs = 10                                # Max subdirectories
max_depth = 8                                # Max nesting depth
warn_threshold = 0.8                         # Warn at 80% of limits
warn_files_at = 25                           # Absolute threshold (takes precedence)
warn_dirs_at = 8                             # Absolute threshold (takes precedence)
count_exclude = ["*.md", ".gitkeep"]         # Don't count these toward limits
deny_extensions = [".exe", ".dll", ".bak"]   # Forbidden file types
deny_files = [".DS_Store", "Thumbs.db"]      # Forbidden files

[baseline]
ratchet = "warn"                             # warn|auto|strict - violations can only decrease

[trend]
max_entries = 100                            # Keep last N snapshots
max_age_days = 90                            # Delete older entries
min_interval_secs = 3600                     # At most one entry per hour
min_code_delta = 10                          # Ignore changes < N lines
auto_snapshot_on_check = false               # Auto-record on successful check
```

---

## Features

### Content Rules (SLOC Limits)

Override line limits for specific paths (last match wins):

```toml
[[content.rules]]
pattern = "src/generated/**"
max_lines = 2000
reason = "Auto-generated code"

[[content.rules]]
pattern = "**/*_test.rs"
max_lines = 800
skip_comments = false                        # Override: count comments for tests
reason = "Test files need more space"

# Temporary exemption with expiration
[[content.rules]]
pattern = "src/legacy/parser.rs"
max_lines = 1500
reason = "Refactoring in progress - JIRA-1234"
expires = "2025-06-01"
```

### Structure Rules (Directory Organization)

Override structure limits and enforce naming conventions:

```toml
[[structure.rules]]
scope = "src/components/**"
max_files = 50
file_naming_pattern = "^[A-Z][a-zA-Z0-9]*\\.(tsx|css)$"
allow_extensions = [".tsx", ".css"]          # Only these extensions allowed
reason = "React components: PascalCase required"

[[structure.rules]]
scope = "src/features/**"
max_files = -1                               # -1 = unlimited
max_depth = 3
relative_depth = true                        # Depth relative to scope, not project root
siblings = [
    { match = "*.tsx", require = "{stem}.test.tsx" }
]
reason = "Feature modules: max 3 levels deep, every component needs a test"

[[structure.rules]]
scope = "tests/**"
max_files = -1
max_dirs = -1
reason = "No limits for test directories"
```

### Git Integration

Check only what changed for fast CI:

```bash
# Check files changed since main branch
sloc-guard check --diff main

# Check only staged files (pre-commit hooks)
sloc-guard check --staged

# Check specific commit range
sloc-guard check --diff v1.0..v2.0
```

### Baseline & Grandfathering

Adopt sloc-guard in existing projects without fixing everything at once:

```bash
# Create baseline from current violations
sloc-guard check --update-baseline

# Violations in baseline are "grandfathered" (pass with note)
sloc-guard check --baseline

# Ratchet mode: violations can only decrease over time
sloc-guard check --baseline --ratchet strict
```

### Stats Subcommands

Analyze your codebase with focused subcommands:

```bash
# Project-level summary
sloc-guard stats summary

# Top files by code lines
sloc-guard stats files --top 10 --sort code

# Language or directory breakdown
sloc-guard stats breakdown                   # By language (default)
sloc-guard stats breakdown --by dir --depth 2

# Trend comparison with history
sloc-guard stats trend                       # vs. last entry
sloc-guard stats trend --since 7d            # vs. 7 days ago

# View historical snapshots
sloc-guard stats history --limit 20

# Comprehensive report (combines all above)
sloc-guard stats report --format html -o report.html
```

Record a snapshot manually:
```bash
sloc-guard snapshot                          # Record current state to history
```

Output example (`stats trend --since 7d`):
```
Trend (vs 7 days ago):
  Code:     +127 lines (+0.4%)  ↑
  Comments:  -12 lines (-0.3%)  ↓
  Files:      +3               ↑

  Previous: 2024-12-22 @ abc123f (main)
  Current:  2024-12-29 @ def456a (main)
```

### Split Suggestions

When a file exceeds limits, get actionable suggestions:

```bash
sloc-guard check --suggest
```

Output:
```
✗ src/parser.rs: 723 lines (limit: 500)
  
  Split suggestion:
  ├── parse_expression() (lines 45-180, 135 lines)
  ├── parse_statement() (lines 182-340, 158 lines)
  └── parse_block() (lines 342-520, 178 lines)
```

### Multiple Output Formats

```bash
sloc-guard check --format text      # Human-readable (default)
sloc-guard check --format json      # Machine-readable
sloc-guard check --format sarif     # IDE integration (VS Code, GitHub)
sloc-guard check --format markdown  # Documentation
sloc-guard check --format html      # Rich reports with charts
```

### Explain Command

Debug which rules apply to a path:

```bash
sloc-guard explain src/components/Button.tsx
```

Output:
```
Path: src/components/Button.tsx
Matched Rule: [[content.rules]] #2
  Pattern: src/components/**
  Reason: "React components"
Effective Limit: 400 lines
Warn At: 320 lines (80%)
```

### Config Inheritance

Share configuration across projects:

```toml
# Built-in presets: rust-strict, node-strict, python-strict, monorepo-base
extends = "preset:rust-strict"

# Or remote URL with optional integrity check
extends = "https://example.com/team-config.toml"
extends_sha256 = "abc123..."

# Local values override inherited ones
[content]
max_lines = 600
```

### Custom Languages

Define comment syntax for unsupported languages:

```toml
[languages.hcl]
extensions = ["tf", "hcl"]
single_line_comments = ["#", "//"]
multi_line_comments = [["/*", "*/"]]
```

---

## CLI Reference

```bash
Usage: sloc-guard [OPTIONS] <COMMAND>

Commands:
  check     Check files against line count thresholds
  stats     Display statistics (subcommands: summary, files, breakdown, trend, history, report)
  snapshot  Record a statistics snapshot to trend history
  init      Generate a default configuration file
  config    Configuration file utilities
  explain   Explain which rules apply to a path
  help      Print this message or the help of the given subcommand(s)

Options:
  -v, --verbose...     Increase output verbosity (-v, -vv for more)
  -q, --quiet          Suppress non-essential output
      --color <COLOR>  Control color output [default: auto] [possible values: auto, always, never]
      --no-config                      Skip loading configuration file
      --no-extends                     Skip resolving extends in configuration (ignore remote/local inheritance)
      --extends-policy <MODE>          Remote config fetch policy [default: normal] [values: normal, offline, refresh]
  -h, --help                           Print help (see more with '--help')
  -V, --version        Print version
```

### Stats Subcommands

| Subcommand | Description | Key Flags |
|------------|-------------|-----------|
| `summary`  | Project totals (files, code, comments, blanks) | `--format` |
| `files`    | File list with sorting | `--top`, `--sort`, `--format` |
| `breakdown`| Group by language or directory | `--by`, `--depth`, `--format` |
| `trend`    | Delta comparison with history | `--since`, `--format` |
| `history`  | List historical snapshots | `--limit`, `--format` |
| `report`   | Comprehensive report | `--format`, `-o`, `--exclude-section` |



## Pre-commit Hook

Add to `.pre-commit-config.yaml`:

```yaml
repos:
  - repo: https://github.com/doraemonkeys/sloc-guard
    rev: master
    hooks:
      - id: sloc-guard
```

The hook uses `--staged` mode for fast incremental checks.

---

## Advanced Usage

### GitHub Actions

```yaml
name: Code Quality
on: [push, pull_request]

jobs:
  sloc-guard:
    runs-on: ubuntu-latest
    permissions:
      contents: read
      security-events: write  # Required for SARIF upload
    steps:
      - uses: actions/checkout@v4
        with:
          fetch-depth: 0  # Required for --diff mode

      - name: Run sloc-guard
        uses: doraemonkeys/sloc-guard/.github/action@master
        with:
          sarif-output: results.sarif
          # Only diff on PRs, check all files on push
          diff: ${{ github.event.pull_request.base.ref && format('origin/{0}', github.event.pull_request.base.ref) || '' }}

      - name: Upload SARIF
        uses: github/codeql-action/upload-sarif@v4
        with:
          sarif_file: results.sarif
```

See [Action README](.github/action/README.md) for all available inputs and outputs.

### Docker

```bash
# Linux/macOS/WSL
docker run --rm -v $(pwd):/workspace -w /workspace ghcr.io/doraemonkeys/sloc-guard check

# PowerShell
docker run --rm -v "${PWD}:/workspace" -w /workspace ghcr.io/doraemonkeys/sloc-guard check
```

Build locally:
```bash
docker build -t sloc-guard .
```

### Monorepo Setup

```toml
version = "2"
extends = "preset:monorepo-base"

[scanner]
exclude = [".git/**", "node_modules/**", "target/**", "dist/**"]

[[structure.rules]]
scope = "packages/*"
max_files = 50
max_dirs = 15

[[structure.rules]]
scope = "packages/*/src/**"
siblings = [
    { group = ["index.ts", "types.ts"] }  # If one exists, both must exist
]
```

---

## Performance

sloc-guard is designed for speed:

- **Parallel processing** via Rayon — utilizes all CPU cores
- **Intelligent caching** — skips unchanged files (mtime + size + hash)
- **Git-aware scanning** — respects `.gitignore` by default
- **Incremental mode** — `--diff` and `--staged` check only changed files



## 🛡️ Project Quality

- **90%+ test coverage** enforced by CI
- **Strict Clippy lints** (pedantic + nursery)
- **Comprehensive integration tests**
- **Dependency injection** for testability
- **Well-documented** internal architecture

See [ENGINEERING_GUIDELINES.md](docs/ENGINEERING_GUIDELINES.md) for coding standards.

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | All checks passed |
| 1 | Threshold exceeded (or warnings in `--strict` mode) |
| 2 | Configuration error |

---

## License

Apache-2.0 — see [LICENSE](LICENSE) for details.

