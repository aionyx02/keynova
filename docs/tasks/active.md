---
type: task_index
status: active
priority: p0
updated: 2026-05-23
context_policy: always_retrievable
owner: project
tags: [refactor, ai-capability, search-first, p0]
---

# Active Tasks

## Active Queue

### P0

- [x] `REF.0` lock ADR-0029 as the governing decision for the AI capability refactor.
- [x] `REF.1` define `UnifiedResult` as the shared result/action contract.
- [x] `REF.2` split `CommandPalette.tsx` into feature-first hooks/components and regression-check Bug A/B paths.
- [x] `REF.3` split `handlers/agent/mod.rs` so lifecycle, local context, tool dispatch, and dev runner stop living in one module.
- [ ] `REF.4` add the stateless AI capability layer for `explain`, `summarize`, and `fix_error`.
- [ ] `REF.5` add workflow memory as a P0 differentiator, in parallel with `REF.4` after the schema boundary is clear.
- [ ] `REF.6` switch the palette result list to consume `UnifiedResult` and remove embedded non-core surfaces from the hot path.
- [ ] `REF.7` add quantitative gates, default `ai.legacy_agent = false`, and observe one release cycle.
- [ ] `REF.8` after the observation window, physically remove deprecated agent/chat code and legacy flags.

Detailed batch definitions, done criteria, non-goals, file map, and validation gates live in `docs/tasks/refactor-ai-capability.md`.

### P1

- (empty; P0 refactor owns planning and execution priority)

### P2

- Only safety fixes or regressions that directly block the P0 refactor track.

## Strategy

Keynova's active priority is now search-first workflow refactor: AI moves from product core to stateless capability layer, and unified search/result handling becomes the product spine.

Until `REF.7` is complete, freeze new feature work unless it is required for the refactor, fixes a P0 regression, or protects a documented safety boundary.

Keep `active.md` compact. Put batch-level task detail in `docs/tasks/refactor-ai-capability.md`, detailed implementation notes in `docs/memory/sessions/YYYY-MM-DD.md`, and future non-refactor ideas in `docs/tasks/backlog.md`.

## Next Phase Candidates

- After `REF.7`, revalidate parked feature tracks from `docs/tasks/backlog.md`.
- After `REF.8`, refresh affected ADR statuses and architecture docs.
