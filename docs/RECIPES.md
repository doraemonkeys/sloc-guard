# Recipes & Examples

Common configuration patterns for various project types.

## Monorepo Setup

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

## React/Next.js Project

```toml
version = "2"
extends = "preset:node-strict"

[content]
extensions = ["ts", "tsx", "js", "jsx"]
max_lines = 400
exclude = ["**/*.test.tsx", "**/*.stories.tsx"]

[[content.rules]]
pattern = "src/components/**"
max_lines = 300
reason = "Components should be small and focused"

[[structure.rules]]
scope = "src/components/**"
file_naming_pattern = "^[A-Z][a-zA-Z0-9]*\\.(tsx|css|module\\.css)$"
allow_extensions = [".tsx", ".css"]
reason = "React components: PascalCase required"

[[structure.rules]]
scope = "src/components/**"
siblings = [
    { match = "*.tsx", require = "{stem}.test.tsx", severity = "warn" }
]
reason = "Every component should have a test"
```

## Rust Library

```toml
version = "2"
extends = "preset:rust-strict"

[content]
max_lines = 500

[[content.rules]]
pattern = "**/*_tests.rs"
max_lines = 800
reason = "Test modules can be larger"

[[content.rules]]
pattern = "src/generated/**"
max_lines = 5000
reason = "Auto-generated code"
```

## Go Service

```toml
version = "2"

[content]
extensions = ["go"]
max_lines = 500
exclude = ["**/*_test.go", "**/*.pb.go"]

[[content.rules]]
pattern = "**/*_test.go"
max_lines = 1000
skip_comments = false
reason = "Test files can be verbose"

[[content.rules]]
pattern = "**/*.pb.go"
max_lines = -1  # Unlimited
reason = "Protobuf generated code"

[[structure.rules]]
scope = "cmd/*"
max_files = 5
reason = "Each command should be minimal"
```

## Python Django Project

```toml
version = "2"
extends = "preset:python-strict"

[content]
exclude = ["**/migrations/**"]

[[content.rules]]
pattern = "**/migrations/**"
max_lines = -1
reason = "Django migrations are auto-generated"

[[content.rules]]
pattern = "**/tests/**"
max_lines = 600
reason = "Test files need more space"

[[structure.rules]]
scope = "**/migrations/**"
max_files = -1
max_dirs = -1
reason = "No limits for migration directories"
```

## Temporary Exemptions

When you need to temporarily allow violations:

```toml
[[content.rules]]
pattern = "src/legacy/parser.rs"
max_lines = 1500
reason = "Refactoring in progress - JIRA-1234"
expires = "2025-06-01"

[[structure.rules]]
scope = "src/legacy/**"
max_files = -1
reason = "Legacy code - to be refactored"
expires = "2025-12-31"
```

## Sibling File Patterns

Enforce related files exist together:

```toml
[[structure.rules]]
scope = "src/features/**"
siblings = [
    # Directed: if .tsx exists, require matching .test.tsx
    { match = "*.tsx", require = "{stem}.test.tsx" },
    
    # Group: if any exists, all must exist
    { group = ["index.ts", "types.ts", "utils.ts"] }
]
```

## Remote Config Inheritance

Share team-wide configuration:

```toml
version = "2"

# From URL with integrity check
extends = "https://raw.githubusercontent.com/myorg/configs/main/sloc-guard.toml"
extends_sha256 = "abc123..."

# Override specific settings
[content]
max_lines = 600
```

## Mixed Language Project

```toml
version = "2"

[content]
extensions = ["rs", "py", "go", "ts"]
max_lines = 500

[[content.rules]]
pattern = "**/*.rs"
max_lines = 500
skip_comments = true

[[content.rules]]
pattern = "**/*.py"
max_lines = 400
skip_comments = true

[[content.rules]]
pattern = "**/*.go"
max_lines = 600
skip_comments = true

# Custom language definition
[languages.terraform]
extensions = ["tf", "hcl"]
single_line_comments = ["#", "//"]
multi_line_comments = [["/*", "*/"]]
```
