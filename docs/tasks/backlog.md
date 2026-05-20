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

Backlog is on-demand planning context. Keep P0/P1 specific, keep P2 one-line, and avoid speculative P3+ expansion until a track is selected.

## P0

- [ ] Tune docs guard thresholds if normal edits are blocked too aggressively.
- [ ] Decide whether `completed.md` regeneration should stay manual or become part of `docs:refresh`.
- [ ] Review `docs/tasks/bug-followup.md`: keep structural bug patterns, migrate dated narratives to sessions.

## P1

- [ ] `LAUNCH.2.C` per-workspace quick actions: metadata schema, quick-action UI, and workspace-specific overrides.
- [ ] `ONBOARD.1.D` re-engage prompt: lightweight usage tracking and delayed prompt rules.
- [ ] `ONBOARD.1.E` first-run hotkey guidance: conflict detection and fallback copy.
- [ ] `NOTE.1` daily note driver: today/yesterday commands, templates, backlinks, tags, and launcher note search.
- [ ] `UTIL.1.B-online` online currency rates after ADR-038 acceptance.

## P2

- `AGENT.3` tool surface rebalance after ADR-029.
- `AGENT.4` context awareness after ADR-030.
- `CLIP.1` clipboard history after ADR-031.
- `SNIP.1` text expansion after ADR-032.
- `WIN.1` window switcher after ADR-033.
- `UTIL.3` reminders and timers after ADR-034.
- `DEV.1` external provider utilities after ADR-035.
- `SYNC.1` git-backed sync after ADR-036.
- `AI.1` inline AI surfaces after ADR-037.

## Frozen

- Do not expand speculative feature breakdowns during the docs governance/refactor track.
- Do not move detailed execution history back into `current.md`, `active.md`, `completed.md`, or this backlog.
- When a task group is completed, record detail in the relevant session log and let the completed index stay compact.
