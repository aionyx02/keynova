---
type: task_plan
status: active
priority: p1
updated: 2026-06-05
context_policy: on_demand
owner: project
tags: [product, workflow-core, ranking, search, unfrozen]
---

# PRODUCT.1 — v0.6 Stable Workflow Core

Detail home for the PRODUCT.1 batches. Unfrozen 2026-06-05 (conditional partial
unfreeze; search-core is orthogonal to the REF.7 AI gates). High-level roadmap:
`docs/tasks/product-roadmap.md`. Sub-batches `A`–`H`; only `A` is scoped here so
far.

## Audit: what ranking already exists

Before adding terms, the live ranking model (audited 2026-06-05):

- **Score model** (`handlers/search/ranking.rs`, `handlers/search.rs::apply_rank_boost`,
  `managers/search_manager.rs::rank_boost_breakdown`):
  `score = base + recency_boost + frequency_boost`.
  - `base` = text_match: Tantivy/Everything file score, app fuzzy score, or fixed
    source constants (command 90/85/40, note 75, history 65/70, memory 68, model 60).
  - `recency_boost` = 0/8/15/25 by age window; `frequency_boost` = `count.min(10) * 4`.
    Both come from `SearchManager.rank_memory`, keyed `source:path`, fed by
    `record_selection` (REF.5 post-success hook in `app/dispatch.rs`).
- **Workspace awareness already exists as a hard filter**, not a score:
  `resolve_workspace_filter` restricts file/folder/app results to
  `WorkspaceManager.current().project_root` (populated via `workspace.set_restore_state`);
  `:global` prefix escapes the filter. History already gets a `+10` same-workspace bump.
- **Rank-reason UI exists**: `ScoreBreakdown { base, recency_boost, frequency_boost }`
  ships over IPC; `RankTooltip.tsx` renders it on hover (room for more rows).
- **Determinism + tests exist**: `rank_boost_breakdown` has unit tests in
  `search_manager.rs`.

Roadmap target formula: `text_match + workspace_context + recency + frequency +
action_success - risk_penalty - noise_penalty`. **Live today:** text_match,
recency, frequency (+ workspace as a filter). **Missing:** workspace_context as a
*score*, action_success, risk_penalty, noise_penalty.

## PRODUCT.1.A — Ranking Baseline

Goal: make the transparent ranking formula real and documented, and add the one
missing term that directly serves the headline done-criterion ("same-name files
prefer the current workspace"). Keep it an additive, bounded extension of the
existing model — not a scoring rewrite.

### Primary (this batch)

1. **Formalize the transparent formula.** This audit section is the record of
   which terms are live vs deferred and why. No code; it anchors 1.A scope.
2. **Add `workspace_context` as a score component** (distinct from the existing
   hard filter, so it also helps in `:global` mode and as a soft signal):
   - Extend `ScoreBreakdown` (Rust `models/action.rs`, TS `types/search.ts`) with
     `workspace_boost: i64`.
   - In `apply_rank_boost` (`handlers/search.rs`): for `file`/`folder`/`app`
     results whose `path` is under `current().project_root`, add a bounded boost.
     Resolve `project_root` once per search and thread it into scoring (avoid
     locking `workspace_manager` per item).
   - Surface in `RankTooltip.tsx` (new row + add to `total`) and add `rank.workspace`
     to both locales.
3. **README / config soft boost (trim-able):** small additive bump for `README*`
   and common config files (`*.toml`, `*.json`, `*.yml`, `Makefile`, etc.). Folds
   into the workspace component or its own breakdown line. Cut this first if the
   batch grows.

### Simplification-only / deferred (do NOT block 1.A closure)

- **`action_success`** as a distinct term — the existing `frequency` (selection
  count) is the MVP proxy; a true success-rate term needs per-item success
  tracking (dispatch already records status). Follow-up, not 1.A.
- **`risk_penalty`** — risk lives on `UnifiedResult.ActionChip`, not result rows;
  a ranking penalty is speculative here. Revisit with PRODUCT.1.D/E command risk.
- **`noise_penalty`** (generated-dir suppression) — this is index/filter tuning,
  i.e. PRODUCT.1.C scope, not a score term. Defer to 1.C.

### Done criteria

- Audit/formula documented; live vs deferred terms explicit (this section).
- In `:global` mode, a file under the current `project_root` outranks an
  identical-name file outside it — covered by a deterministic Rust unit test on
  the new score component.
- `ScoreBreakdown` + `RankTooltip` show the workspace component; `rank.workspace`
  localized in both locales.
- New unit test(s) for the workspace component; existing recency/frequency tests
  still pass. `npm run build` + `npm run lint` + `cargo test` + `cargo clippy -- -D warnings` green.

### File map

- `src-tauri/src/models/action.rs` — `ScoreBreakdown` gains `workspace_boost`.
- `src-tauri/src/handlers/search.rs` — `apply_rank_boost` computes the boost from
  resolved `project_root`; thread root through `base_results_to_ui_items` /
  provider calls.
- `src/types/search.ts` — mirror `workspace_boost` on `ScoreBreakdown`.
- `src/shared/components/RankTooltip.tsx` — render workspace row + total.
- `src/i18n/{zh-TW,en-US}.ts` — `rank.workspace`.
- Tests: `handlers/search.rs` or `managers/search_manager.rs` unit test for the
  workspace component.

### ADR call

No new ADR for 1.A: adding one more bounded, additive term to an already-additive
score model is consistent with the un-ADR'd recency/frequency boosts. **If** a
later batch reworks ranking into weighted/normalized scoring (a real algorithmic
change per governance §7), that needs an ADR first.

## PRODUCT.1.B–H — not yet scoped

`B` unified-result contract audit, `C` workspace-aware search (incl. noise
suppression), `D` project command discovery, `E` terminal workflow, `F` file
actions/preview, `G` developer utilities, `H` keyboard/perf gate. Scope each batch
here (primary vs simplification-only) before coding, per the task-persistence rule.
Risky sub-items (destructive file ops, kill-port, shell handoff) still pass their
own approval/ADR gates.
