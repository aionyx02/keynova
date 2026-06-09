---
type: task_plan
status: completed
priority: p1
context_policy: on_demand
owner: project
tags: [ref-8, cut, converge, legacy-agent, scope]
---

# REF.8 finish + scope cut + convergence

**Status: completed 2026-06-09** (commits `6f871cf` A, `2d35f53` B, + C convergence;
not yet merged — awaiting developer review). Scope change during execution:
**`notes` was kept** (developer reversed the cut — NoteManager is load-bearing for
learning_material/local_context/search); only **nvim** and **mouse_control** were
cut in batch B. Outcome detail: `docs/memory/sessions/2026-06-09.md`.

Branch: `feature/ref8-cut-parked-converge` (from `main`). Developer-directed
2026-06-09 after dropping PROFILE.3: spend the "converge" energy on deleting dead
weight and sharpening scope.

## A. REF.8 — remove the dormant legacy agent (primary)

Delete the ReAct/agent stack abandoned by ADR-0029 (AI = stateless capability,
not agent). **Surgical**, because two symbols in the agent tree are shared:

- **Preserve:** `models/agent.rs` types `GroundingSource` / `ContextVisibility`
  (used across `ai_capability/*`, `grounding`, `local_context`, `context_bundle`)
  and `core/agent_observation.rs` (`prepare_observation` etc., used by
  `core/preview.rs`). Keep the files; remove only agent-only items.
- **Delete:** `core/agent_runtime.rs` (~1948 L), `handlers/agent/` (~4541 L),
  `models/agent.rs::AgentRun` + agent-only types, the `agent_archive`
  knowledge_store table (schema/sql/worker/`AgentArchiveEntry`/`try_archive_*`),
  and all wiring in `app/state.rs` (AgentRuntime field, archive sink,
  `AgentHandler` registration, `agent.run_history_cap`), `core/mod.rs`
  (`agent_runtime` mod + `AgentRuntime` re-export), `handlers/mod.rs` (`agent`),
  `core/diagnostics.rs`, `models/settings_schema.rs` (`ai.legacy_agent`,
  `agent.run_history_cap`), and `src/components/panel/PanelRegistry.tsx`.
- **ADRs:** mark 0011 / 0016 / 0022 / 0026 superseded by 0029 (`status: 取代`),
  update `docs/decisions.md`.

## B. Cut parked surfaces: `nvim`, `notes`, `mouse_control`

Use the DECOUP seams (delete module + one registrar entry):

- **nvim:** `src/features/nvim/` + `featureManifest.ts` entry; backend
  `handlers/nvim.rs` + `REGISTRARS` entry; `managers/portable_nvim_manager.rs`;
  references in `terminal_manager`, `startup_preflight`, `handlers/feature.rs`.
- **notes:** `src/features/notes/` + `featureManifest.ts` entry; backend
  `handlers/note.rs` + `REGISTRARS` entry; `managers/note_manager.rs`;
  `note_manager` from `AssemblyCtx`; `handlers/builtin_cmd/note.rs` (`note`
  builtin); `note_storage_dir` config in `state.rs`; `note_ids` may stay as inert
  `WorkspaceState` data. (notes/nvim are entangled via "open note in editor" — cut
  together.)
- **mouse_control:** `src/features/mouse-control/`; backend `handlers/mouse.rs` +
  `managers/mouse_manager.rs`; `MouseManager` / `mouse_active` in `state.rs`; the
  `Ctrl+Alt+M` toggle + move/click bindings in `app/shortcuts.rs`; any `mouse.*`
  reference in `automation_engine.rs`.
- Remove their `features.*` flags from `settings_schema.rs` and i18n strings.

## C. Convergence (tracking hygiene)

- `REF.6.H` → mark **done** (feature-first migration; model-manager tab
  consolidation explicitly dropped, not deferred).
- `REF.7` → the `ai.legacy_agent` observation/flag-removal items close *with* A.
- Refresh `current.md` constraints that name the legacy agent / flag.

## Non-goals

- No behavior change to search / capabilities / ranking / profiles.
- Keep `GroundingSource` / `ContextVisibility` / `agent_observation` intact.
- No new features.

## Validation (per phase + final)

```bash
cargo test --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
npm run lint && npm run test && npm run build
npm run docs:refresh
```

Sequence as 3 green commits: A (agent), B (cuts), C (docs/ADRs).
