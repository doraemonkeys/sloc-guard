# sloc-guard

Source Lines of Code enforcement tool - enforces file size limits by counting code lines (excluding comments and blanks) and enforces directory structure limits (file/folder counts).

## Installation

```bash
cargo install sloc-guard
```

## Quick Start

```bash
# Initialize config
sloc-guard init

# Check files
sloc-guard check

# View statistics
sloc-guard stats
```

## Commands

| Command | Description |
|---------|-------------|
| `check` | Check files against line count thresholds |
| `stats` | Display statistics without checking thresholds |
| `init` | Generate a default configuration file |
| `config validate` | Validate configuration file syntax |
| `config show` | Display the effective configuration |
| `baseline update` | Generate baseline for grandfathering violations |
| `explain <PATH>` | Show which rules apply to a path |

## CLI Parameters

### Scan Roots vs Include Filter

**Scan roots** (`<PATH>` arguments) are starting points for file discovery:
```bash
sloc-guard check src tests    # Scan src/ and tests/ directories
```

**Include filter** (`--include`, `-I`) restricts scanning to specific subdirectories, overriding both `<PATH>` arguments and config `include_paths`:
```bash
sloc-guard check --include src/core --include src/utils
```

Priority: `--include` > CLI `<PATH>` > config `include_paths` > default "."

### CLI Override Scope

CLI parameters like `--max-lines`, `--max-files`, `--max-dirs` override **config defaults only**, not rules:

```bash
# Overrides [content] max_lines default, but [[content.rules]] still take precedence
sloc-guard check --max-lines 200
```

### Diff Mode

`--diff` filters content checks to changed files only. Defaults to HEAD when no value provided:

```bash
sloc-guard check --diff          # Same as --diff HEAD
sloc-guard check --diff main     # Compare against main branch
```

**Structure checks** are NOT filtered by `--diff` - they always count full directory state. The `--diff` flag only limits which files are checked for SLOC violations.

## Configuration

Create `.sloc-guard.toml` with `sloc-guard init` or manually:

```toml
version = 2

[scanner]
gitignore = true
exclude = ["target/**", "node_modules/**"]

[content]
extensions = ["rs", "go", "py", "js", "ts"]
max_lines = 300
warn_threshold = 0.8
skip_comments = true
skip_blank = true

[[content.rules]]
pattern = "**/*_test.rs"
max_lines = 500

[[content.override]]
path = "src/legacy/huge_file.rs"
max_lines = 1000
reason = "Legacy code pending refactor"

[structure]
max_files = 20
max_dirs = 10

[[structure.rules]]
pattern = "src/components/**"
max_files = 30
```

### Rule Priority

**Content (SLOC limits):**
1. `[[content.override]]` - exact path match
2. `[[content.rules]]` - glob pattern (last match wins)
3. `[content]` defaults

**Structure (directory limits):**
1. `[[structure.override]]` - exact path match
2. `[[structure.rules]]` - glob pattern (last match wins)
3. `[structure]` defaults

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | All checks passed |
| 1 | Threshold violations found |
| 2 | Configuration or runtime error |

## License

Apache-2.0
