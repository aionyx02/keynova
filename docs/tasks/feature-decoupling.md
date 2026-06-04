---
type: task_plan
status: active
priority: p1
updated: 2026-06-04
context_policy: on_demand
owner: project
tags: [architecture, decoupling, feature-manifest, decoup]
---

# DECOUP — Feature Self-Registration Decoupling

Developer-directed architecture decoupling. Governing decision: ADR-0044
(`proposed`). Branch: `feature/personal-memory` (continues current work).

Goal: each feature owns its wiring so removal = delete the module + one manifest
entry, instead of surgery across ~10 central files (`state.rs`, `dispatch.rs`,
`settings_schema.rs`, `default_config.toml`, `builtin_cmd.rs`, `search/providers.rs`,
`PanelRegistry.tsx`, `FeatureFlagsContext.tsx`, `SearchResultsList.tsx`, `routes.ts`).

Principle: every batch is **behavior-preserving** — no IPC routes, config keys,
or data formats change. The new (manifest) and old (central) wiring **coexist**
during the rollout; un-migrated features keep working. Tests + clippy + lint +
tsc green per batch; one commit per batch.

## Scope boundary

Leaf features self-register fully. Cross-cutting consumers (`search`,
`ai_capability`, `agent`) that share many managers stay centrally assembled but
consume shared infra via `AssemblyCtx` and receive contributions via the
registrar — migrated last.

## Batches (in order)

- [x] `DECOUP.1` Backend scaffolding + calculator pilot. `app/feature_registry.rs`
  added: `AssemblyCtx` (empty for now — grows as shared-dep features migrate),
  `FeatureRegistrar` (handler registration; builtins/search/settings/spec join
  later), `REGISTRARS` list, `register_all(router)`. **calculator** migrated to
  `handlers::calculator::register(reg, _ctx)` (builds its own leaf manager);
  `state.rs` dropped `calculator_manager` from `ManagerBundle`/`create_managers`
  and the explicit `CalculatorHandler` registration, now calls `register_all`.
  Behavior-preserving: 473 tests pass, clippy clean.
- [x] `DECOUP.2` Frontend scaffolding + calculator manifest.
  `features/featureManifest.ts`: `FeatureManifest` type + `FEATURE_MANIFESTS` +
  `manifestPanels()` / `manifestPanelGates()`. `features/calculator/manifest.ts`
  owns the calculator panel (lazy) + its panel gate. `PanelRegistry.tsx` and
  `usePalettePanels.ts` dropped calculator and spread the manifest maps. (Result
  badges / routes join in DECOUP.5.) Behavior-preserving: lint + tsc clean, 170
  vitest pass.
- [x] `DECOUP.3` Roll out backend features. `translation` + `system` (leaf —
  build own manager in `register`), `notes` / `history` / `learning_material`
  (handler self-registers; shared `note_manager`/`history_manager`/`workspace_manager`
  come from `AssemblyCtx`), `nvim` + `system_monitoring` (no manager; shared
  `event_bus`/`config`). `AssemblyCtx` grew the 5 shared handles; `register_all`
  takes `&ctx`. `state.rs` dropped 7 registrations + the 2 leaf managers from
  `ManagerBundle`/`create_managers` + 9 imports. Builtin-command + `FeatureSpec`
  migration deferred (builtins stay central; spec lands with DECOUP.4).
  Behavior-preserving: 473 tests pass, clippy clean.
- [ ] `DECOUP.4` Derive cross-cutting central lists from specs: `dispatch.rs`
  namespace guard built from registered `FeatureSpec`s (retire the hand-kept
  `NAMESPACE_FEATURE_GUARDS`); `settings_schema` concatenates per-feature
  fragments.
- [x] `DECOUP.5` Frontend panel rollout. `features/<x>/manifest.ts` for
  translation / notes / history / system / system-monitor / nvim (panel + gate).
  `PanelRegistry.tsx` now hand-lists only `setting` + `model` (bootstrap-exempt)
  and spreads `manifestPanels()`; `usePalettePanels` `PANEL_FEATURE` =
  `manifestPanelGates()` (no hardcodes left). (Result-badge `KIND_BADGE` for the
  memory kind stays central — single AI-owned row, not worth a manifest field
  yet.) Behavior-preserving: lint + tsc clean, 170 vitest pass.
- [ ] `DECOUP.6` Cross-cutting backend (`search`, `ai_capability`, `agent`):
  consume shared managers via `AssemblyCtx`; search providers (note/history/
  memory/model/command) register through a registrar hook instead of the
  hard-coded `append_non_file_results` chain. Largest, last.

## Non-goals

- No dynamic/hot-load plugin runtime (ADR-0044 §2 rejected alternative b).
- No behavior, IPC, config, or data-format changes.
- No forced isolation of genuinely-shared managers (config/workspace/event_bus/
  knowledge_store/action_arena stay shared via `AssemblyCtx`).
