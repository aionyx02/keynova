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

Source: extracted from the external planning document
`Keynova_Refactor_Plan_v1-2.docx` (not currently checked into this repo).
Anchors below preserve the referenced docx sections.

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
  auto-detection (NL heuristic / terminal regex / empty-state mount). Developer
  redirected to **prefix-keyword** pattern: type `explain <q>` / `summarize <t>` /
  `cmd <intent>` / `fix <error>` / `next` in the palette; capability mode owns
  the result area; no row chips and no auto-detect. REF.6.A's per-row Explain
  chip and `Ctrl+E` are removed in REF.6.B. Wire format (`UnifiedResult`) is
  retained for non-AI action data. Authority: see [[feedback-inline-ai-prefix]].
  Details in `docs/tasks/refactor-ai-capability-ui-spec.md`.

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

### REF.4 - Stateless AI Capability Layer — PARTIALLY DONE

Three capabilities landed: `explain`, `summarize`, `fix_error`. The remaining
docx Step 4 capabilities — `gen_command`, `suggest_next` — are intentionally
deferred to REF.6.C so they ship together with their UI scenes (4.4 / 4.5),
which need workflow memory wiring (REF.5) already in place.

Live Ollama smoke against `qwen2.5:0.5b` cold-start: 4.6–5.0 s for `explain` /
`fix_error`. Formal P50/P95 reading on `qwen2.5:7b` remains pending until REF.7
bench scripts exist.

### REF.5 - Workflow Memory — DONE

Schema v4 `workflow_history` + `record` / `suggest` entry points landed. No
`suggest_next` capability wiring yet (that is REF.6.C / REF.6.E scope).

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

#### REF.6.C — Complete capability set: gen_command + suggest_next (docx §3.2)

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

Done:
- 5 / 5 capabilities live behind one `call_capability` entry.
- Unit tests cover typed payload contracts.
- Live Ollama smoke green for both new capabilities.

#### REF.6.D — `fix <error>` prefix wired to `CapabilityAnswerCard`

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

Done:
- Pasting a cargo error after `fix ` streams a usable fix suggestion within
  5 s on cold model.
- Switching to and from `fix` prefix is instant.

#### REF.6.E — `next` prefix + `CapabilityListCard`

Spec: ui-spec §9 REF.6.E.

Scope:
- Add `next` (zero-arg) to the prefix parser.
- Add `CapabilityListCard` rendering a navigable list of
  `SuggestedNextAction` items.
- Wire `suggest_next` capability output.
- Extract row-navigation keyboard handling into a shared hook reusable by
  `SearchResultsList`.

Non-goals:
- No automatic empty-state mounting (docx §4.4 split dropped).
- No mixed ranking with workflow_memory raw data — that data is the input
  to the suggester.

Done:
- Typing `next` returns ranked suggestions within 1 s warm cache.
- Arrow keys + Enter dispatch each suggestion via its `action_ref`.
- Empty result handled gracefully (`No recent workflows`).

#### REF.6.F — `cmd <intent>` prefix + `CapabilityCommandCard`

Spec: ui-spec §9 REF.6.F.

Scope:
- Add `cmd` to the prefix parser.
- Add `CapabilityCommandCard` (structured command + confidence + rationale
  + `[↵ Run]` / `[Edit before]` / `[Copy]` chips).
- Wire `gen_command` capability.
- Confidence display: `low` dims `[Run]` and requires explicit Tab focus;
  `high` auto-focuses `[Run]`.

Non-goals:
- No NL-query auto-detection (prefix is the only entry).
- No multi-step plan output.

Done:
- `cmd 把當前 branch 上 commit 推到 origin` returns `git push origin HEAD`-
  shaped card within 1 s warm cache.
- `[Edit before]` returns command into palette input (prefix stripped).
- Low-confidence card does not auto-focus `[Run]`.

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

#### REF.6.I — ADR-0030 template slimming (docx §9.2 push back 2)

Scope:
- Draft `docs/adr/0030-adr-template-slim.md` proposing the 4-section template:
  `Context` / `Decision` / `Consequences` / `Rollback`.
- Move `Alternatives` to PR description guidance; move `Validation` /
  `Implementation` tracking to task files.
- Status `proposed` only; awaits developer acceptance.

Non-goals:
- Do not retroactively rewrite existing ADRs.
- Do not modify ADR-0029 or any accepted ADR.

Done:
- ADR-0030 file exists with `status: proposed`.
- `docs/decisions.md` indexes it.

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
- Add `src/features/ai-capability/` (DONE for 3 capabilities; extend in REF.6.C).
- Add `src/features/workflow-memory/` (DONE).
- Consolidate model panels into `src/features/model-manager/` (REF.6.H).
- Migrate remaining `src/components/*.tsx` panels into `src/features/*` (REF.6.H).

Backend targets:
- Shared result models (DONE).
- `src-tauri/src/core/ai_capability/` (DONE for 3 capabilities; extend in REF.6.C).
- `src-tauri/src/core/workflow_memory.rs` (DONE).
- `src-tauri/src/handlers/ai_capability.rs` (DONE; extend in REF.6.C).
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
