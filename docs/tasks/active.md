---
type: task_index
status: active
priority: p0
updated: 2026-06-07
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
- [~] `REF.7` quantitative gates, default `ai.legacy_agent = false`. .A/.B in v0.3.0; .C release notes shipped. REF.7.D **done** (2026-06-06): ADR-0029 §10 filled + §8 → **tiered** (CPU-host P50<5s/P95<8s) with **`qwen2.5:1.5b` reference default** (PASS). PRODUCT.2 done. Detail: `sessions/2026-06-06.md`.
- [~] `REF.8` physical removal. Done: `AiPanel.tsx` + `ai_legacy` route/builtin deleted (2026-06-01). Retained (developer): `agent_runtime.rs` + `handlers/agent/` + `ai.legacy_agent` flag (dormant). Not done: backend agent trim, flag removal, supersede ADRs 0011/0016/0022/0026.

Detailed batch definitions, done criteria, non-goals, file map, and validation gates live in `docs/tasks/refactor-ai-capability.md`.

### P1

- [x] `MEM.1` personal-memory layer (ADR-0043 proposed). .A/.B/.C all landed at
  unit level. Detail: `docs/tasks/archive/personal-memory.md`.
- [x] `FEAT.GATE` feature-visibility gating (developer-directed; overrides REF.7
  freeze). `features.*` gate UI + IPC end-to-end (capability.call, dispatch
  namespace guard, search providers, frontend `FeatureFlagsContext`); model
  manager exempt. Detail: `docs/memory/sessions/2026-06-04.md`.
- [x] `DECOUP` feature self-registration decoupling (ADR-0044 proposed). `.1`–`.6`
  landed: 8 backend features + 6 frontend panels self-register; removing a feature
  ≈ delete module/folder + manifest. Detail: `docs/tasks/archive/feature-decoupling.md`.

- [~] `UX.AUDIT` UI/UX consistency pass (developer-directed). Done: stale "AI
  Chat" copy, a11y live regions (incl. AI capability completion/error), full
  i18n conversion, UTF-8 BOM cleanup, and the garbled-text (`嚙`) fallback (raw
  path instead of "Path unavailable"). Open: `嚙` root cause still blocked on a
  repro (non-urgent). Detail: `docs/tasks/ux-audit.md`.
- [x] `PRODUCT.1` v0.6 stable workflow core **complete** (search-core; `1.E`
  execution dropped, copy-only). Detail: `archive/product-1-workflow-core.md`.
- [x] `PRODUCT.2` v0.7 inline AI capabilities **complete** (2026-06-06; A–E,
  copy-only, ADR-0046). Detail: `archive/product-2-ai-capabilities.md`.

- [~] `CONT` workflow continuity on `feature/workflow-continuity`: CONT.1 backend
  transition `next` (ADR-0052) **done** + CONT.2 resize coalescing **done**.
  Remaining: CONT.1 proactive idle surface (+dev-run check). Detail:
  `tasks/workflow-continuity.md`.

### P2

- Only safety fixes or regressions that directly block the P0 refactor track.

## Strategy

Keynova's active priority is the search-first workflow refactor: AI is a stateless capability layer invoked inline from unified result rows and prefix-keyword palette flows; chat-first surfaces leave the hot path.

Freeze status (2026-06-06, developer-directed): `PRODUCT.1` and `PRODUCT.2` are
both **complete**. REF.7.D done + latency direction finalized (tiered §8 +
`qwen2.5:1.5b` default) satisfied the PRODUCT.2 gate. `ai.legacy_agent`
observation-window items remain separate, non-blocking tracking. Other parked
tracks (`AGENT.*`, `CLIP.1`, etc.) stay frozen.

Keep `active.md` compact. Put batch-level task detail in `docs/tasks/refactor-ai-capability.md`, detailed implementation notes in `docs/memory/sessions/YYYY-MM-DD.md`, and future non-refactor ideas in `docs/tasks/backlog.md`.

## Next Phase Candidates

- [~] `PRODUCT.3` trusted-release (ADR-0047/0048/0049/0050): self-contained
  items done + green; branch `feature/product-3-diagnostics-export`
  **merge-ready into `main`**. Only open: secret-gated signing (**deferred per
  dev**) + updater keypair. Detail: `sessions/2026-06-07.md`.
- `REF.7.C` final closer: user-side Bug A/B smoke.
- After `REF.8`, refresh affected ADR statuses and architecture docs.
