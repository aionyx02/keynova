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

- [ ] `DECOUP.1` Backend scaffolding + calculator pilot. Add
  `core/feature_registry.rs` (or `features/mod.rs`): `AssemblyCtx` (shared infra
  handle), `FeatureRegistrar` (collects handlers / builtins / search hooks /
  settings fragments / `FeatureSpec{flag_key, namespace}`), and a central
  `REGISTRARS` list. Migrate **calculator** (smallest leaf) to a `register(reg,
  ctx)` fn; `build_command_router` calls it via the list while every other
  feature stays on the existing path. Done = calculator works identically, the
  pattern compiles, central files no longer name calculator directly.
- [ ] `DECOUP.2` Frontend scaffolding + calculator manifest. Add
  `FeatureManifest` type + an aggregator that folds `manifests[]` into
  `PanelRegistry` / `GateKey` set / `KIND_BADGE`. Migrate calculator's frontend
  manifest; other features still register centrally (coexist).
- [ ] `DECOUP.3` Roll out leaf backend features, one batch each: `translation`,
  `notes`, `history`, `system` + `system_monitoring`, `nvim`,
  `learning_material`. Each: move its manager construction + handler + builtin +
  spec into its `register` fn; drop its lines from the central files.
- [ ] `DECOUP.4` Derive cross-cutting central lists from specs: `dispatch.rs`
  namespace guard built from registered `FeatureSpec`s (retire the hand-kept
  `NAMESPACE_FEATURE_GUARDS`); `settings_schema` concatenates per-feature
  fragments.
- [ ] `DECOUP.5` Roll out remaining frontend manifests; the aggregator array
  becomes the only per-feature list (retire central `PanelRegistry` /
  `GateKey` / `KIND_BADGE` hand-lists).
- [ ] `DECOUP.6` Cross-cutting backend (`search`, `ai_capability`, `agent`):
  consume shared managers via `AssemblyCtx`; search providers (note/history/
  memory/model/command) register through a registrar hook instead of the
  hard-coded `append_non_file_results` chain. Largest, last.

## Non-goals

- No dynamic/hot-load plugin runtime (ADR-0044 §2 rejected alternative b).
- No behavior, IPC, config, or data-format changes.
- No forced isolation of genuinely-shared managers (config/workspace/event_bus/
  knowledge_store/action_arena stay shared via `AssemblyCtx`).
