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

## Pre-commit Hook

sloc-guard integrates with the [pre-commit](https://pre-commit.com/) framework for automatic SLOC checking on every commit.

### Setup

Add to your `.pre-commit-config.yaml`:

```yaml
repos:
  - repo: https://github.com/doraemonkeys/sloc-guard
    rev: v0.1.0  # Pin to specific version
    hooks:
      - id: sloc-guard
        # Optional: restrict to specific file types
        # types_or: [rust, python, javascript, typescript]
```

Then run:

```bash
pre-commit install
```

### How it Works

1. **Binary Download**: On first run, downloads the matching pre-built binary to `~/.cache/sloc-guard/`
2. **Checksum Verification**: Verifies SHA256 checksum before installation
3. **Incremental Mode**: Uses `--files` parameter to check only staged files (no full scan)
4. **Version Pinning**: The `rev` in your config pins the sloc-guard version

### Manual Installation Alternative

If you prefer to install sloc-guard globally and use the system binary:

```yaml
repos:
  - repo: local
    hooks:
      - id: sloc-guard
        name: sloc-guard
        entry: sloc-guard check --files
        language: system
        types: [file]
        pass_filenames: true
```

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | All checks passed |
| 1 | Threshold violations found |
| 2 | Configuration or runtime error |

## License

Apache-2.0
