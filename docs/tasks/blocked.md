---
type: task_blockers
status: active
priority: p0
updated: 2026-05-13
context_policy: retrieve_when_planning
owner: project
tags: [security, approval, shell, blockers]
---

# Blocked Tasks and Safety Constraints

## Generic Shell Tool (Still Blocked)

Status: blocked

Reason:
- Product-level sandbox boundary is still incomplete for unrestricted shell execution.
- Arbitrary shell tool remains high destructive risk even with approval flow.
- Current audit/approval path is not sufficient for full generic execution.

Do not:
- Add generic shell execution tools.
- Allow direct LLM-triggered arbitrary system commands.

Allowed:
- Add deterministic typed tools.
- Keep approval-gated read-only or narrowly scoped commands.
- Keep bounded output and safety checks in tool observations.

## Workflow MVP Scope Guard

Status: constrained

For P1 workflow v0, do not add:
- branching
- parallel execution
- checkpoint/resume
- retry policy
- conditional execution

Rationale:
- Keep first release as linear pipeline only.
- Prevent MVP from expanding into full runtime rewrite.

## Refactor Backlog Execution Gate

Only execute TD.1-TD.5 early when they block P0-P3.

Current known dependency gates:
- TD.4.B depends on TD.2.A.
- TD.4.C depends on TD.2.B.
