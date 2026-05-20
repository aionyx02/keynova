---
type: working_memory
status: active
priority: p0
updated: 2026-05-20
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

- `REF.0` / `REF.1` 已完成。
- 下一步是 `REF.2`：把 `CommandPalette.tsx` 拆成 `src/features/command-palette/` 下的 feature-first hooks/components，並對 Bug A/B 路徑做回歸檢查。
- Run `npm run docs:refresh` before commit or handoff.
- Treat guard failures as routing feedback: current state stays here; history goes to sessions.
