---
name: task-implementation
description: Execute next task from Implementation Plan. Use when user asks to implement next task, start next feature, or continue implementation.
---

# Task Implementation

## Workflow

1. Review `docs/PROJECT_OVERVIEW.md` and `docs/Implementation Plan.md` to understand project context and identify next pending task by priority order
2. Assess task size and choose approach:
   - **Small**: Implement directly
   - **Medium**: Enter plan mode - design complete solution before coding
   - **Large**: Invoke task-splitting skill to break into 2-3 subtasks
3. Implement with tests to maintain coverage (Design for Testability)
4. Run `make ci` to verify
5. Invoke docs-maintenance skill to update documentation

