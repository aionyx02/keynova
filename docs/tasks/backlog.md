---
type: task_index
status: backlog
priority: p1
updated: 2026-05-13
context_policy: retrieve_when_planning
owner: project
tags: [feature-first, roadmap, refactor-backlog]
---

# Backlog Tasks

## P2 - Dev Workflow Pack

Goal:
- Make daily developer workflows executable in Keynova with safe, approval-gated primitives.

### P2.A - Git Status Tool Hardening

- [ ] Restrict `git.status` to workspace-scoped cwd only.
- [ ] Add approval preview before execution.
- [ ] Bound stdout/stderr size in observation.
- [ ] Add timeout handling and clear timeout error response.

### P2.B - Cargo/NPM Read-only Commands

- [ ] Add `dev.cargo_test`.
- [ ] Add `dev.cargo_check`.
- [ ] Add `dev.npm_build`.
- [ ] Add `dev.npm_lint`.
- [ ] Keep all commands approval-gated and non-destructive.

### P2.C - Explain Compiler Error

- [ ] Extract compiler/runtime errors from command outputs.
- [ ] Add AI explain flow for errors.
- [ ] Keep behavior read-only (no direct file modification).

## P3 - Context Compiler Lite

Goal:
- Give agent/workflow bounded and controllable context without filesystem prompt dumping.

### P3.A - ContextBundle v0

- [ ] Define `ContextBundle` with:
  - `user_intent`
  - `workspace`
  - `recent_actions`
  - `selected_files`
  - `search_results`
  - `token_budget`
- [ ] Build context from managers only (Search/Workspace/History), no recursive filesystem scan.
- [ ] Apply token budget using initial char-count approximation.

### P3.B - Agent Integration

- [ ] Build `ContextBundle` before agent run.
- [ ] Do not inject full filesystem results into prompt.
- [ ] Use bounded preview for large content.
- [ ] Add prompt audit record for context composition.

## TD - Refactor Backlog (Not Mainline)

These tasks are important but execute only when they block P0-P3.

### TD.0 - Minimal Verify Baseline

- [x] Add `check` script in `package.json`.
- [x] Add `rust:test` script in `package.json`.
- [x] Add `rust:clippy` script in `package.json`.
- [x] Add `verify` script in `package.json`.
- [ ] Run `verify` in regular local workflow and CI gate.

### TD.1 - Frontend Boundary Refactor

- [ ] Centralize IPC client and forbid direct `invoke` usage.
- [ ] Extract launcher search/ranking pure domain logic.
- [ ] Split large CommandPalette concerns into dedicated hooks.

### TD.2 - Backend Composition Root Refactor

- [ ] Extract `AppContainer`.
- [ ] Introduce feature module registration boundary.
- [ ] Reduce `AppState::new()` assembly weight.

### TD.3 - Typed IPC and DTO

- [ ] Add typed request/response DTOs for key routes.
- [ ] Reduce raw `Value` parsing in critical handlers.
- [ ] Align frontend route constants and payload types.

### TD.4 - Actor-Like Services

- [ ] Agent approval/cancel channelization cleanup.
- [ ] Search worker message boundary.
- [ ] Terminal actor command boundary.

### TD.5 - Security and Quality Hardening

- [ ] CSP and network allowlist enforcement.
- [ ] Keyboard/panel routing regression coverage.
- [ ] Chunk merge and stale request tests.
