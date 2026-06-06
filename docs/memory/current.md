---
type: working_memory
status: active
priority: p0
updated: 2026-06-06
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
- `PERF.1.FU` closed: real unique footprint ~`80 MB` (Private-WS), under the `200 MB` goal; the ~`324 MB` process-tree figure is shared Edge/Chromium DLL pages, not keynova's. Landed `[profile.release]` (strip+LTO) + host `EmptyWorkingSet` on hide. Detail in `sessions/2026-05-30.md`.
- `agent_runtime.rs < 400` REF.7 criterion is dropped because that file does not exist in repo. Documented in the REF.7 plan as the same class of docx-vs-repo divergence as the `CommandPalette.tsx < 250` deviation.
- `current.md` and `active.md` are current-state indexes only. REF.6 batch detail lives in `docs/memory/sessions/2026-05-29.md`.
- Coding-style cleanup now has `docs/coding-style.md`, `.editorconfig`, and a
  no-behavior formatting/comment pass across source files.

## Important Constraints

- AI agents can draft ADRs as `proposed`; ADR-0039 must remain `proposed` in docs even though the developer explicitly approved the bounded startup-preflight runtime implementation on 2026-05-29.
- Freeze lifted (2026-06-06, developer-directed): `PRODUCT.1` and `PRODUCT.2` are
  **unfrozen** — REF.7.D done + latency direction finalized (tiered §8 + 1.5b
  default) satisfied the PRODUCT.2 gate. `ai.legacy_agent` observation-window items
  stay separate, non-blocking. Other parked tracks (`AGENT.*`, `CLIP.1`) stay frozen.
- LLM-driven execution must stay approval-gated for risky or system-affecting actions; risk + `ConfirmRequirement` live on `UnifiedResult.ActionChip`.
- Generic shell tool exposure stays blocked until the platform sandbox boundary is complete.
- Background Core memory targets exclude active WebView, loaded local LLM model memory, PTY terminal sessions, monitoring streams, and index rebuild tasks.
- Three intentional deviations from docx literal text remain in force: `CommandPalette.tsx < 250` is dropped, `AiPanel.tsx` physical deletion is deferred to `REF.8`, and inline AI invocation uses capability cards in the palette body rather than row chips; entry is now mixed (`explain` / `summarize` / `fix` stay explicit prefixes, while `next` empty-state mount and `cmd` NL-query detection are enabled).

## Next Step

- `PRODUCT.1` search-core is unfrozen. `1.A`–`1.D` + `PROJECT_ROOT.wire` + `1.F`
  preview polish + `1.G` slashless inline utilities + `1.H` unit/docs keyboard
  gate are done; `1.E` command execution / terminal handoff is intentionally
  dropped for now. Next is the `PRODUCT.1.H` manual dogfood pass with a real
  `Ctrl+K` input-ready timing sample.
- REF.7.D **done** (2026-06-06): bench proved CPU throughput (not model size) is the
  wall — even 1.5b ~5× over old 800 ms. Developer-approved: ADR-0029 §8 → **tiered**
  (CPU-host P50<5s/P95<8s + GPU<800ms aspirational) + **`qwen2.5:1.5b` reference
  default** (ai.rs/live_tests/bench/Cargo.toml); 1.5b P50 4266/P95 6269 **PASS** CPU
  tier. ADR stays `accepted`. `PRODUCT.2` unfreeze gate now **met**, awaiting explicit
  flip; REF.7.C closer = user Bug A/B smoke. Detail: `sessions/2026-06-06.md`.
- `v0.6.0` release commit bumps app metadata and adds release notes for the
  PRODUCT.1 workflow-core pass. Pushing tag `v0.6.0` triggers GitHub release CI
  and creates a draft release. Still UNSIGNED. `v0.5.0` remains the previous
  stable tag. Detail in `sessions/2026-06-05.md`.
- Run `npm run docs:refresh` before commit or handoff.
