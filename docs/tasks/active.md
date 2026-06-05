---
type: task_index
status: active
priority: p0
updated: 2026-06-05
context_policy: always_retrievable
owner: project
tags: [refactor, ai-capability, search-first, p0]
---

# Active Tasks

## Active Queue

### P0

`PERF.1.FU`, `PREFLIGHT`, and `BRAND.ICON` completed in v0.3.0 (see `sessions/2026-05-30.md` COMPLETED markers). `PERF.1.FU` closed: real unique footprint is `~80 MB`, under the `200 MB` goal.

- [x] `REF.0` lock ADR-0029 as the governing decision for the AI capability refactor.
- [x] `REF.1` define `UnifiedResult` as the shared result/action contract.
- [x] `REF.2` split `CommandPalette.tsx` (landed 598 lines; `< 250` dropped, see plan).
- [x] `REF.3` split `handlers/agent/mod.rs` (landed 616 lines, observation target `< 600`).
- [x] `REF.4` stateless AI capability layer. All 5 capabilities are now live behind one `call_capability` entry: `explain`, `summarize`, `fix_error`, `gen_command`, `suggest_next`.
- [x] `REF.5` workflow memory schema v4 + `record` / `suggest`.
- [~] `REF.6` search box = pure dispatcher. Sub-batches `A`–`G`, `I`, `J` done
  (prefix dispatcher + all 5 capabilities + `CapabilityAnswerCard` family +
  `classifyNlIntent` NL fallback + ADR-0040). Only `REF.6.H` open:
  - [~] `REF.6.H` feature-first directory migration. 11 panels + 3 model panels relocated to `src/features/<feature>/`; 7 shared components moved to `src/shared/components/`. Model-manager tab consolidation deferred.
- [~] `REF.7` quantitative gates, default `ai.legacy_agent = false`. .A + .B shipped in v0.3.0 (tag `v0.3.0`, merged to `main`). .C release notes shipped as `docs/release-notes/v0.3.0.md`; ADR-0029 §10 measurement scaffolded, `pending REF.7.D`. .D is user-action (`qwen2.5:7b` 4.7 GB bench). Observation-window items (`ai.legacy_agent` off, idle RSS) stay `pending observation`.
- [~] `REF.8` physical removal. Done: `AiPanel.tsx` + `ai_legacy` route + `ai_legacy_chat` builtin deleted (developer lifted the observation gate 2026-06-01). Retained by developer decision: `agent_runtime.rs` + `handlers/agent/` + `ai.legacy_agent` flag (dormant, no UI entry). Not done: backend agent trim/removal, flag removal, superseding ADRs 0011/0016/0022/0026.

Detailed batch definitions, done criteria, non-goals, file map, and validation gates live in `docs/tasks/refactor-ai-capability.md`.

### P1

- [x] `MEM.1` personal-memory layer (ADR-0043 proposed). .A/.B/.C all landed at
  unit level. Detail: `docs/tasks/personal-memory.md`.
- [x] `FEAT.GATE` feature-visibility gating (developer-directed; overrides REF.7
  freeze). `features.*` gate UI + IPC end-to-end (capability.call, dispatch
  namespace guard, search providers, frontend `FeatureFlagsContext`); model
  manager exempt. Detail: `docs/memory/sessions/2026-06-04.md`.
- [x] `DECOUP` feature self-registration decoupling (ADR-0044 proposed;
  developer-directed). `.1`–`.6` all landed at unit level: 8 backend features +
  6 frontend panels self-register; dispatch guard spec-derived; search chain
  declarative. Removing a feature ≈ delete module/folder + manifest entries.
  Cross-cutting search/ai/agent stay central by ADR-0044 §2. Detail:
  `docs/tasks/feature-decoupling.md`.

- [~] `UX.AUDIT` UI/UX consistency pass (developer-directed). Done: stale "AI
  Chat" copy, a11y live regions (incl. AI capability completion/error), full
  i18n conversion, UTF-8 BOM cleanup, and the garbled-text (`嚙`) fallback (raw
  path instead of "Path unavailable"). Open: `嚙` root cause still blocked on a
  repro (non-urgent). Detail: `docs/tasks/ux-audit.md`.

### P2

- Only safety fixes or regressions that directly block the P0 refactor track.

## Strategy

Keynova's active priority is the search-first workflow refactor: AI is a stateless capability layer invoked inline from unified result rows and prefix-keyword palette flows; chat-first surfaces leave the hot path.

Until `REF.7` is complete, freeze new feature work unless it is required for the refactor, fixing a P0 regression, or protecting a safety boundary. `PREFLIGHT`, `BRAND.ICON`, and `PERF.1.FU` shipped/closed in v0.3.0.

Keep `active.md` compact. Put batch-level task detail in `docs/tasks/refactor-ai-capability.md`, detailed implementation notes in `docs/memory/sessions/YYYY-MM-DD.md`, and future non-refactor ideas in `docs/tasks/backlog.md`.

## Next Phase Candidates

- Run `REF.7.D` (`ollama pull qwen2.5:7b && npm run bench:ai -- --runs 10 --model qwen2.5:7b`), then fill ADR-0029 §10 to close `REF.7.C`.
- After `REF.7`, revalidate parked tracks from `docs/tasks/backlog.md`.
- After `REF.8`, refresh affected ADR statuses and architecture docs.
