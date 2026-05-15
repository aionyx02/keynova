---
type: task_index
status: active
priority: p0
updated: 2026-05-15
context_policy: always_retrievable
owner: project
tags: [feature-first, safety-first, performance]
last_change: feasibility review integrated from Keynova_Agent_Architecture_Tasks_Review.docx
---

# Active Tasks

## Feasibility Verdict (2026-05-14)

- Feasibility is high for low-memory runtime plus safe local review if rollout is staged.
- FEAT.11 runtime work remains blocked until ADR, security policy, and typed DTO prerequisites are complete.
- RAM goal is scoped to Background Core only (exclude active WebView, loaded LLM model, PTY terminal session, and monitoring streams).

## Strategy

Safety first for runtime lifecycle; feature-first delivery after guardrails are in place.

## Current Execution Order

1. TD.5.A minimal verify baseline as execution gate.
2. PERF.1 Low Memory Background Mode.
3. TD.1 + TD.2 + TD.3 prerequisite slices required by PERF and FEAT tracks.
4. PERF.2 Search Execution Bound.
5. PERF.3 Heavy Feature Lazy Runtime.
6. TD.4 actor-like services and remaining TD.5 hardening.
7. P3 Context Compiler Lite.
8. FEAT.11 Learning Material Review (blocked until prerequisites clear).

## Recent Execution Notes

- 2026-05-15: AI panel UX hotfix delivered for clear behavior and chronology:
  - `AGENT` mode now has backend clear command (`agent.clear_runs`) and frontend clear wiring.
  - Agent runs are rendered in chronological order (old -> new) to keep the newest request at the bottom.
  - Chat history hydration now avoids stale overwrite after local send/clear actions.

See `docs/tasks/backlog.md` for detailed breakdown.
See `docs/tasks/blocked.md` for gating constraints.
See `docs/tasks/completed.md` for P0/P1/P2 history.
