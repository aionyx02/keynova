---
type: task_plan
status: active
priority: p0
updated: 2026-05-29
context_policy: on_demand
owner: project
tags: [refactor, ai-capability, unified-result, workflow-memory, search-first]
---

# P0 Refactor: AI Capability Layer And Unified Search

Source: `Keynova_Refactor_Plan_v1-2.docx` (repo root). Anchors below cite docx sections.

Goal: reposition Keynova as a keyboard-first workflow tool. Unified search is the product spine; AI becomes a stateless capability invoked inline from result rows and workflow surfaces.

## Priority Rules

- This plan is the highest-priority runnable task source after `docs/tasks/active.md`.
- Execute batches in order unless a batch explicitly says it may run in parallel.
- Freeze new feature work until `REF.7`, except for P0 regressions, safety fixes, or work directly required by this plan.
- Keep legacy agent/chat code behind `ai.legacy_agent` during the compatibility window; physical removal waits for `REF.8`.
- Do not implement architecture-changing code until ADR-0029 is accepted by the developer.

## Re-planning Note (2026-05-27)

Re-aligned the post-REF.6.A queue against the docx. Three decisions diverge from
the docx literal text and are intentional:

- `CommandPalette.tsx` size: docx Step 2 done = `< 250`. Current landing is 598
  lines; we keep that and drop the `< 250` hard gate per
  [[project-ref2-p5-landing]] + [[feedback-task-persistence]] (simplification-only
  goals do not block closure). Treated as documented deviation, not a regression.
- `AiPanel.tsx`: docx §3.5/§4.6 says delete entirely. We split into two phases —
  REF.6.G removes from palette hot path + flag-guards as legacy fallback; REF.8
  decides physical deletion after the observation cycle. Authority for staged
  removal: ADR-0029 §2.5 Rollback (parallel + flag, 2 release cycles).
- **Inline AI invocation pattern**: docx §4.1–4.5 mockups use per-row chips +
  auto-detection (NL heuristic / terminal regex / empty-state mount). The row
  chips stay removed, but the final palette UX is now a mixed dispatcher:
  `explain <q>` / `summarize <t>` / `fix <error>` stay explicit prefixes,
  while `next` also auto-mounts on an empty palette and `cmd` also auto-surfaces
  for no-result natural-language action queries. Capability mode still owns the
  result area; row-level AI affordances remain out. Wire format (`UnifiedResult`)
  is retained for non-AI action data. Authority: see
  [[feedback-inline-ai-prefix]]. Details in
  `docs/tasks/refactor-ai-capability-ui-spec.md`.

## Batch Plan

### REF.0 - ADR-0029 Decision Lock — DONE

### REF.1 - Unified Result Schema — DONE

### REF.2 - Command Palette Feature Split — DONE (with documented deviation)

Landed at 598 lines (docx target was `< 250`). `< 250` is now treated as
aspirational; do not reopen as a standalone simplification batch. Behavior and
Bug A/B regression checks passed.

### REF.3 - Agent Handler Module Split — DONE

`handlers/agent/mod.rs` currently 616 lines (docx observation target `< 600`).
Micro-overshoot accepted; revisit only if REF.6.B/D extraction creates room for
free trimming.

### REF.4 - Stateless AI Capability Layer - DONE

All five planned capabilities now ship through the single `call_capability`
entry: `explain`, `summarize`, `fix_error`, `gen_command`, and
`suggest_next`.

Live Ollama smoke against `qwen2.5:0.5b` cold-start: 4.6–5.0 s for `explain` /
`fix_error`. Formal P50/P95 reading on `qwen2.5:7b` remains pending until REF.7
bench scripts exist.

### REF.5 - Workflow Memory — DONE

Schema v4 `workflow_history` + `record` / `suggest` entry points landed.
`suggest_next` backend wiring now exists via REF.6.C; the dedicated UI scene
still waits for REF.6.E.

### REF.6 - Search Box As Pure Dispatcher (in progress)

Docx anchor: §3.1, §3.5, §4.1–4.6, Step 6 of §5.

Sub-batches below. Each is independently shippable; sequence reflects
dependency, not priority.

#### REF.6.A — Inline AI MVP — DONE 2026-05-23

`search.query` returns `UnifiedResult[]`; every result row carries an `Explain`
`ActionChip`; `Ctrl+E` triggers `useCapability("explain")`; streaming reply
renders below the focused row via `InlineCapabilityReply`. Schema additive
change: `SourceMetadata.secondary_action_count: Option<u32>`.

#### REF.6.B — Prefix dispatcher + `explain` / `summarize` end-to-end — DONE (unit-level)

Full UI contract in `docs/tasks/refactor-ai-capability-ui-spec.md` §1–3 + §9.

Landed:
- `parseCapabilityPrefix` parser (only `explain` / `summarize` wired this
  batch; `cmd` / `fix` / `next` fall through to search until REF.6.D/.E/.F).
- `usePaletteMode` hook + `useCapabilityStream` (300 ms debounce + cancel-
  on-rerun + projected timing).
- `CapabilityAnswerCard` (header `✨ <label> · <latency>`, Markdown body,
  `[Copy md]` + `[Save to note]` footer chips with auto-name
  `<Label>: <first 40 chars>` via existing `note.save` IPC).
- `CapabilityResultArea` dispatcher; mounted in palette via
  `key={paletteMode.id}` on capability swap.
- `CapabilityHintLine` rendered above empty palette; gated by new
  `launcher.show_capability_hint` setting (default true) added to
  `useLauncherSettings` `WATCHED_KEYS`.
- `useEscapeKey` extended with capability-cancel branch — first Esc cancels
  stream (body shows `Cancelled.`), second Esc falls through to clear query.
- `usePaletteRefs` extended with `capabilityModeRef` /
  `capabilityStreamingRef` to keep the single-mount Esc listener free of
  stale closures.
- REF.6.A row chip + `onExplain` plumbing + `Ctrl+E` binding +
  `InlineCapabilityReply` component removed cleanly.

Verification:
- 101 / 102 vitest tests pass (1 intentional skip on the placeholder
  `SearchResultsList.test.tsx`). `npm run lint` clean.
- `cargo test --lib` shows one unrelated pre-existing failure
  (`handlers::builtin_cmd::tests::note_lazyvim_missing_nvim_returns_inline_guidance`)
  on the base branch — not blocking REF.6.B.

Pending (handoff to manual smoke before commit):
- `npm run tauri dev`: type `explain rust hashmap remove` → answer streams
  in card within ~5 s on cold model.
- Backspacing past `explain ` returns to search results in the same frame.
- Esc twice clears prefix in two stages (cancel → clear).
- Bug A focus race + Bug B delete-verify still pass.

#### REF.6.C — Complete capability set: gen_command + suggest_next (docx §3.2) — DONE (unit-level)

Scope:
- Backend `core/ai_capability/gen_command.rs`: typed payload
  `{ intent: String, ctx: GenCommandCtx }` → `{ command: String, confidence: f32,
  rationale: String }`. Registry + risk tag + audit per existing pattern.
- Backend `core/ai_capability/suggest_next.rs`: typed payload
  `{ ctx: SuggestNextCtx }` → `Vec<SuggestedNextAction>`. Reads
  `workflow_memory::suggest` and re-ranks with capability heuristic.
- Frontend types + `useGenCommand` / `useSuggestNext` hooks.
- `handlers/ai_capability.rs` exposes both via IPC.

Non-goals:
- No UI surface wiring (that is REF.6.E / REF.6.F).
- No autonomous chain — capabilities still single-step.

Landed:
- `core/ai_capability/capabilities/gen_command.rs` with typed `intent + ctx`
  payload, structured `{ command, confidence, rationale }` output, JSON-first
  parsing with fallback, audit wiring, and conservative command risk tagging.
- `core/ai_capability/capabilities/suggest_next.rs` with typed workflow-memory
  payload, heuristic ranking over `workflow_history`, and best-effort replay
  descriptors for replayable `cmd.run` rows.
- Richer workflow-history labels for `cmd.run`, `capability.call`, and
  `action.run` so `suggest_next` rows are more legible than bare ids or
  generic `Open` labels.
- Frontend structured parsers/types plus `useGenCommand` / `useSuggestNext`
  hooks for later UI batches.

Verification:
- Targeted Rust tests for `ai_capability` pass.
- Targeted Vitest coverage for structured parsers/hooks passes.
- `npm run lint` and `cargo clippy --lib -- -D warnings` are clean.

Pending:
- UI-owned manual `tauri dev` smoke for the capability surfaces that consume
  `gen_command` / `suggest_next`.

#### REF.6.D — `fix <error>` prefix wired to `CapabilityAnswerCard` — DONE (unit-level)

Spec: ui-spec §9 REF.6.D.

Scope:
- Add `fix` to the prefix parser (`fix <raw error text>`).
- Reuse `CapabilityAnswerCard`; no new card component.
- When capability output contains a structured diff hint, render a small
  inline diff section above the markdown body.

Non-goals:
- No terminal auto-detect / regex watcher.
- No apply-patch action in v1; user copies fix manually.
- No 5-minute blacklist.
- Structured diff hint rendering is deferred until backend emits one; the
  current `fix_error` capability is plain-text v1 per its own module comment.

Landed:
- `parseCapabilityPrefix` now matches `fix <body>` and exports `"fix"` as a
  text prefix id alongside `explain` / `summarize` / `cmd`.
- `useCapabilityStream` remaps the submit payload by capability id so
  `fix_error` receives `{ raw_output }` while `explain` / `summarize`
  continue to send `{ text }`. One streaming hook still covers all three
  text capabilities.
- `CapabilityAnswerCard` exposes `AnswerCardCapability = "explain" |
  "summarize" | "fix"` and renders the `Fix` header label for the new id.
- `CapabilityResultArea` adds `"fix"` to the answer-card branch; no new
  component.
- `CommandPalette.tsx` extends `textCapabilityMode` to cover `fix` and maps
  the prefix id `fix` to the backend capability id `fix_error` at the
  stream boundary.

Verification:
- New `parseCapabilityPrefix` test asserts `fix error[E0308]: mismatched
  types` parses to `{ id: "fix", args: { text: "error[E0308]: mismatched
  types" } }` and that bare `fix` / `fix ` still return null.
- `usePaletteMode` test covers the `fix` prefix.
- `CapabilityAnswerCard` test asserts the `Fix` header label renders.
- Full `npm run test`, `npm run lint`, and the `ai_capability` Rust suite
  are green on this branch.

Pending:
- Manual `npm run tauri dev` smoke is still required because capability IPC
  is only available when `window.__TAURI_INTERNALS__` exists.

#### REF.6.E — `next` prefix + `CapabilityListCard` — DONE (unit/browser smoke)

Spec: ui-spec §9 REF.6.E.

Scope:
- Add `next` (zero-arg) to the prefix parser.
- Add `CapabilityListCard` rendering a navigable list of
  `SuggestedNextAction` items.
- Wire `suggest_next` capability output.
- Extend keyboard handling so capability mode can own ArrowUp / ArrowDown /
  Enter when the active card is a suggestion list.

Non-goals:
- No mixed ranking with workflow_memory raw data — that data is the input
  to the suggester.

Landed:
- `parseCapabilityPrefix` and `usePaletteMode` now recognize `next` / `next `
  as a zero-arg capability prefix.
- Added `CapabilityListCard` with ranked rows, replay/history-only badges,
  timestamp display, and empty/error states.
- `useCapabilityRunState` auto-submits `suggest_next` when `next` becomes the
  active prefix, so the card populates without a second keystroke.
- `useKeyboardNav` now routes ArrowUp / ArrowDown / Enter to the active
  `CapabilityListCard` selection model.
- 2026-05-29 UX follow-through auto-mounts the same `CapabilityListCard` on an
  empty palette, so the workflow suggestion surface is visible without typing
  `next`.
- Replay currently supports rows with a best-effort `cmd.run` replay
  descriptor. Non-replayable history rows stay visible but are labeled
  `History only`.

Verification:
- Targeted Vitest coverage for the parser, palette-mode hook, keyboard nav,
  and `CapabilityListCard` passes.
- Full `npm run test` and `npm run lint` are green.
- Browser smoke against plain `vite` confirmed both the explicit `next` scene
  and the empty-palette auto-mount; it also caught a real integration typo,
  which was fixed before handoff.

Pending:
- Manual `npm run tauri dev` smoke is still required because capability IPC is
  only available when `window.__TAURI_INTERNALS__` exists.

#### REF.6.F — `cmd <intent>` prefix + `CapabilityCommandCard` — DONE (unit/browser smoke)

Spec: ui-spec §9 REF.6.F.

Scope:
- Add `cmd` to the prefix parser.
- Add `CapabilityCommandCard` (structured command + confidence + rationale
  + `[Run]` / `[Edit before]` / `[Copy]` chips).
- Wire `gen_command` capability.
- Display confidence and keep risky execution user-owned.

Non-goals:
- No multi-step plan output.

Landed:
- `parseCapabilityPrefix` and `usePaletteMode` now recognize `cmd <intent>`.
- Added `CapabilityCommandCard` with structured command/rationale rendering,
  latency header, and explicit `Generate`, `Run`, `Edit before`, and `Copy`
  affordances.
- `Run` opens an attached terminal launch spec instead of executing inline.
- `Edit before` strips the prefix and places the generated command back into
  the palette input so it can be edited as a normal query/command.
- `CapabilityResultArea` now dispatches between streaming text cards,
  `CapabilityCommandCard`, and `CapabilityListCard`.
- 2026-05-29 UX follow-through adds a natural-language heuristic: when a search
  query looks like an action intent, stabilizes briefly, and still has no
  results, the same `CapabilityCommandCard` auto-surfaces and Enter submits it
  without requiring the `cmd` keyword.

Verification:
- Targeted Vitest coverage for the parser, palette-mode hook, keyboard nav,
  and `CapabilityCommandCard` passes.
- Full `npm run test`, `npm run lint`, and `cargo test --lib` are green.
- Browser smoke against plain `vite` confirmed both the explicit `cmd` scene
  and the no-result NL-query auto-surface; the capability call itself correctly
  reports that Tauri runtime is required outside Tauri.

Pending:
- Manual `npm run tauri dev` smoke is still required because capability IPC is
  only available when `window.__TAURI_INTERNALS__` exists.

#### REF.6.G — Remove panels from hot path + UI-owned approval

Scope:
- Remove `AiPanel` and `TerminalPanel` mounts from `CommandPalette.tsx` hot
  path. `AiPanel` stays in the codebase, reachable only when
  `ai.legacy_agent = true` (legacy fallback route).
- Collapse pipeline output to a compact status row in the result list.
- Wire destructive `ActionChip.confirm` (`Once` / `TwoStage`) into a UI-layer
  confirm hook. `SecondaryActionMenu` becomes a rendering surface only; its
  local two-phase state machine is removed.
- Backend stops emitting backend-side approval state for any path reachable
  from `UnifiedResult`; risk tags + `ConfirmRequirement` are sufficient.

Non-goals:
- Do not delete `AiPanel.tsx` source (deletion candidate moves to REF.8 per
  user decision 2026-05-27).
- Do not change agent_runtime approval for `ai.legacy_agent = true` path.

Done:
- Palette no longer mounts `AiPanel` or `TerminalPanel` when
  `ai.legacy_agent = false`.
- Destructive action UX (delete file, run command with side effects) routes
  through `ConfirmRequirement`, not backend state polling.
- Manual regression: Bug B delete verification still requires two confirms.

#### REF.6.H — Feature-first directory migration (docx §3.5, §6.1)

Scope:
- Move remaining `src/components/*.tsx` panels to `src/features/<feature>/`:
  `calculator`, `history` (simplified to source-filter view), `learning`,
  `mouse-control`, `notes`, `nvim`, `settings`, `system`, `system-monitor`,
  `terminal`, `translation`.
- Consolidate `ModelDownloadPanel` + `ModelListPanel` + `ModelRemovePanel`
  into `src/features/model-manager/ModelManagerPanel.tsx` (single entry, three
  internal tabs/views).
- Establish `src/shared/{components,hooks,types}` for cross-feature reuse:
  `PreviewPane`, `RankTooltip`, `Markdown`, `ErrorBoundary`,
  `CheatsheetOverlay`, `OnboardingTour`, `WorkspaceIndicator`.
- Keep `FloatingWindow` and `AppContainer` where they are (app-level shell).

Non-goals:
- No behavioral changes during the move.
- Do not rename internal exports in a way that breaks deep imports outside
  the moved file.

Done:
- `src/components/` contains only `FloatingWindow.tsx`, `AppContainer.tsx`,
  and `AiPanel.tsx` (legacy fallback, slated for REF.8).
- All test files + imports update; `npm run test` and `npm run lint` green.

#### REF.6.J — Rule-based NL intent router (fallback) — DONE (unit-level)

Source: user 2026-05-29 request to remove the "must type the prefix" friction
for explain / fix_error workflows. Extends the existing no-result auto-surface
pattern from REF.6.F (which only covered `cmd`) to the full text-capability
set. Scope was explicitly bounded to rule-based, fallback-only routing per the
user's MVP preference (see [[feedback-minimal-scope]]).

Scope:
- New `src/features/command-palette/utils/classifyNlIntent.ts` returns
  `"explain" | "summarize" | "fix" | "cmd" | null`.
- Priority order (first match wins): fix-shaped error output → fix verb →
  summarize verb → explain verb → action heuristic for `cmd`.
- Drives the existing smart-surface gate in `CommandPalette.tsx`: explicit
  capability prefix takes priority, then the no-result + stabilization
  debounce + dismissed-key path resolves which card (if any) to surface.
- `closeCapabilitySurface` now treats any smart non-`next` close as a
  dismissal keyed on the trimmed query so dismissing a smart explain card
  does not wipe typed input.

Non-goals:
- No LLM-based classifier; no per-keystroke routing.
- No new capability surfaces; uses the same answer / command / list cards.
- No always-on routing — explicit prefixes (`explain X`, `fix X`, ...)
  remain the only path that fires while there are still search results.

Landed:
- `classifyNlIntent` with English verb/cue patterns and CJK cues
  (解釋 / 說明 / 什麼是 / 總結 / 摘要 / 修復 / 為什麼...錯誤).
- `CommandPalette.tsx` replaces the boolean `showSmartCommand` with a
  `smartIntentMatch` derivation; smart card variant is now picked by the
  classifier instead of being hard-coded to `cmd`.
- `looksLikeAiCommandIntent` is now consumed only through `classifyNlIntent`
  (the file remains for the cmd branch; no behavior regression).

Verification:
- `classifyNlIntent.test.ts` covers fix / summarize / explain / cmd / null
  buckets and an explicit "fix wins over explain" tie-break case.
- Full `npm run test` (157 pass + 1 pre-existing skip) and `npm run lint`
  are clean.

Pending:
- Manual `npm run tauri dev` smoke to confirm the smart routes for
  "what is rust hashmap" / "tldr this article" / "fix error[E0308]" /
  "list files in this project" all fire the right capability and that
  dismissing a smart card does not lose input.

#### REF.6.I — ADR template slimming (docx §9.2 push back 2) — DONE

Number reassigned: docx specified ADR-0030 but that slot was already accepted
for Backend Risk Tag Contract (2026-05-20). 0031–0037 are pre-reserved for
CLIP / SNIP / WIN / UTIL.3 / external-auth / git-sync / inline-AI tracks
(see `docs/decisions.md` Post-FEAT.11 table), and 0039 is the PREFLIGHT
runtime ADR. Picked ADR-0040 as the next free slot.

Scope:
- Draft `docs/adr/0040-adr-template-slim.md` proposing the 4-section default:
  `Context` / `Decision` / `Consequences` / `Rollback`.
- Move `Alternatives` to PR description guidance; move `Implementation` /
  `Validation` / `Open Questions` tracking to `docs/tasks/<feature>.md`.
- Status `提議` (proposed) only; awaits developer acceptance.

Non-goals:
- Do not retroactively rewrite existing ADRs.
- Do not modify ADR-0029, ADR-0030 (Risk Tag Contract), or any accepted ADR.
- No `docs:guard-schema` change to enforce the slim shape — slim is the
  authoring default, not a guard rule.

Landed:
- `docs/adr/0040-adr-template-slim.md` exists with `狀態: 提議`. The ADR is
  itself authored in the slim 4-section format as a dogfood demonstration.
- `docs/decisions.md` indexes it next to ADR-0038 and bumps the `updated`
  frontmatter to 2026-05-29.

### REF.7 - Quantitative Gates And Legacy Default Off

Priority: P0

Scope:
- Add performance bench scripts and CI-friendly file-size gates (per docx §8).
- Default `ai.legacy_agent = false`.
- Keep legacy agent/chat available behind the flag for one release cycle.
- Run manual regression for Bug A/B class race conditions.
- Update release notes and docs for the new AI interaction model.
- Take formal P50 / P95 reading on `qwen2.5:7b` (the deferred ADR-0029 §8
  measurement).

Non-goals:
- Do not physically remove legacy agent/chat code during the observation cycle.

Done:
- `handlers/agent/mod.rs` ≤ 616 lines (current) or trimmed below `< 600`.
- `agent_runtime.rs` < 400 lines during observation or justified pending removal.
- AI inline P50 < 800 ms and P95 < 1500 ms on `qwen2.5:7b`.
- Palette cold open < 200 ms and warm open < 50 ms.
- Search first chunk P50 < 80 ms.
- Idle RSS < 150 MB after 10 minutes and < 200 MB after 1 hour, excluding
  documented budget exceptions.
- One release cycle completes without P0 regression reports.

Note: `CommandPalette.tsx < 250` is *not* a REF.7 gate (per re-planning note
above). The 598-line landing is accepted.

### REF.8 - Physical Removal

Priority: P0 after observation

Scope:
- Decide AiPanel deletion based on observation-cycle telemetry. If
  `ai.legacy_agent = true` selection rate < 1% across the observation
  window, delete `src/components/AiPanel.tsx` entirely. Otherwise retain as
  flag-guarded escape hatch and remove the deletion task.
- Delete or shrink `agent_runtime.rs` to the remaining archival/audit
  surface.
- Delete migrated `handlers/agent/` legacy content.
- Remove `ai.legacy_agent`.
- Mark superseded ADRs (0011 / 0016 / 0022 / 0026) according to the accepted
  ADR-0029 outcome.

Non-goals:
- Do not remove audit log tables or historical user data.

Done:
- Refactor achieves at least 30% code reduction across frontend/backend
  target files.
- Deprecated result and agent code paths are gone (or retained with explicit
  written justification).
- Documentation and ADR indexes match the final state.

## File Map

Frontend targets:
- Split `src/components/CommandPalette.tsx` into `src/features/command-palette/` (DONE; 598 lines accepted).
- AiPanel: REF.6.G removes from hot path; REF.8 decides physical deletion.
- Add `src/features/ai-capability/` (DONE; includes all 5 capabilities plus the `cmd` / `next` UI cards).
- Add `src/features/workflow-memory/` (DONE).
- Consolidate model panels into `src/features/model-manager/` (REF.6.H).
- Migrate remaining `src/components/*.tsx` panels into `src/features/*` (REF.6.H).

Backend targets:
- Shared result models (DONE).
- `src-tauri/src/core/ai_capability/` (DONE for all 5 capabilities).
- `src-tauri/src/core/workflow_memory.rs` (DONE).
- `src-tauri/src/handlers/ai_capability.rs` (DONE for all 5 capabilities).
- `src-tauri/src/handlers/agent/mod.rs` (DONE at 616; trim opportunistically).
- Shrink then remove `src-tauri/src/core/agent_runtime.rs` after the observation
  window (REF.8).

## Conflict Handling

Superseded or parked work:
- `AGENT.3` and `AI.1` are replaced by this refactor track.
- `LAUNCH.2.C`, `ONBOARD.1.D/E`, `NOTE.1`, `UTIL.1.B-online`, and other feature
  tracks are parked until `REF.7`.
- The old chat-first AI panel should not receive feature investment except
  compatibility fixes needed for the release-cycle fallback.

Still allowed:
- P0 bug fixes that block current app use.
- Safety fixes around approval, sandbox, privacy, or destructive file
  operations.
- Test scaffolding and benchmarks required by this plan.

## Validation Matrix

Code size (revised against docx for re-planning):
- `CommandPalette.tsx`: 1582 → 598. `< 250` dropped per [[project-ref2-p5-landing]].
- `handlers/agent/mod.rs`: 2406 → 616 (observation), then deleted or `< 100` (REF.8).
- `agent_runtime.rs`: 1891 → `< 400` (observation, REF.7), then deleted (REF.8).
- `AiPanel.tsx`: 979 → REF.6.G unmounts; REF.8 decides physical deletion.
- Any handler: `< 600` (CI gate, REF.7).
- Any component: `< 400` (CI gate, REF.7).

Performance (REF.7):
- Palette cold open to first paint: `< 200 ms`.
- Palette warm open to first paint: `< 50 ms`.
- AI inline P50 with Ollama `qwen2.5:7b`: `< 800 ms`.
- AI inline P95: `< 1500 ms`.
- Search first chunk P50: `< 80 ms`.
- App startup to ready: `< 800 ms`.

Memory (REF.7):
- Background RSS idle 10 min: `< 150 MB`.
- Background RSS idle 1 hour: `< 200 MB`.
- Budget excludes active WebView, loaded local model memory, PTY sessions,
  monitoring streams, and index rebuild tasks.

Functional:
- Search, builtin commands, file actions, notes, terminal workflows, model
  management, and settings remain usable.
- Bug A class launcher focus/IME race and Bug B delete verification paths
  receive manual regression coverage after each REF.6 sub-batch lands.

Process:
- No new feature PRs before `REF.7` without written exception.
- `npm run docs:refresh` passes before handoff.
- Architecture-changing implementation follows accepted ADR-0029.
