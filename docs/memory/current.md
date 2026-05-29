---
type: working_memory
status: active
priority: p0
updated: 2026-05-29
context_policy: always_retrievable
owner: project
---

# Current Project Memory

## Current Strategy

- Treat Keynova as a keyboard-first workflow tool, not an AI chat product.
- Make unified search/result handling the product spine.
- Move AI from product core to a stateless capability layer exposed through inline action chips.
- Keep the docs system retrieval-first: startup state stays small, and detailed plans live in on-demand task files.

## Current Focus

- P0 is the ADR-0029 / AI capability refactor track from `docs/tasks/refactor-ai-capability.md`.
- User inserted a P0 stability override on 2026-05-29: `PREFLIGHT` now implements a startup preflight snapshot with once-per-boot refresh so `/model_download` stops paying the cold path and stops flash-crashing on first open.
- Re-planned 2026-05-27 against `Keynova_Refactor_Plan_v1-2.docx`: REF.6 is split into sub-batches B–I to recover deferred docx Step 4/6 scope (2 missing capabilities, 3 missing UI scenes, feature-first migration, ADR-0030 slim template).
- Execution order is `REF.0` → `REF.6.A` (done) → `REF.6.B` → ... → `REF.6.I` → `REF.7` → `REF.8`.
- `current.md` and `active.md` are current-state indexes only.

## Important Constraints

- AI agents can draft ADRs as `proposed`; ADR-0039 must remain `proposed` in docs even though the developer explicitly approved the bounded startup-preflight runtime implementation on 2026-05-29.
- Freeze new feature work before `REF.7` unless it is required by the refactor, fixes a P0 regression, or protects a documented safety boundary.
- LLM-driven execution must stay approval-gated for risky or system-affecting actions; risk + `ConfirmRequirement` live on `UnifiedResult.ActionChip`.
- Generic shell tool exposure stays blocked until the platform sandbox boundary is complete.
- Background Core memory targets exclude active WebView, loaded local LLM model memory, PTY terminal sessions, monitoring streams, and index rebuild tasks.
- Three intentional deviations from docx literal text (documented in plan): (1) `CommandPalette.tsx < 250` dropped — current 598 lines accepted; (2) `AiPanel.tsx` deletion split into REF.6.G (unmount from hot path) + REF.8 (physical deletion decision); (3) inline AI invocation switched from per-row chips + auto-detect to **prefix-keyword** pattern (`explain <q>` / `cmd <intent>` / `fix <error>` / `summarize <text>` / `next`) — see [[feedback-inline-ai-prefix]].

## Next Step

- Immediate execution order: finish `PREFLIGHT` validation/docs follow-through and cold-path smoke coverage, then resume `REF.6.C` backend `gen_command` + `suggest_next`.
- `REF.6.B` landed at unit level: prefix dispatcher (`explain` / `summarize`) + `CapabilityAnswerCard` + `[Copy md]` / `[Save to note]` chips + capability-cancel Esc branch + discovery hint line (`launcher.show_capability_hint`). REF.6.A row chip + `Ctrl+E` + `InlineCapabilityReply` removed. Post-handoff `explain <text>` UI event alias/final-response fallback regression fixed; targeted vitest pass.
- Pending manual `tauri dev` smoke for REF.6.B (live Ollama stream end-to-end + Bug A/B regression).
- Next batch: `REF.6.C` backend `gen_command` + `suggest_next` capabilities (no UI surface in that batch; UI consumers are REF.6.D/.E/.F).
- Housekeeping pass landed without changing refactor order: Vite starter shell assets are gone, ignore rules are tighter, and loose root `.docx` references now live under `docs/`.
- Follow-up housekeeping keeps the repo root slimmer: `src/assets/keynova_icon.png` is now the canonical brand source, `tauri:icon` points at it, and local cleanup can run via `npm run clean:local`.
- Live Ollama smoke for `explain` + `fix_error` passed on `qwen2.5:0.5b` cold-start (4.6–5.0 s). Formal `qwen2.5:7b` P50/P95 reading still owed; runs in REF.7.
- Pre-existing Rust test failure on base: `handlers::builtin_cmd::tests::note_lazyvim_missing_nvim_returns_inline_guidance` — investigate separately, not blocking REF.6.B.
- Run `npm run docs:refresh` before commit or handoff.
