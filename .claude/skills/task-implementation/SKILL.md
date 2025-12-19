---
name: task-implementation
description: Execute next task from Implementation Plan. Use when user asks to implement next task, start next feature, or continue implementation.
---

# Task Implementation

## Workflow

1. Depend on `docs/Implementation Plan.md`, identify next pending task by priority order
2. Assess task size and choose approach:
   - **Small**: Implement directly
   - **Medium**: Enter plan mode - design complete solution before coding
   - **Large**: Invoke task-splitting skill to break into 2-3 subtasks
3. Implement with tests to maintain coverage
4. Run `make ci` to verify
5. Update Implementation Plan (mark completed, add notes)

