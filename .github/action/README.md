# sloc-guard GitHub Action

Enforce source lines of code limits and directory structure constraints in your CI/CD pipeline.

## Usage

```yaml
- uses: doraemonkeys/sloc-guard/.github/action@master
  with:
    paths: 'src'
```

## Inputs

| Input | Description | Default |
|-------|-------------|---------|
| `paths` | Paths to check (space-separated, see note below) | `.` |
| `config-path` | Path to sloc-guard.toml | Auto-detected |
| `fail-on-warning` | Treat warnings as failures | `false` |
| `version` | sloc-guard version to install | `latest` |
| `cache` | Enable result caching | `true` |
| `sarif-output` | Path for SARIF output file | (disabled) |
| `baseline` | Path to baseline file | (disabled) |
| `diff` | Only check files changed since ref | (disabled) |

**Note on `paths`:** Paths containing spaces are not supported. If you need to check directories with spaces in their names, use glob patterns in your `sloc-guard.toml` configuration instead.

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

### Fast Binary Downloads

The action downloads pre-built binaries from GitHub Releases when available, significantly reducing installation time compared to compiling from source.

**Supported platforms:**
- Linux (x86_64, ARM64)
- macOS (x86_64, ARM64)
- Windows (x86_64)

**Security:** All binaries are verified using SHA256 checksums before installation.

**Fallback:** If pre-built binaries are unavailable or checksum verification fails, the action automatically falls back to `cargo install`.

### Efficient Multi-Format Output

When SARIF output is enabled, the action uses single-run multi-format output (`--write-sarif` and `--write-json` flags) to generate all formats in one execution. This eliminates the need to run sloc-guard multiple times, significantly improving CI performance.

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

## Docker Image

A lightweight Docker image (~10MB) is available for use in any CI/CD platform.

```bash
docker pull ghcr.io/<owner>/sloc-guard:latest
```

**Supported platforms:** linux/amd64, linux/arm64

### Docker CLI Usage

```bash
# Check current directory
docker run --rm -v "$(pwd):/workspace" ghcr.io/<owner>/sloc-guard check .

# With custom config
docker run --rm -v "$(pwd):/workspace" ghcr.io/<owner>/sloc-guard check --config sloc-guard.toml src

# Generate SARIF output
docker run --rm -v "$(pwd):/workspace" ghcr.io/<owner>/sloc-guard check --format sarif src > sloc-guard.sarif

# Get statistics
docker run --rm -v "$(pwd):/workspace" ghcr.io/<owner>/sloc-guard stats src
```

### GitLab CI

```yaml
sloc-guard:
  image: ghcr.io/<owner>/sloc-guard:latest
  stage: lint
  script:
    - sloc-guard check src
  rules:
    - if: $CI_PIPELINE_SOURCE == "merge_request_event"
    - if: $CI_COMMIT_BRANCH == $CI_DEFAULT_BRANCH
```

**With SARIF artifact:**

```yaml
sloc-guard:
  image: ghcr.io/<owner>/sloc-guard:latest
  stage: lint
  script:
    - sloc-guard check --format sarif src > sloc-guard.sarif
  artifacts:
    reports:
      sast: sloc-guard.sarif
```

### Jenkins

**Declarative Pipeline:**

```groovy
pipeline {
    agent {
        docker {
            image 'ghcr.io/<owner>/sloc-guard:latest'
        }
    }
    stages {
        stage('SLOC Check') {
            steps {
                sh 'sloc-guard check src'
            }
        }
    }
}
```

**Scripted Pipeline:**

```groovy
node {
    stage('SLOC Check') {
        docker.image('ghcr.io/<owner>/sloc-guard:latest').inside {
            sh 'sloc-guard check src'
        }
    }
}
```

### Azure Pipelines

```yaml
trigger:
  - main

pool:
  vmImage: 'ubuntu-latest'

container: ghcr.io/<owner>/sloc-guard:latest

steps:
  - checkout: self

  - script: sloc-guard check src
    displayName: 'Run sloc-guard'
```

**With artifact publishing:**

```yaml
steps:
  - checkout: self

  - script: sloc-guard check --format sarif src > $(Build.ArtifactStagingDirectory)/sloc-guard.sarif
    displayName: 'Run sloc-guard'

  - task: PublishBuildArtifacts@1
    inputs:
      pathToPublish: '$(Build.ArtifactStagingDirectory)/sloc-guard.sarif'
      artifactName: 'CodeAnalysisLogs'
```

### CircleCI

```yaml
version: 2.1

jobs:
  sloc-guard:
    docker:
      - image: ghcr.io/<owner>/sloc-guard:latest
    steps:
      - checkout
      - run:
          name: Check SLOC limits
          command: sloc-guard check src

workflows:
  main:
    jobs:
      - sloc-guard
```
