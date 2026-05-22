---
type: working_memory
status: active
priority: p0
updated: 2026-05-23
context_policy: always_retrievable
owner: project
---

# Current Project Memory

## Current Strategy

- Treat Keynova as a keyboard-first workflow tool, not an AI chat product.
- Make unified search/result handling the product spine.
- Move AI from product core to a stateless capability layer exposed through inline action chips.
- Keep the docs system retrieval-first: startup state stays small, and detailed plans live in on-demand task files.

## Current Focus

- P0 is the ADR-0029 / AI capability refactor track from `docs/tasks/refactor-ai-capability.md`.
- Active execution order is `REF.0` through `REF.8`; `REF.4` and `REF.5` may overlap after schema boundaries are clear.
- `current.md` and `active.md` are current-state indexes only.
- `completed.md` remains a compact archive index, with detail living in session logs.

## Important Constraints

- AI agents can draft ADRs as `proposed`; the developer must accept ADR-0029 before architecture-changing implementation begins.
- Freeze new feature work before `REF.7` unless it is required by the refactor, fixes a P0 regression, or protects a documented safety boundary.
- LLM-driven execution must stay approval-gated for risky or system-affecting actions.
- Generic shell tool exposure stays blocked until the platform sandbox boundary is complete.
- Background Core memory targets exclude active WebView, loaded local LLM model memory, PTY terminal sessions, monitoring streams, and index rebuild tasks.
- Do not duplicate historical narrative across current memory, active tasks, and completed history.

## Next Step

- `REF.0` / `REF.1` / `REF.2` / `REF.3` 已完成。`handlers/agent/mod.rs` 從 2406 → 615 行 (-74%)，已遠低於 `<600` 觀察期目標。拆出 `core/grounding.rs` + `core/local_context.rs` + `core/dev_runner.rs`（reusable by REF.4）以及 `handlers/agent/{lifecycle, planning, answers, sources, tools}.rs`（planning + answers 標記為 deprecated、待 REF.8 移除）。
- 下一步是 `REF.4`：新增 `core/ai_capability/` 與 `handlers/ai_capability.rs`，把 `explain` / `summarize` / `fix_error` 三個能力做成 stateless single-step 呼叫，並把現有 `core/local_context` + `core/dev_runner` 接進去。`REF.5`（workflow memory）可在 `REF.4` schema 穩定後並行。
- Run `npm run docs:refresh` before commit or handoff.
- Treat guard failures as routing feedback: current state stays here; history goes to sessions.
