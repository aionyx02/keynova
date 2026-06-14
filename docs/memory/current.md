---
type: working_memory
status: active
priority: p0
updated: 2026-06-14
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
- `PRODUCT.2` completed on `feature/product-2-ai-capabilities` (2026-06-06):
  hardened all five capabilities while keeping generated commands copy-only.
- REF.7.A/.B + `BRAND.ICON` + `PREFLIGHT` are merged to `main` and shipped as tag `v0.3.0`. `dev` is strictly behind `main`. Active close-out work is on `feature/ref-7-closeout` (forked from `main`, since REF.7/v0.3.0 do not exist on `dev`). REF.7.C release notes shipped as `docs/release-notes/v0.3.0.md`; ADR-0029 §10 measurement is scaffolded and `pending REF.7.D`.
- `REF.8` + `CI.HARDEN` merged to `main` (`--no-ff` `8e49dd1`, 2026-06-10): legacy agent + nvim/mouse_control removal, post-cleanup, and Dependabot/CodeQL/branch-protection runbook landed together. `main` is 6 ahead of `origin/main` — **push + branch protection pending developer** (`gh` unauth). Detail: `sessions/2026-06-10.md`.
- `PERF.1.FU` closed: real unique footprint ~`80 MB` (Private-WS), under the `200 MB` goal; the ~`324 MB` process-tree figure is shared Edge/Chromium DLL pages, not keynova's. Landed `[profile.release]` (strip+LTO) + host `EmptyWorkingSet` on hide. Detail in `sessions/2026-05-30.md`.
- The `agent_runtime.rs < 400` REF.7 size criterion is moot: REF.8 deleted the file outright (legacy agent removed, ADR-0029).
- `current.md` and `active.md` are current-state indexes only. REF.6 batch detail lives in `docs/memory/sessions/2026-05-29.md`.
- Coding-style cleanup now has `docs/coding-style.md`, `.editorconfig`, and a
  no-behavior formatting/comment pass across source files.

## Important Constraints

- AI agents can draft ADRs as `proposed`; ADR-0039 must remain `proposed` in docs even though the developer explicitly approved the bounded startup-preflight runtime implementation on 2026-05-29.
- `PRODUCT.1` and `PRODUCT.2` are complete. The legacy ReAct agent was fully
  removed in REF.8 (2026-06-09; ADR-0029 supersedes it) — no `ai.legacy_agent`
  flag or `agent_runtime`/`handlers/agent` remain. Parked tracks (`AGENT.*`,
  `CLIP.1`) stay frozen.
- LLM-driven execution must stay approval-gated for risky or system-affecting actions; risk + `ConfirmRequirement` live on `UnifiedResult.ActionChip`.
- Generic shell tool exposure stays blocked until the platform sandbox boundary is complete.
- Background Core memory targets exclude active WebView, loaded local LLM model memory, PTY terminal sessions, monitoring streams, and index rebuild tasks.
- Windows `Command` spawns must chain `.no_window()` (`core::SilentCommandExt` = `CREATE_NO_WINDOW`) or a console flashes, stealing focus. `sessions/2026-06-13.md`
- Two intentional deviations from docx literal text remain in force: `CommandPalette.tsx < 250` is dropped, and inline AI invocation uses capability cards in the palette body rather than row chips; entry is now mixed (`explain` / `summarize` / `fix` stay explicit prefixes, while `next` empty-state mount and `cmd` NL-query detection are enabled). (`AiPanel.tsx` deletion + legacy-agent removal landed in REF.8, 2026-06-09.)

## Next Step

- `v0.7.1` cut (2026-06-14): stability/perf/CI patch bundling the post-`v0.7.0`
  delta — Windows console-flash fix (`SilentCommandExt`), native shell-icon perf,
  hardware-probe memoize, per-push cross-platform CI matrix. Version → `0.7.1`
  (package.json/tauri.conf.json/Cargo.toml/Cargo.lock); notes
  `release-notes/v0.7.1.md`. Merged `feature/cross-platform-stability` → `main`
  (`--no-ff`), tagged `v0.7.1`; `release.yml` builds the 3-OS **draft** (still
  **UNSIGNED**, certs deferred per dev) to publish. Detail: `sessions/2026-06-14.md`.
- Cross-platform stability Phase 1 (CI gate + static review, no panics) done and
  CI-green on all 3 OSes; Phase 2 (fill `platform/linux.rs`/`macos.rs` skeletons)
  deferred. Detail: `tasks/cross-platform-stability.md`.
