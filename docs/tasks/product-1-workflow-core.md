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

## PRODUCT.1.A — Ranking Baseline — DONE (2026-06-05, unit level)

Shipped: `workspace_boost` (+20 for file/folder/app under the active
`project_root`) and `config_boost` (+6 for README/manifests/config files) as two
new additive `ScoreBreakdown` terms, surfaced in `RankTooltip` (rows shown when
non-zero) with `rank.workspace` / `rank.config` localized. Pure scoring helpers
live in `handlers/search/ranking.rs` (6 unit tests incl. the headline
"in-workspace file outranks same-name outside" case); `apply_rank_boost` resolves
`project_root` once and adds both terms. Validation: 482 cargo tests + clippy
clean; frontend build/lint + 180 vitest (2 new RankTooltip cases). No ADR
(additive term). Deferred terms (action_success/risk/noise) untouched.

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

## PRODUCT.1.B — Unified Result Contract Audit — DONE (2026-06-05)

Audit outcome: **the contract is already uniform and functional.** Every result
source crosses IPC as `UnifiedResult` via `From<UiSearchItem>` (search rows) and
`From<BuiltinCommandResult>` / `From<SearchResult>` (other producers). The shim
preserves a stable `id` (`item_ref.id`), `kind`, `path`, `score` + `breakdown`,
exactly one primary `ActionChip`, lazy `preview` (`PreviewPayload::None` until the
user expands), and `secondary_action_count`. Risk is `ConfirmRequirement::none()`
on search rows **by design** — risk lives on actions / capability chips, not rows.

Done this batch:

- **Per-source-kind test coverage** (done-criterion): new
  `ui_search_item_shim_covers_every_source_kind` in `models/unified_result.rs`
  asserts all 8 `ResultKind`s (App/File/Folder/Command/Note/History/Model/Memory)
  map with stable id, preserved kind/path/score, one primary action, no row-level
  confirm.
- **Audit recorded** (this section).

Confirmed clean (no change needed):

- Source-specific frontend branches in `utils/secondaryActions.ts` are legitimate
  per-kind *actions* (open/reveal/rename for files; note open), not duplicated
  behavior. The memory-row paste short-circuit is a real distinct capability.

### Deferred finding (not a regression)

The `From<UiSearchItem>` shim wraps **every** source as `ResultSource::File { kind,
path }`, so the `ResultSource::BuiltinCommand` / `Other` variants are unused by the
search path. Reworking `ResultSource` into proper per-source variants is a **data
contract change** (governance §7) that ripples through `unifiedToLegacy` /
`unifiedKindOf` / `unifiedBucketKey` and needs an ADR — out of scope for the
audit. Kept as a documented deviation; revisit if a source needs semantics the
`File`-wrapper cannot express.

## PRODUCT.1.C — Workspace-Aware Search + Noise Suppression — DONE (2026-06-05)

Shipped (developer chose **demote -30**): `noise_penalty` term — file/folder
results inside a generated/dependency dir (denylist: `node_modules`, `target`,
`dist`, `build`, `.git`, `.next`, `.nuxt`, `.venv`, `venv`, `__pycache__`,
`.cache`, `coverage`, `.gradle`, `.idea`, `.svn`, `.tox`, `.pytest_cache`,
`.mypy_cache`) get `-30`, demoting them below clean matches while staying
reachable. Segment-exact match (`mytarget` does not trip `target`); file source
only. Surfaced in `ScoreBreakdown.noise_penalty` + `RankTooltip` (`rank.noise`,
rose, shown when non-zero). Workspace coverage + switch-without-restart confirmed
already working in the audit (no code). Validation: 486 cargo tests + clippy
clean; build/lint + 181 vitest (3 new noise/ranking cases incl. the "clean
src/index.js outranks node_modules same-name" regression fixture). No ADR.



### Audit

- **Workspace coverage already largely works.** `resolve_workspace_filter`
  restricts file/folder/app results to `current().project_root` (`:global`
  escapes), and 1.A's `workspace_boost` ranks the in-workspace copy first. The
  workspace root is read per search, so **switching workspaces updates context
  without restart** (no code needed).
- **Noise is the real gap.** The `IgnoreWalk` fallback already respects
  `.gitignore` + hidden dirs (skips `node_modules`/`target`/`.git`), but the
  **primary providers — Everything (Windows) and Tantivy — do not filter
  generated dirs**, so on the main dev target `node_modules`/`target`/`dist`
  hits can dominate the first page. This is exactly 1.A's deferred `noise_penalty`.

### Primary (this batch)

1. **Generated-dir noise control** (headline; provider-agnostic). A denylist of
   generated path segments (`node_modules`, `target`, `dist`, `build`, `.git`,
   `.next`, `.venv`, `__pycache__`, `.cache`, `coverage`, `out`, `.gradle`, …)
   applied to file/folder results after provider merge. **Default policy:
   demote** — subtract a `noise_penalty` so noisy hits fall below clean results
   but stay reachable (never silently hidden). Surface `noise_penalty` (negative)
   in `ScoreBreakdown` + `RankTooltip`.
2. **Regression fixtures**: duplicate-filename-across-workspaces (in-workspace
   wins via `workspace_boost`) and noise demotion (clean `src/index.js` outranks
   `node_modules/.../index.js`).

### Open decision (gates implementation)

- **Demote vs hide** generated-dir hits. Recommend **demote** (safe, reachable,
  additive like 1.A → no ADR). Hide is a harder filter (removes rows) — cleaner
  first page but can surprise; would still not need an ADR (result-policy, not a
  data contract) but is more aggressive.
- `noise_penalty` magnitude (proposed `-30`, enough to sink below clean
  `base`+boosts but not absurd).

### Simplification-only / deferred (do NOT block 1.C closure)

- **Query-aware exemption** (don't penalize when the query itself names a noisy
  segment, e.g. typing `node_modules`). Needs the cleaned query threaded into
  scoring — deferred refinement; MVP demotes unconditionally (still reachable).
- **Indexer-level exclusion** (teach Tantivy's walk / Everything to not index
  generated dirs). Everything is a system index we don't own; the post-merge
  penalty covers the user-visible need. Defer.
- **Workspace-identity badge** (show which project a row belongs to when the same
  file exists in multiple workspaces). UI polish; `workspace_boost` already ranks
  correctly. Defer.

### File map (once go'd)

- `handlers/search/ranking.rs` — pure `noise_penalty(source, path) -> i64` + tests.
- `models/action.rs` + `types/search.ts` — `ScoreBreakdown.noise_penalty`.
- `handlers/search.rs::apply_rank_boost` — subtract the penalty into score + breakdown.
- `RankTooltip.tsx` + i18n `rank.noise`.

### ADR call

No ADR for the demote (additive bounded penalty, same class as 1.A). Revisit only
if we move to indexer-level exclusion (changes the indexing model, governance §7).

## PRODUCT.1.D–H — not yet scoped

`D` project command discovery, `E` terminal workflow, `F` file actions/preview,
`G` developer utilities, `H` keyboard/perf gate. Scope each here before coding.
Risky sub-items (destructive file ops, kill-port, shell handoff) still pass their
own approval/ADR gates.
