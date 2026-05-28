---
type: task_index
status: active
priority: p0
updated: 2026-05-28
context_policy: always_retrievable
owner: project
tags: [refactor, ai-capability, search-first, p0]
---

# Active Tasks

## Active Queue

### P0

- [~] `BRAND.ICON` swap Tauri placeholder icon for `keynova_icon.png` brand across desktop surfaces (tauri icon-generated set, in-panel logo + wordmark, tray About/Version, NSIS sidebar/header BMPs). User-mandated polish — explicit override of pre-REF.7 freeze. Branch `feature/brand-icon`.
- [x] `REF.0` lock ADR-0029 as the governing decision for the AI capability refactor.
- [x] `REF.1` define `UnifiedResult` as the shared result/action contract.
- [x] `REF.2` split `CommandPalette.tsx` (landed 598 lines; `< 250` dropped — see plan).
- [x] `REF.3` split `handlers/agent/mod.rs` (landed 616 lines, observation target `< 600`).
- [~] `REF.4` stateless AI capability layer — 3/5 capabilities live (`explain`, `summarize`, `fix_error`). `gen_command` + `suggest_next` deferred to `REF.6.C` so they ship with their UI scenes.
- [x] `REF.5` workflow memory schema v4 + `record`/`suggest`.
- [ ] `REF.6` search box = pure dispatcher. Sub-batches:
  - [x] `REF.6.A` inline AI MVP wire format (UnifiedResult + capability stream); per-row chip + `Ctrl+E` ripped out in REF.6.B.
  - [x] `REF.6.B` prefix dispatcher + `explain` / `summarize` end-to-end (`CapabilityAnswerCard`, `parseCapabilityPrefix`, `usePaletteMode`, `useCapabilityStream`, hint line). Unit tests green; manual `tauri dev` smoke pending.
  - [ ] `REF.6.C` backend `gen_command` + `suggest_next` capabilities + IPC + hooks (no UI).
  - [ ] `REF.6.D` `fix <error>` prefix wired to `CapabilityAnswerCard`.
  - [ ] `REF.6.E` `next` prefix + `CapabilityListCard` for `suggest_next`.
  - [ ] `REF.6.F` `cmd <intent>` prefix + `CapabilityCommandCard` for `gen_command`.
  - [ ] `REF.6.G` remove `AiPanel` / `TerminalPanel` mounts from palette hot path; UI-owned `ConfirmRequirement`.
  - [ ] `REF.6.H` feature-first directory migration for remaining panels + `src/shared/*` + model-manager consolidation (docx §3.5/§6.1).
  - [ ] `REF.6.I` ADR-0030 (proposed) — ADR template slimming to 4 sections (docx §9.2).
- [ ] `REF.7` quantitative gates, default `ai.legacy_agent = false`, observe one release cycle.
- [ ] `REF.8` physical removal decision (AiPanel deletion, agent_runtime trim, flag removal).

Detailed batch definitions, done criteria, non-goals, file map, and validation gates live in `docs/tasks/refactor-ai-capability.md`.

### P1

- (empty; P0 refactor owns planning and execution priority)

### P2

- Only safety fixes or regressions that directly block the P0 refactor track.

## Strategy

Keynova's active priority is search-first workflow refactor: AI is a stateless capability layer invoked inline from unified result rows; AiPanel and chat-first surfaces leave the hot path.

Until `REF.7` is complete, freeze new feature work unless required for the refactor, fixing a P0 regression, or protecting a documented safety boundary.

Keep `active.md` compact. Put batch-level task detail in `docs/tasks/refactor-ai-capability.md`, detailed implementation notes in `docs/memory/sessions/YYYY-MM-DD.md`, and future non-refactor ideas in `docs/tasks/backlog.md`.

## Next Phase Candidates

- After `REF.7`, revalidate parked feature tracks from `docs/tasks/backlog.md`.
- After `REF.8`, refresh affected ADR statuses and architecture docs.