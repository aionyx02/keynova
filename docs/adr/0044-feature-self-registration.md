---
type: adr
status: proposed
priority: p1
updated: 2026-06-04
context_policy: retrieve_only
owner: project
---

# Feature Self-Registration Manifest

**Status:** proposed
**Date:** 2026-06-04
**Decision makers:** AI agent draft; developer acceptance required
**Related documents:**

- `docs/adr/0009-builtin-command-registry.md`
- `docs/adr/0018-app-module-split.md`
- `docs/architecture.md`
- `docs/tasks/feature-decoupling.md`
- `docs/CLAUDE.md`

---

## 1. Context

Feature *logic* is already split (`managers/X_manager.rs`, `handlers/X.rs`,
frontend `features/X/`), but feature *wiring* is centralized and duplicated, so
adding or removing one feature today edits ~10 shared files:

- Backend: `app/state.rs` (`ManagerBundle` struct + `create_managers()` +
  the ~30-call `build_command_router()`), `app/dispatch.rs`
  (`NAMESPACE_FEATURE_GUARDS`), `models/settings_schema.rs`,
  `default_config.toml` `[features]`, `handlers/builtin_cmd.rs`
  (`FEATURE_GUARDS` + builtin registration), `handlers/search/providers.rs`.
- Frontend: `components/panel/PanelRegistry.tsx`,
  `context/FeatureFlagsContext.tsx` (`GateKey` union),
  `features/command-palette/SearchResultsList.tsx` (`KIND_BADGE`), `ipc/routes.ts`.

Removing a feature is therefore surgery across many central files rather than
deleting a folder. FEAT.GATE (2026-06-04) made features *runtime-removable*
(flags hide UI + refuse IPC); this ADR makes them *structurally removable*.
Changing the assembly/registration boundary is an architecture change
(governance §7), so it is recorded here as `proposed`.

## 2. Decision

If accepted, each feature declares its own wiring in one place and the central
assembly iterates a manifest list instead of hand-listing every feature.

**Backend** — a feature module exposes a registration entry point:

```rust
pub fn register(reg: &mut FeatureRegistrar, ctx: &AssemblyCtx) { /* … */ }
```

- `AssemblyCtx` carries the genuinely-shared infrastructure (`config`,
  `event_bus`, `knowledge_store`, `action_arena`, `workspace_manager`, …).
- Inside `register`, a **leaf** feature builds its own private manager and
  registers: its `CommandHandler`(s), any builtin commands, an optional search
  provider hook, an optional settings-schema fragment, and a `FeatureSpec
  { flag_key, namespace }`.
- A single central list (`features::REGISTRARS`) names each feature's `register`
  fn. `state.rs::build_command_router` loops over it; `dispatch.rs` derives its
  namespace→flag guard from the registered `FeatureSpec`s (no separately
  hand-maintained table); `settings_schema` concatenates the fragments.

**Frontend** — each `features/X/` exports a `manifest`:

```ts
export const manifest: FeatureManifest = { panel?, gateKey?, resultBadge?, routes? };
```

A central aggregator builds `PanelRegistry`, the `GateKey` set, and `KIND_BADGE`
from a `manifests` array. Removal = delete the folder + remove one array entry.

**Boundary (honest limitation):** leaf features (calculator, translation, notes,
history, system, system_monitoring, nvim, learning_material) self-register
fully. Cross-cutting consumers that depend on many shared managers (`search`,
`ai_capability`, `agent`) stay assembled centrally but consume shared managers
through `AssemblyCtx` and receive feature contributions (e.g. search providers)
via the registrar rather than hard-coded calls.

Each feature migration is mechanical and behavior-preserving — no IPC routes,
config keys, or data formats change.

Rejected alternatives: (a) lightweight "just tidy the central lists" — does not
make removal local, the stated goal; (b) full dynamic plugin architecture
(hot-load) — large effort and runtime risk against the project's minimal-scope
preference, and unnecessary for compile-time feature removal.

## 3. Consequences

Positive:

- Removing/adding a feature becomes local: delete the module + one manifest
  entry. Central files shrink and stop being merge-conflict hot spots.
- The dispatch namespace guard and settings schema become derived from one
  source of truth (the specs) instead of duplicated hand-lists.

Negative / tradeoffs:

- Adds one indirection layer (the registrar + manifest types).
- Cross-cutting features (search/ai_capability/agent) remain partially central;
  full isolation is not achievable while they share managers.
- The migration touches the central assembly; it is rolled out incrementally
  (one feature per batch) to keep each diff reviewable and behavior-preserving.

## 4. Rollback

- The manifest list *is* the registration, so abandoning the pattern means
  inlining feature `register` fns back into `build_command_router` — a mechanical
  revert with no data/contract change.
- Each feature batch is an isolated commit and can be reverted independently;
  un-migrated features keep working under the existing central wiring during the
  rollout (the two styles coexist).
