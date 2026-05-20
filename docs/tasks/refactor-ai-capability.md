---
type: task_plan
status: active
priority: p0
updated: 2026-05-20
context_policy: on_demand
owner: project
tags: [refactor, ai-capability, unified-result, workflow-memory, search-first]
---

# P0 Refactor: AI Capability Layer And Unified Search

Source: `C:\Users\shawn\Downloads\Keynova_Refactor_Plan_v1-2.docx`

Goal: reposition Keynova as a keyboard-first workflow tool. Unified search is the product spine; AI becomes a stateless capability invoked inline from result rows and workflow surfaces.

## Priority Rules

- This plan is the highest-priority runnable task source after `docs/tasks/active.md`.
- Execute batches in order unless a batch explicitly says it may run in parallel.
- Freeze new feature work until `REF.7`, except for P0 regressions, safety fixes, or work directly required by this plan.
- Keep legacy agent/chat code behind `ai.legacy_agent` during the compatibility window; physical removal waits for `REF.8`.
- Do not implement architecture-changing code until ADR-0029 is accepted by the developer.

## Batch Plan

### REF.0 - ADR-0029 Decision Lock

Priority: P0

Scope:
- Create `docs/adr/0029-ai-capability-layer.md` from the plan's ADR draft.
- State that AI moves from product core to capability layer.
- State that independent chat-first `AiPanel` is removed from the target architecture.
- State that autonomous multi-step ReAct is replaced by stateless single-step capability calls.
- State that backend approval state becomes UI-owned confirmation over backend risk tags.
- Mark ADR-0011, ADR-0016, ADR-0022, ADR-0023, ADR-0024, ADR-0026, and planned ADR-0037 as affected.

Non-goals:
- Do not change runtime code.
- Do not mark ADR-0029 accepted without developer action.
- Do not amend affected ADRs yet; only mark impact.

Done:
- ADR-0029 exists as proposed/accepted per developer decision.
- `docs/decisions.md` references ADR-0029.
- Active tasks reflect this plan as P0.

### REF.1 - Unified Result Schema

Priority: P0

Scope:
- Add backend shared result types for `UnifiedResult`, `ResultSource`, `ActionChip`, `ConfirmRequirement`, `PreviewPayload`, `RankSignals`, and source metadata.
- Add matching frontend TypeScript types.
- Add conversion shims from existing search, builtin command, and file/action results.
- Keep old `SearchResult` and `BuiltinCommandResult` as deprecated compatibility types.

Non-goals:
- Do not switch `CommandPalette` consumption yet.
- Do not delete old result models.

Done:
- Rust checks and TypeScript checks pass.
- Three existing result producers can emit or convert to `UnifiedResult`.
- UI scenarios in the design doc are represented by the schema.

### REF.2 - Command Palette Feature Split

Priority: P0

Scope:
- Create `src/features/command-palette/`.
- Split `CommandPalette.tsx` into feature hooks/components while preserving behavior.
- Prioritize `useSearchStream`, `usePipeline`, and `useRecentlyDeleted` because they touch Bug A/B state.
- Move `CommandSuggestions`, `FilterChips`, and `SecondaryActionMenu` under the command-palette feature.
- Keep shared primitives in `src/shared/components/` only when reused outside the feature.

Non-goals:
- Do not switch to consuming `UnifiedResult`; that waits for `REF.6`.
- Do not remove `AiPanel`.

Done:
- `CommandPalette.tsx` is under 250 lines.
- Extracted hooks have focused tests.
- Bug A launcher/focus regression and Bug B delete verification flows are manually checked after the split.
- Visual and keyboard behavior match the pre-split baseline.

### REF.3 - Agent Handler Module Split

Priority: P0

Scope:
- Split `handlers/agent/mod.rs` so lifecycle stays separate from tool dispatch, local context, dev command running, and intent answer helpers.
- Move `LocalContextSearcher` to `core/local_context.rs`.
- Move dev command runner logic to `core/dev_runner.rs` for later `fix_error` capability use.
- Move or isolate risk/safety helpers so `REF.4` can reuse them.
- Mark intent-answer modules deprecated where they only support chat-first behavior.

Non-goals:
- Do not remove `agent_runtime.rs` yet.
- Do not change legacy ReAct behavior except where required by safe extraction.

Done:
- `handlers/agent/mod.rs` is under 600 lines during the observation phase.
- Existing agent tests still pass.
- Approval state coupling is explicit and isolated.

### REF.4 - Stateless AI Capability Layer

Priority: P0

Scope:
- Add `core/ai_capability/` with a single public call entry.
- Add compile-time capability registry, resolver, typed payload/result contracts, and risk tagging.
- Implement `explain`, `summarize`, and `fix_error` first.
- Add `handlers/ai_capability.rs` IPC entry.
- Add frontend `src/features/ai-capability/` hooks and inline UI surfaces.
- Preserve prompt audit per call.

Non-goals:
- Do not allow capability functions to call follow-up capabilities.
- Do not keep session memory inside capability execution.
- Do not delete `agent_runtime.rs` yet.
- Keep `ai.legacy_agent` default true until `REF.7`.

Done:
- Each initial capability has unit and integration coverage.
- Inline explain and terminal fix-error scenarios work.
- Ollama `qwen2.5:7b` AI inline P50 latency is under 800 ms or the gap is documented before `REF.7`.

### REF.5 - Workflow Memory

Priority: P0

Parallelism:
- May run alongside `REF.4` after `REF.1` schema shape is stable.

Scope:
- Add knowledge DB schema v4 with `workflow_history`.
- Add `core/workflow_memory.rs` with `record` and `suggest` entry points.
- Add frontend `src/features/workflow-memory/` for recent workflows.
- Use `context_hash` and heuristic ranking before considering embeddings.

Non-goals:
- Do not connect `suggest_next` capability until the capability layer exists.
- Do not add broad local-content scanning.

Done:
- Migration is additive and has rollback guidance.
- Recent workflow suggestions work without AI.
- Workflow memory gives unified search differentiation without slowing cold open.

### REF.6 - Search Box As Pure Dispatcher

Priority: P0

Scope:
- Switch `CommandPalette` result rendering to consume `UnifiedResult`.
- Treat `ActionChip` as the first-class action model.
- Remove embedded `AiPanel` and `TerminalPanel` mounts from the palette hot path.
- Collapse pipeline output to a compact status row.
- Make UI approval confirmation read `ConfirmRequirement` and risk tags from results.

Non-goals:
- Do not delete old backend compatibility types yet.
- Do not physically remove legacy agent code yet.

Done:
- UI scenarios 4.1 through 4.5 from the design doc work.
- Palette layout stays stable except for explicit preview expansion.
- Search, builtin command, file action, note, and model workflows do not regress.

### REF.7 - Quantitative Gates And Legacy Default Off

Priority: P0

Scope:
- Add performance bench scripts and CI-friendly file-size gates.
- Default `ai.legacy_agent = false`.
- Keep legacy agent/chat available behind the flag for one release cycle.
- Run manual regression for Bug A/B class race conditions.
- Update release notes and docs for the new AI interaction model.

Non-goals:
- Do not physically remove legacy agent/chat code during the observation cycle.

Done:
- `CommandPalette.tsx` < 250 lines.
- `handlers/agent/mod.rs` < 600 lines during observation.
- `agent_runtime.rs` < 400 lines during observation or justified pending removal.
- AI inline P50 < 800 ms and P95 < 1500 ms.
- Palette cold open < 200 ms and warm open < 50 ms.
- Search first chunk P50 < 80 ms.
- Idle RSS < 150 MB after 10 minutes and < 200 MB after 1 hour, excluding documented budget exceptions.
- One release cycle completes without P0 regression reports.

### REF.8 - Physical Removal

Priority: P0 after observation

Scope:
- Delete chat-first `AiPanel` code and remove palette links to it.
- Delete or shrink `agent_runtime.rs` to the remaining archival/audit surface.
- Delete migrated `handlers/agent/` legacy content.
- Remove `ai.legacy_agent`.
- Mark superseded ADRs according to the accepted ADR-0029 outcome.

Non-goals:
- Do not remove audit log tables or historical user data.

Done:
- Refactor achieves at least 30% code reduction across frontend/backend target files.
- Deprecated result and agent code paths are gone.
- Documentation and ADR indexes match the final state.

## File Map

Frontend targets:
- Split `src/components/CommandPalette.tsx` into `src/features/command-palette/`.
- Delete target: `src/components/AiPanel.tsx` after observation.
- Move command-palette support components: `CommandSuggestions`, `FilterChips`, `SecondaryActionMenu`.
- Add `src/features/ai-capability/`.
- Add `src/features/workflow-memory/`.
- Consolidate model panels into `src/features/model-manager/` after hot-path refactor work is stable.

Backend targets:
- Add shared result models under `src-tauri/src/models/` or the established local model namespace.
- Add `src-tauri/src/core/ai_capability/`.
- Add `src-tauri/src/core/workflow_memory.rs`.
- Add `src-tauri/src/handlers/ai_capability.rs`.
- Split `src-tauri/src/handlers/agent/mod.rs`.
- Shrink then remove `src-tauri/src/core/agent_runtime.rs` after the observation window.

## Conflict Handling

Superseded or parked work:
- `AGENT.3` and `AI.1` are replaced by this refactor track.
- `LAUNCH.2.C`, `ONBOARD.1.D/E`, `NOTE.1`, `UTIL.1.B-online`, and other feature tracks are parked until `REF.7`.
- The old chat-first AI panel should not receive feature investment except compatibility fixes needed for the release-cycle fallback.

Still allowed:
- P0 bug fixes that block current app use.
- Safety fixes around approval, sandbox, privacy, or destructive file operations.
- Test scaffolding and benchmarks required by this plan.

## Validation Matrix

Code size:
- `CommandPalette.tsx`: 1582 lines to < 250.
- `handlers/agent/mod.rs`: 2406 lines to < 600 during observation, then deleted or < 100.
- `agent_runtime.rs`: 1891 lines to < 400 during observation, then deleted.
- `AiPanel.tsx`: 979 lines to deleted after observation.
- Any handler: < 600 lines.
- Any component: < 400 lines.

Performance:
- Palette cold open to first paint: < 200 ms.
- Palette warm open to first paint: < 50 ms.
- AI inline P50 with Ollama `qwen2.5:7b`: < 800 ms.
- AI inline P95: < 1500 ms.
- Search first chunk P50: < 80 ms.
- App startup to ready: < 800 ms.

Memory:
- Background RSS idle 10 min: < 150 MB.
- Background RSS idle 1 hour: < 200 MB.
- Budget excludes active WebView, loaded local model memory, PTY sessions, monitoring streams, and index rebuild tasks.

Functional:
- Search, builtin commands, file actions, notes, terminal workflows, model management, and settings remain usable.
- Bug A class launcher focus/IME race and Bug B delete verification paths receive manual regression coverage after `REF.2` and before `REF.7`.

Process:
- No new feature PRs before `REF.7` without written exception.
- `npm run docs:refresh` passes before handoff.
- Architecture-changing implementation follows accepted ADR-0029.
