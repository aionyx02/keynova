---
type: task_index
status: backlog
priority: p1
updated: 2026-05-20
context_policy: on_demand
owner: project
tags: [roadmap, docs-governance, planning]
---

# Backlog Tasks

Backlog is on-demand planning context. During the P0 refactor track, it is not a runnable queue.

## P0

- (empty; current P0 work lives in `docs/tasks/active.md` and `docs/tasks/refactor-ai-capability.md`)

## P1

- [ ] Reassess docs guard threshold fit after several normal refactor commits.
- [ ] Decide whether `completed.md` regeneration should stay manual or become part of `docs:refresh`.
- [ ] Review `docs/tasks/bug-followup.md`: keep reusable structural bug patterns, migrate dated narratives to sessions, and fold active regression checks into `REF.2`/`REF.7`.

## P2

- [ ] Revalidate `LAUNCH.2.C` per-workspace quick actions after `REF.7`.
- [ ] Revalidate `ONBOARD.1.D/E` re-engage prompt and hotkey guidance after `REF.7`.
- [ ] Revalidate `NOTE.1` daily note driver after `REF.7`.
- [ ] Revalidate `UTIL.1.B-online` online currency rates after `REF.7` and ADR-038 status review.

## Frozen

- Do not expand or implement new product features before `REF.7` unless they are required by the active refactor or fix a P0 regression.
- `AGENT.3` and `AI.1` are superseded by the ADR-0029 / AI capability refactor track and should not be implemented as separate tracks.
- `AGENT.4`, `CLIP.1`, `SNIP.1`, `WIN.1`, `UTIL.3`, `DEV.1`, and `SYNC.1` remain parked until the refactor observation period is complete and their ADR gates are rechecked.
- Do not move detailed execution history back into `current.md`, `active.md`, `completed.md`, or this backlog.
- When a task group is completed, record detail in the relevant session log and let the completed index stay compact.
