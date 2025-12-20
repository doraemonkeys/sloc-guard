# sloc-guard GitHub Action

Enforce source lines of code limits and directory structure constraints in your CI/CD pipeline.

## Usage

```yaml
- uses: <owner>/sloc-guard/.github/action@main
  with:
    paths: 'src'
```

## Inputs

| Input | Description | Default |
|-------|-------------|---------|
| `paths` | Paths to check (space-separated) | `.` |
| `config-path` | Path to sloc-guard.toml | Auto-detected |
| `fail-on-warning` | Treat warnings as failures | `false` |
| `version` | sloc-guard version to install | `latest` |
| `cache` | Enable result caching | `true` |
| `sarif-output` | Path for SARIF output file | (disabled) |
| `baseline` | Path to baseline file | (disabled) |
| `diff` | Only check files changed since ref | (disabled) |

## Outputs

| Output | Description |
|--------|-------------|
| `total-files` | Total files checked |
| `passed` | Files that passed |
| `failed` | Files that failed |
| `warnings` | Files with warnings |
| `grandfathered` | Violations grandfathered via baseline |
| `sarif-file` | Path to SARIF file (if generated) |

## Features

### Job Summary

The action automatically generates a Job Summary with check results that appears in the GitHub Actions workflow run page. The summary includes:
- Overall status (passed/failed/warnings)
- File count breakdown (total, passed, warnings, failed, grandfathered)
- SARIF output path (if configured)

### PR Annotations

Violations are automatically annotated on the affected files in pull requests:
- **Errors**: Files exceeding the SLOC limit
- **Warnings**: Files approaching the limit

These annotations appear directly in the PR diff view, making it easy to identify problematic files.

## Examples

### Basic Usage

```yaml
name: SLOC Guard
on: [push, pull_request]

jobs:
  check:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: <owner>/sloc-guard/.github/action@main
```

### PR Checks with Diff Mode

```yaml
- uses: <owner>/sloc-guard/.github/action@main
  with:
    diff: 'origin/main'
    fail-on-warning: 'true'
```

### SARIF Upload to GitHub Security

```yaml
- uses: <owner>/sloc-guard/.github/action@main
  id: sloc-guard
  with:
    sarif-output: 'sloc-guard.sarif'

- uses: github/codeql-action/upload-sarif@v3
  if: always()
  with:
    sarif_file: ${{ steps.sloc-guard.outputs.sarif-file }}
```

### Using Outputs

```yaml
- uses: <owner>/sloc-guard/.github/action@main
  id: sloc-guard

- run: |
    echo "Checked ${{ steps.sloc-guard.outputs.total-files }} files"
    echo "Passed: ${{ steps.sloc-guard.outputs.passed }}"
    echo "Failed: ${{ steps.sloc-guard.outputs.failed }}"
```
