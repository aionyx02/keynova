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

`PERF.1.FU`, `PREFLIGHT`, `BRAND.ICON` completed in v0.3.0 (`sessions/2026-05-30.md`).

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
- [~] `REF.7` quantitative gates, default `ai.legacy_agent = false`. .A/.B shipped in v0.3.0. .C release notes shipped; ADR-0029 §10 scaffolded, `pending REF.7.D` (`qwen2.5:7b` bench, user-action). Observation items stay `pending observation`.
- [~] `REF.8` physical removal. Done: `AiPanel.tsx` + `ai_legacy` route/builtin deleted (2026-06-01). Retained (developer): `agent_runtime.rs` + `handlers/agent/` + `ai.legacy_agent` flag (dormant). Not done: backend agent trim, flag removal, supersede ADRs 0011/0016/0022/0026.

Detailed batch definitions, done criteria, non-goals, file map, and validation gates live in `docs/tasks/refactor-ai-capability.md`.

### P1

- [x] `MEM.1` personal-memory layer (ADR-0043 proposed). .A/.B/.C all landed at
  unit level. Detail: `docs/tasks/personal-memory.md`.
- [x] `FEAT.GATE` feature-visibility gating (developer-directed; overrides REF.7
  freeze). `features.*` gate UI + IPC end-to-end (capability.call, dispatch
  namespace guard, search providers, frontend `FeatureFlagsContext`); model
  manager exempt. Detail: `docs/memory/sessions/2026-06-04.md`.
- [x] `DECOUP` feature self-registration decoupling (ADR-0044 proposed). `.1`–`.6`
  landed: 8 backend features + 6 frontend panels self-register; removing a feature
  ≈ delete module/folder + manifest. Detail: `docs/tasks/feature-decoupling.md`.

- [~] `UX.AUDIT` UI/UX consistency pass (developer-directed). Done: stale "AI
  Chat" copy, a11y live regions (incl. AI capability completion/error), full
  i18n conversion, UTF-8 BOM cleanup, and the garbled-text (`嚙`) fallback (raw
  path instead of "Path unavailable"). Open: `嚙` root cause still blocked on a
  repro (non-urgent). Detail: `docs/tasks/ux-audit.md`.
- [ ] `PRODUCT.1` v0.6 stable workflow core — **UNFROZEN 2026-06-05** (conditional
  partial unfreeze; search-core is orthogonal to the REF.7 AI gates). Batches
  `A`–`H`. `A`–`D` **done**: A=`workspace_boost`+`config_boost`, B=contract
  audit, C=`noise_penalty` demote, D=project-command discovery (copy-only;
  npm/cargo/make/just). Next `PRODUCT.1.E` (terminal workflow — execution + risk
  gate). Detail: `docs/tasks/product-1-workflow-core.md`.

### P2

- Only safety fixes or regressions that directly block the P0 refactor track.

## Strategy

Keynova's active priority is the search-first workflow refactor: AI is a stateless capability layer invoked inline from unified result rows and prefix-keyword palette flows; chat-first surfaces leave the hot path.

Freeze status (conditional partial unfreeze, 2026-06-05): `PRODUCT.1` search-core
is **unfrozen** (orthogonal to the AI hot path the REF.7 gates measure). Frozen
until `REF.7.D` + observation window close: `PRODUCT.2` and any AI/agent-touching
work. `REF.7.D` + observation items are now non-blocking tracking items, not a
queue gate. Other parked tracks (`AGENT.*`, `CLIP.1`, etc.) stay frozen.

Keep `active.md` compact. Put batch-level task detail in `docs/tasks/refactor-ai-capability.md`, detailed implementation notes in `docs/memory/sessions/YYYY-MM-DD.md`, and future non-refactor ideas in `docs/tasks/backlog.md`.

## Next Phase Candidates

- Scope and start `PRODUCT.1.A` (ranking baseline) — write the batch plan into
  `docs/tasks/product-roadmap.md` (primary vs simplification-only) before coding.
- `REF.7.D` stays available as a non-blocking tracking item: `ollama pull
  qwen2.5:7b && npm run bench:ai -- --runs 10 --model qwen2.5:7b`, then fill
  ADR-0029 §10 to close `REF.7.C` and unfreeze `PRODUCT.2`.
- After `REF.8`, refresh affected ADR statuses and architecture docs.
