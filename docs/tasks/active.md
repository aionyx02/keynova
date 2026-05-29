---
type: task_index
status: active
priority: p0
updated: 2026-05-29
context_policy: always_retrievable
owner: project
tags: [refactor, ai-capability, search-first, p0]
---

# Active Tasks

## Active Queue

### P0

- [~] `PREFLIGHT` startup preflight snapshot + once-per-boot refresh for required bootstrap data so `/model_download` no longer pays the cold path or flash-crashes on first open. User-mandated P0 stability override before the remaining `REF.6` UI work; validation/docs follow-through still pending.
- [~] `BRAND.ICON` swap Tauri placeholder icon for `keynova_icon.png` brand across desktop surfaces. Explicit user override of the pre-`REF.7` freeze; branch `feature/brand-icon`.
- [x] `REF.0` lock ADR-0029 as the governing decision for the AI capability refactor.
- [x] `REF.1` define `UnifiedResult` as the shared result/action contract.
- [x] `REF.2` split `CommandPalette.tsx` (landed 598 lines; `< 250` dropped, see plan).
- [x] `REF.3` split `handlers/agent/mod.rs` (landed 616 lines, observation target `< 600`).
- [x] `REF.4` stateless AI capability layer. All 5 capabilities are now live behind one `call_capability` entry: `explain`, `summarize`, `fix_error`, `gen_command`, `suggest_next`.
- [x] `REF.5` workflow memory schema v4 + `record` / `suggest`.
- [~] `REF.6` search box = pure dispatcher. Sub-batches:
  - [x] `REF.6.A` inline AI MVP wire format (UnifiedResult + capability stream); per-row chip + `Ctrl+E` ripped out in `REF.6.B`.
  - [x] `REF.6.B` prefix dispatcher + `explain` / `summarize` end-to-end (`CapabilityAnswerCard`, `parseCapabilityPrefix`, `usePaletteMode`, `useCapabilityStream`, hint line). Unit tests green; manual `tauri dev` smoke pending.
  - [x] `REF.6.C` backend `gen_command` + `suggest_next` capabilities + IPC + hooks. Unit tests green; `gen_command` live smoke passed on the local `qwen3:0.6b` model.
  - [x] `REF.6.D` `fix <error>` prefix wired to `CapabilityAnswerCard`. Backend payload remap (`raw_output` vs `text`) lives in `useCapabilityStream`; UI shares the existing answer card with a new `fix` label. Final manual validation still needs `npm run tauri dev`.
  - [x] `REF.6.E` `next` prefix + `CapabilityListCard` for `suggest_next`. Prefix wiring landed first; 2026-05-29 follow-through now auto-mounts the same card on empty palette so `next` is visible without typing the keyword. Final manual validation still needs `npm run tauri dev` because capability IPC requires Tauri runtime.
  - [x] `REF.6.F` `cmd <intent>` prefix + `CapabilityCommandCard` for `gen_command`. Prefix wiring landed first; 2026-05-29 follow-through now auto-surfaces the same card for no-result natural-language action queries, so Enter can generate without typing `cmd`. Final manual validation still needs `npm run tauri dev` because capability IPC requires Tauri runtime.
  - [ ] `REF.6.G` remove `AiPanel` / `TerminalPanel` mounts from the palette hot path; UI-owned `ConfirmRequirement`.
  - [ ] `REF.6.H` feature-first directory migration for remaining panels + `src/shared/*` + model-manager consolidation.
  - [ ] `REF.6.I` ADR-0030 (proposed) template slimming to 4 sections.
  - [x] `REF.6.J` rule-based NL intent router (fallback). `classifyNlIntent` routes no-result NL queries to `explain` / `summarize` / `fix` / `cmd` so the user no longer needs the explicit prefix for common asks. Re-uses the existing stabilization debounce + dismissed-key gate; explicit prefixes still take priority.
- [ ] `REF.7` quantitative gates, default `ai.legacy_agent = false`, observe one release cycle.
- [ ] `REF.8` physical removal decision (`AiPanel` deletion, `agent_runtime` trim, flag removal).

Detailed batch definitions, done criteria, non-goals, file map, and validation gates live in `docs/tasks/refactor-ai-capability.md`.

### P1

- (empty; P0 refactor owns planning and execution priority)

### P2

- Only safety fixes or regressions that directly block the P0 refactor track.

## Strategy

Keynova's active priority is the search-first workflow refactor: AI is a stateless capability layer invoked inline from unified result rows and prefix-keyword palette flows; chat-first surfaces leave the hot path.

Until `REF.7` is complete, freeze new feature work unless it is required for the refactor, fixing a P0 regression, or protecting a documented safety boundary. `PREFLIGHT` and `BRAND.ICON` remain explicit user-approved overrides.

Keep `active.md` compact. Put batch-level task detail in `docs/tasks/refactor-ai-capability.md`, detailed implementation notes in `docs/memory/sessions/YYYY-MM-DD.md`, and future non-refactor ideas in `docs/tasks/backlog.md`.

## Next Phase Candidates

- After `REF.6.D` / `.E` / `.F` / `.J`, finish the pending smoke/validation pass before `REF.6.G`.
- After `REF.7`, revalidate parked feature tracks from `docs/tasks/backlog.md`.
- After `REF.8`, refresh affected ADR statuses and architecture docs.
