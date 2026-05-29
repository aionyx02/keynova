---
type: task_index
status: active
priority: p0
updated: 2026-05-30
context_policy: always_retrievable
owner: project
tags: [refactor, ai-capability, search-first, p0]
---

# Active Tasks

## Active Queue

### P0

- [~] `PREFLIGHT` startup preflight snapshot + once-per-boot refresh for required bootstrap data so `/model_download` no longer pays the cold path or flash-crashes on first open. User-mandated P0 stability override before the remaining `REF.6` UI work; validation/docs follow-through still pending.
- [~] `BRAND.ICON` swap Tauri placeholder icon for brand assets. Explicit user-approved override.
- [~] `PERF.1.FU` idle-memory trim. Renderer lazy-loading plus the Windows native follow-up (low-memory-mode prewarm gating, shorter default Ollama keep-alive there, WebView2 GPU-off, hidden-window low-memory target) is in. Debug app-only re-measure is below goal; release/user smoke still pending.
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
  - [x] `REF.6.E` `next` prefix + `CapabilityListCard`. Auto-mounts on empty palette. Tauri smoke pending.
  - [x] `REF.6.F` `cmd <intent>` prefix + `CapabilityCommandCard`. No-result NL queries auto-surface. Tauri smoke pending.
  - [x] `REF.6.G` palette hot-path mounts + UI-owned confirm — current-state audit confirmed criteria already satisfied by prior batches. Unified `useActionConfirm` hook deferred.
  - [~] `REF.6.H` feature-first directory migration. 11 panels + 3 model panels relocated to `src/features/<feature>/`; 7 shared components moved to `src/shared/components/`. Model-manager tab consolidation deferred.
  - [x] `REF.6.I` ADR-0040 (proposed) template slimming to 4 sections (ADR-0030 slot was already taken by Backend Risk Tag Contract; reassigned to 0040).
  - [x] `REF.6.J` rule-based NL intent router (fallback). `classifyNlIntent` routes no-result NL queries to `explain` / `summarize` / `fix` / `cmd` so the user no longer needs the explicit prefix for common asks. Re-uses the existing stabilization debounce + dismissed-key gate; explicit prefixes still take priority.
- [~] `REF.7` quantitative gates, default `ai.legacy_agent = false`. Split: .A done, .B done, .C release notes+ADR measurement, .D user-action qwen2.5:7b bench.
- [ ] `REF.8` physical removal decision (`AiPanel` deletion, `agent_runtime` trim, flag removal).

Detailed batch definitions, done criteria, non-goals, file map, and validation gates live in `docs/tasks/refactor-ai-capability.md`.

### P1

- (empty; P0 refactor owns planning and execution priority)

### P2

- Only safety fixes or regressions that directly block the P0 refactor track.

## Strategy

Keynova's active priority is the search-first workflow refactor: AI is a stateless capability layer invoked inline from unified result rows and prefix-keyword palette flows; chat-first surfaces leave the hot path.

Until `REF.7` is complete, freeze new feature work unless it is required for the refactor, fixing a P0 regression, or protecting a safety boundary. `PREFLIGHT`, `BRAND.ICON`, and `PERF.1.FU` remain approved overrides.

Keep `active.md` compact. Put batch-level task detail in `docs/tasks/refactor-ai-capability.md`, detailed implementation notes in `docs/memory/sessions/YYYY-MM-DD.md`, and future non-refactor ideas in `docs/tasks/backlog.md`.

## Next Phase Candidates

- Verify the new idle-memory behavior in `tauri dev`, then decide whether a release-build measurement is enough to close `PERF.1.FU`.
- Finish the pending `tauri dev` smoke for REF.6, then close `REF.7.C/.D`.
- After `REF.7`, revalidate parked tracks from `docs/tasks/backlog.md`.
- After `REF.8`, refresh affected ADR statuses and architecture docs.
