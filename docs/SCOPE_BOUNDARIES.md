# Scope Boundaries

> **Purpose**: Define what sloc-guard will NOT implement to maintain focus, simplicity, and long-term maintainability.

## Core Philosophy

sloc-guard is a **simple, opinionated SLOC enforcement tool**. Its value comes from being predictable and auditable, not from being a universal code quality platform.

**Guiding principle**: Complexity through composition, not through built-in features.

---

## Features We Will NOT Implement

### 1. Turing-Complete Rule DSL

**Won't do:**
- Conditional expressions (`if file.age > 30days then max_lines = 1000`)
- Logical operators between rules (AND/OR/NOT)
- Variables, macros, or templating in config

**Why:** The current glob pattern + last-match-wins model is expressive enough. A complex DSL would:
- Make `explain` output incomprehensible
- Create debugging nightmares for users
- Cause test combination explosion

**Current boundary:** Declarative rules with simple glob patterns.

---

### 2. Code Quality Metrics Beyond SLOC

**Won't do:**
- Cyclomatic complexity detection
- Code duplication detection
- Function parameter count limits
- Dependency/import analysis
- Code smell detection

**Why:** This is SonarQube/Clippy/ESLint territory. sloc-guard's core value is **simple, brutal line count enforcement**, not comprehensive code quality analysis.

**Current boundary:** Line counts (code/comment/blank) and directory structure only.

---

### 3. Auto-Fix / Refactoring Capabilities

**Won't do:**
- Automatic file splitting
- Code reformatting
- Automatic renaming
- Code generation

**Why:** `--suggest` is the correct boundary—**tell users how to split, but never touch their code**. Auto-fix requires semantic understanding and carries high risk of breaking code.

**Current boundary:** Suggestions via `--suggest` flag, no code modification.

---

### 4. Deep IDE Integration

**Won't do:**
- Language Server Protocol (LSP) implementation
- Real-time in-editor diagnostics
- IDE-specific plugin development
- Editor extensions

**Why:** SARIF output already enables VS Code and GitHub to display issues. The ROI of building LSP support is extremely low.

**Current boundary:** SARIF/JSON output for tooling consumption.

---

### 5. Full AST Parsing

**Won't do:**
- Complete syntax tree parsing for all languages
- Semantic-level analysis (type inference, etc.)
- Language version tracking and updates

**Why:** The current `FunctionParser` uses regex/simple heuristics and is sufficient for split suggestions. Full parser maintenance means chasing every language version update.

**Current boundary:** Simple heuristic-based function detection for suggestions only.

---

### 6. Git Hosting Platform Integration

**Won't do:**
- PR comment bots
- Webhook handlers
- GitHub App authentication
- Direct API integration with GitHub/GitLab/Bitbucket

**Why:** Let CI call sloc-guard and output SARIF/Markdown. Let GitHub Actions/GitLab CI handle the platform integration. Don't become a bot.

**Current boundary:** CLI tool that CI pipelines invoke.

---

### 7. Web Dashboard / Server Mode

**Won't do:**
- Built-in HTTP server
- Database storage for history
- Multi-project aggregation views
- User authentication/authorization

**Why:** `stats report --format html` is the correct boundary—**generate static reports, let users host them**. We're a CLI tool, not a SaaS platform.

**Current boundary:** Static HTML/Markdown report generation.

---

### 8. Fine-Grained Inline Controls

**Won't do:**
- Per-function line limits
- Inline annotations to adjust counts (`// sloc: +5`)
- Per-line ignore granularity

**Why:** Current ignore directives (`ignore-file`, `ignore-next`, `ignore-start/end`) are sufficient. Finer granularity would fragment rules and make global reasoning impossible.

**Current boundary:** File-level and block-level ignore directives.

---

### 9. Plugin / Extension System

**Won't do:**
- User-defined checker logic
- Dynamic module loading
- Third-party checker interfaces
- Scripting hooks

**Why:** `extends` + presets is the correct extension model—**configuration-level extension, not code-level extension**. Plugins would create version compatibility nightmares.

**Current boundary:** Config inheritance via `extends` (local/remote/preset).

---

### 10. Multi-Repository Orchestration

**Won't do:**
- Cross-repo aggregation
- Monorepo workspace management
- Repository discovery/scanning

**Why:** Each invocation targets one project root. For monorepos, users configure once at root with path-based rules. We don't orchestrate multiple repos.

**Current boundary:** Single project root per invocation.

---

## Boundary Zone: Preset/Documentation Instead of Core Features

| Requirement | Approach |
|-------------|----------|
| Framework-specific rules (React, Vue, etc.) | → `preset:react-strict` |
| Complex monorepo templates | → Documentation in RECIPES.md |
| Industry compliance rules (MISRA, etc.) | → Community-maintained `extends` URLs |
| Team-specific conventions | → Remote config with `extends_sha256` |

---

## Benefits of Clear Boundaries

1. **Predictable behavior** — Users can reason about what the tool does
2. **Auditable output** — `explain` command remains useful
3. **Maintainable codebase** — 90%+ test coverage stays achievable
4. **Fast iteration** — New features don't break existing workflows
5. **Clear documentation** — Less to explain, fewer edge cases

---

## When to Reconsider

A boundary might be reconsidered if:

1. **80%+ of users** request the same feature through issues
2. The feature can be implemented **without breaking existing behavior**
3. It doesn't require **ongoing language/platform tracking**
4. The maintenance burden is **bounded and predictable**

Until then, prefer pointing users to complementary tools rather than expanding scope.
