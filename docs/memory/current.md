---
type: working_memory
status: active
priority: p0
updated: 2026-05-30
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
- `PREFLIGHT` startup preflight snapshot + once-per-boot refresh shipped on `main` (`8c1a43b`) and in v0.3.0; `/model_download` cold-path / first-open crash resolved.
- REF.6 closed at unit level 2026-05-29 (merged into `dev` as `31786eb`). All 5 capabilities live (`explain` / `summarize` / `fix_error` / `gen_command` / `suggest_next`); palette is a prefix + NL-fallback dispatcher; src layout migrated to feature-first + `src/shared/`; ADR-0040 (slim template) proposed.
- REF.7.A/.B + `BRAND.ICON` + `PREFLIGHT` are merged to `main` and shipped as tag `v0.3.0`. `dev` is strictly behind `main`. Active close-out work is on `feature/ref-7-closeout` (forked from `main`, since REF.7/v0.3.0 do not exist on `dev`). REF.7.C release notes shipped as `docs/release-notes/v0.3.0.md`; ADR-0029 §10 measurement is scaffolded and `pending REF.7.D`.
- User-directed memory follow-up is active on this branch. The pass started with eager WebView2 renderer JS/DOM trimming, then moved into Windows-native WebView2 memory controls once bundle wins flattened out.
- Latest Windows `tauri-app.exe` re-check still meets the user's `200 MB` goal on debug: cold hidden launch measured about `63.7 MB WS / 124.7 MB PM`, `keynova start` returned in about `10-13 ms`, and post-wake samples stayed around `93-106 MB WS / 124.4-124.9 MB PM`.
- A direct `cargo build --release` binary pass did not reproduce the same WS profile: PM stayed around `124-129 MB`, but WS stayed much higher (`273.6 MB` hidden after a warm hide and `334.7 MB` active), so the release baseline is still unresolved.
- `agent_runtime.rs < 400` REF.7 criterion is dropped because that file does not exist in repo. Documented in the REF.7 plan as the same class of docx-vs-repo divergence as the `CommandPalette.tsx < 250` deviation.
- `current.md` and `active.md` are current-state indexes only. REF.6 batch detail lives in `docs/memory/sessions/2026-05-29.md`.

## Important Constraints

- AI agents can draft ADRs as `proposed`; ADR-0039 must remain `proposed` in docs even though the developer explicitly approved the bounded startup-preflight runtime implementation on 2026-05-29.
- Freeze new feature work before `REF.7` unless it is required by the refactor, fixes a P0 regression, or protects a documented safety boundary.
- LLM-driven execution must stay approval-gated for risky or system-affecting actions; risk + `ConfirmRequirement` live on `UnifiedResult.ActionChip`.
- Generic shell tool exposure stays blocked until the platform sandbox boundary is complete.
- Background Core memory targets exclude active WebView, loaded local LLM model memory, PTY terminal sessions, monitoring streams, and index rebuild tasks.
- Three intentional deviations from docx literal text remain in force: `CommandPalette.tsx < 250` is dropped, `AiPanel.tsx` physical deletion is deferred to `REF.8`, and inline AI invocation uses capability cards in the palette body rather than row chips; entry is now mixed (`explain` / `summarize` / `fix` stay explicit prefixes, while `next` empty-state mount and `cmd` NL-query detection are enabled).

## Next Step

- Investigate why the direct `target/release/tauri-app.exe` path still shows `~274-335 MB WS` while PM stays near the debug reading before promoting a release baseline or adding more WebView2 flags.
- REF.7.D is still user-action: `ollama pull qwen2.5:7b && npm run bench:ai -- --runs 10 --model qwen2.5:7b`. Only `qwen3:0.6b` is local.
- REF.7.C closes by filling ADR-0029 §10 from the REF.7.D output (one follow-up edit) + user-side Bug A/B smoke.
- REF.8 stays gated on the observation window (1 minor release tag + 14 days, no P0 regression).
- Run `npm run docs:refresh` before commit or handoff.
