---
type: task_blockers
status: active
priority: p0
updated: 2026-05-14
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

## FEAT.11 Learning Material Review (Blocked)

Status: blocked

Blocked by:
- ADR-028 accepted with explicit local-context security boundary.
- TD.5.A verify baseline execution gate.
- PERF.1 low-memory runtime baseline.
- TD.1.A IPC boundary.
- TD.2.A composition root extraction.
- TD.3.A typed DTO baseline.

Until unblocked, do not:
- Add runtime local-context scanning features that touch private user files.
- Add agent tool paths that bypass approval-gated scan scope selection.
- Add full-content recursive document indexing for this feature.

Allowed before unblock:
- ADR drafting and review.
- Config/DTO design scaffolding that does not activate runtime scanning behavior.
- Unit tests for denylist/redaction logic in isolation.

## RAM Budget Scope Guard

Status: constrained

Rule:
- The target "Background Core < 100 MB" excludes active WebView, loaded local LLM model memory, PTY terminal sessions, monitoring streams, and index rebuild tasks.

Do not:
- Claim end-to-end "Keynova + loaded local model" under 100 MB.
- Regress startup by enabling heavy runtime services in background by default.

## Refactor Backlog Execution Gate

Only execute TD.1-TD.5 early when they are required by current mainline tasks.

Current known dependency gates:
- PERF.2 aligns with TD.4.B (SearchService actor boundary).
- PERF.3 aligns with TD.4.C (Terminal actor boundary).
