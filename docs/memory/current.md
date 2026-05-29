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
- Move AI from product core to a stateless capability layer surfaced inline from the palette.
- Keep the docs system retrieval-first: startup state stays small, and detailed plans live in on-demand task files.

## Current Focus

- P0 is the ADR-0029 / AI capability refactor track from `docs/tasks/refactor-ai-capability.md`.
- User inserted a 2026-05-29 P0 stability override: `PREFLIGHT` now owns the startup preflight snapshot + once-per-boot refresh path so `/model_download` stops paying the cold path and stops flash-crashing on first open.
- Re-planned 2026-05-27 against `Keynova_Refactor_Plan_v1-2.docx`: `REF.6` is split into sub-batches `A` through `I` to recover the deferred Step 4 / Step 6 scope.
- `REF.6.C` landed at unit level on 2026-05-29: `gen_command` + `suggest_next` backend capabilities, structured TS parsers/hooks, and richer workflow-history labels for `cmd.run`, `capability.call`, and `action.run`.
- `REF.6.E` / `REF.6.F` landed later on 2026-05-29: the palette recognizes `next` / `cmd`, renders `CapabilityListCard` / `CapabilityCommandCard`, supports replay/edit/run interactions, and keeps Esc / keyboard navigation aligned with capability mode.
- Same-day UX follow-through removed the biggest friction point: empty palette now auto-mounts `next`, and no-result natural-language action queries auto-surface the `cmd` card after a short stabilization window.
- `REF.6.D` + `REF.6.J` landed at unit level on 2026-05-29: `fix <error>` prefix shares the answer card with explain/summarize, and a new rule-based `classifyNlIntent` extends the no-result auto-surface to `explain` / `summarize` / `fix` / `cmd` so common asks ("what is rust hashmap", "tldr this article", "list files in this project") no longer need an explicit prefix.
- `REF.6.G` / `REF.6.H` / `REF.6.I` closed 2026-05-29: G was a current-state audit (hot-path mounts + UI-owned confirm already satisfied by prior batches; unified `useActionConfirm` deferred); H relocated 21 files to `src/features/<feature>/` + `src/shared/components/` (model-manager tab consolidation deferred); I proposed `ADR-0040 ADR Template Slim` (status: 提議).
- REF.6 batch is closed at unit level. Outstanding work is `REF.7` quantitative gates + observation cycle (cannot complete in-session; requires release-cycle telemetry) and `REF.8` physical removal (blocked on REF.7 observation outcome).
- `current.md` and `active.md` are current-state indexes only.

## Important Constraints

- AI agents can draft ADRs as `proposed`; ADR-0039 must remain `proposed` in docs even though the developer explicitly approved the bounded startup-preflight runtime implementation on 2026-05-29.
- Freeze new feature work before `REF.7` unless it is required by the refactor, fixes a P0 regression, or protects a documented safety boundary.
- LLM-driven execution must stay approval-gated for risky or system-affecting actions; risk + `ConfirmRequirement` live on `UnifiedResult.ActionChip`.
- Generic shell tool exposure stays blocked until the platform sandbox boundary is complete.
- Background Core memory targets exclude active WebView, loaded local LLM model memory, PTY terminal sessions, monitoring streams, and index rebuild tasks.
- Three intentional deviations from docx literal text remain in force: `CommandPalette.tsx < 250` is dropped, `AiPanel.tsx` physical deletion is deferred to `REF.8`, and inline AI invocation uses capability cards in the palette body rather than row chips; entry is now mixed (`explain` / `summarize` / `fix` stay explicit prefixes, while `next` empty-state mount and `cmd` NL-query detection are enabled).

## Next Step

- Pending manual `tauri dev` smoke for `REF.6.B` / `REF.6.D` / `REF.6.E` / `REF.6.F` / `REF.6.J`: explicit prefix streaming, smart NL routing for explain/summarize/fix/cmd, empty-state `next`, Esc cancel/clear chain, and Bug A/B regression coverage. Plain `vite` browser smoke now confirms the new auto surfaces and keyboard routing, but capability IPC still requires `window.__TAURI_INTERNALS__`.
- `REF.6.C` verification now includes a live `gen_command` smoke pass against the locally available `qwen3:0.6b` model. `suggest_next` is covered by the full Rust suite and does not depend on an external model.
- Formal `qwen2.5:7b` P50/P95 still belongs to `REF.7`.
- Next coding work is `REF.7` setup (bench scripts + size gates + default-flip `ai.legacy_agent = false`) once the manual smoke pass completes; release-cycle observation gates close `REF.7`.
- `PREFLIGHT` follow-through still needs validation/docs cleanup on top of the runtime landing.
- Pre-existing Rust test failure on the base branch remains non-blocking: `handlers::builtin_cmd::tests::note_lazyvim_missing_nvim_returns_inline_guidance`.
- Run `npm run docs:refresh` before commit or handoff.
