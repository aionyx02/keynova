---
type: task_blockers
status: active
priority: p0
updated: 2026-05-20
context_policy: retrieve_when_planning
owner: project
tags: [security, approval, shell, blockers, refactor]
---

# Blocked Tasks and Safety Constraints

## ADR-0029 Refactor Gate

Status: accepted (2026-05-20); REF.1+ implementation unlocked

Reason:
- ADR-0029 and derived ADR-0030 accepted by developer; AI capability layer refactor governs REF.1–REF.8 implementation.
- `AGENT.3` and `AI.1` remain superseded.

Do not:
- Implement `AGENT.3` or `AI.1` as separate tracks; they are superseded by the ADR-0029 / AI capability refactor.
- Add new product features before `REF.7` unless they are required by the refactor or fix a P0 regression.
- Change ADR-0029 / ADR-0030 status or scope without a new ADR (per governance §3).

Allowed:
- Implement REF.1–REF.8 in the order defined by `docs/tasks/refactor-ai-capability.md`.
- Draft v2-style follow-up ADRs when capability schema needs extending (e.g., risk tag category × severity → new ADR per ADR-0030 §9).

## Generic Shell Tool

Status: blocked

Reason:
- Product-level sandbox boundary is still incomplete for unrestricted shell execution.
- Arbitrary shell tools remain high destructive risk even with approval flow.
- Current audit/approval path is not sufficient for full generic execution.

Do not:
- Add generic shell execution tools.
- Allow direct LLM-triggered arbitrary system commands.

Allowed:
- Add deterministic typed tools.
- Keep approval-gated read-only or narrowly scoped commands.
- Keep bounded output and safety checks in tool observations.

## FEAT.11 Learning Material Review

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

## Parked Post-Refactor Tracks

Status: parked until `REF.7` completes

Tracks to revalidate after the refactor observation gate:
- `LAUNCH.2.C` per-workspace quick actions.
- `ONBOARD.1.D/E` re-engage prompt and hotkey guidance.
- `NOTE.1` daily note driver.
- `UTIL.1.B-online` online currency rates.
- `AGENT.4`, `CLIP.1`, `SNIP.1`, `WIN.1`, `UTIL.3`, `DEV.1`, and `SYNC.1`.

Do not:
- Expand these into detailed work items during the P0 refactor.
- Wire global hooks, clipboard capture, system monitoring, secret storage, or sync behavior without fresh ADR review after `REF.7`.
