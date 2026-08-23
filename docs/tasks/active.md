---
type: task_index
status: active
priority: p0
updated: 2026-08-23
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
- [x] `REF.4` stateless AI capability layer: all 5 capabilities behind one `call_capability` entry.
- [x] `REF.5` workflow memory schema v4 + `record` / `suggest`.
- [x] `REF.6` search box = pure dispatcher. Sub-batches `A`–`G`, `I`, `J` +
  `REF.6.H` (feature-first directory migration: 11 panels + 3 model panels →
  `src/features/<feature>/`, 7 shared components → `src/shared/components/`) done.
  Model-manager tab consolidation deferred to backlog (cosmetic, non-blocking).
- [x] `REF.7` quantitative gates. .A/.B in v0.3.0; .C release notes shipped;
  .D **done** (2026-06-06): ADR-0029 §10 + §8 tiered gates (`qwen2.5:1.5b` ref
  default, PASS). The `ai.legacy_agent` default/observation items are moot —
  the flag and the whole legacy agent were removed in REF.8.
- [x] `REF.8` legacy-agent removal + parked-feature cut (2026-06-09): deleted
  `agent_runtime`/`handlers/agent` + flag (ADRs 0016/0022/0023/0026 superseded by
  0029), cut nvim + mouse_control (notes kept). Detail: `sessions/2026-06-09.md`.

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

- [~] `UX.AUDIT` UI/UX consistency pass (developer-directed). Done: stale copy,
  a11y live regions, full i18n, UTF-8 BOM cleanup, `嚙` garbled-path fallback.
  Open: `嚙` root cause blocked on a repro (non-urgent; = STAB.4). Detail:
  `docs/tasks/ux-audit.md`.
- [x] `PRODUCT.1` v0.6 stable workflow core **complete** (search-core; `1.E`
  execution dropped, copy-only). Detail: `archive/product-1-workflow-core.md`.
- [x] `PRODUCT.2` v0.7 inline AI capabilities **complete** (2026-06-06; A–E,
  copy-only, ADR-0046). Detail: `archive/product-2-ai-capabilities.md`.

- [x] `CI.HARDEN` Dependabot + CodeQL + branch-protection (`/status` triage;
  `feature/ci-supply-chain-hardening`). Detail: `sessions/2026-06-09.md`.
  Dependabot half **retired** 2026-08-23; CodeQL stands. `sessions/2026-08-23.md`.
- [x] `CONT` (`8fc68c7`): idle `next` (ADR-0052), resize coalescing, Ctrl+K fix.
  `STAB` (`8825cce`): crash-log hook (ADR-0051), `/diag` fixes.
- [x] `PRODUCT.4` ranking (`b0a8b5c`): 4.A freq+workspace (ADR-0052), 4.B
  success-rate (ADR-0053). Detail: `tasks/product-4-ranking.md`.
- [x] `PROFILE` workspace profiles: .1 (`b616bdd`/`6964d90`, ADR-0054); .2
  **removed** (2026-06-10, pins cut, ADR-0055 rejected); .3 dropped. Detail:
  `tasks/product-4-profiles.md`.
- [~] `SEC-PERF` security + perf hardening (`hardening/security-perf`): audit
  baseline, `security.md` reconcile (nvim/keychain/CSP), perf baseline. Detail:
  `tasks/security-perf-hardening.md`.
- [~] `ICON.NATIVE`+UX (2026-06-13): native icon rewrite (ADR-0056). `tasks/icon-native-and-ux.md`.
- [x] `XPLAT` x-platform stability Phase 1 (CI-green 3 OSes, in `v0.7.1`):
  per-push 3-OS clippy+test + static review (no panics). Phase 2 deferred.
  Detail: `tasks/cross-platform-stability.md`.

### P2

- Only safety fixes or regressions that directly block the P0 refactor track.

## Strategy

Keynova's active priority is the search-first workflow refactor: AI is a stateless capability layer invoked inline from the palette; chat-first surfaces leave the hot path.

Freeze status (developer-directed): `PRODUCT.1`/`.2`/`.3` complete; REF.6/7/8
done. The legacy ReAct agent was fully removed (REF.8); parked tracks
(`AGENT.*`, `CLIP.1`) stay frozen, non-blocking.

Keep `active.md` compact. Put batch-level task detail in `docs/tasks/refactor-ai-capability.md`, detailed implementation notes in `docs/memory/sessions/YYYY-MM-DD.md`, and future non-refactor ideas in `docs/tasks/backlog.md`.

## Next Phase Candidates

- `XPLAT` Phase 2: fill `platform/{linux,macos}.rs` (CI-gated).
