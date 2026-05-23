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

- `REF.0` / `REF.1` / `REF.2` / `REF.3` / `REF.4` 已完成。新增的 `core/ai_capability/` 提供 stateless single-step `explain` / `summarize` / `fix_error`，由 `handlers/ai_capability.rs` 以 `capability.*` IPC 暴露；前端 hook 在 `src/features/ai-capability/`。本批未動 `CommandPalette.tsx`、未動 `agent_runtime.rs`、`ai.legacy_agent` 預設仍為 `true`。
- 下一步可並行兩條：(a) `REF.5` workflow memory（knowledge DB schema v4 + `core/workflow_memory.rs`），把 `record` / `suggest` 立起來，capability schema 已預留 `context_hash` optional 入口；(b) `REF.6` 把 `CommandPalette` 切到消費 `UnifiedResult`，把 capability `ActionChip` 掛到結果列熱路徑。
- Live Ollama smoke 已對 `explain` + `fix_error` 跑過（model `qwen2.5:0.5b` cold-start single sample，4.6–5.0s；pipeline 通），ADR-0029 §8 對 `qwen2.5:7b` 的正式 P50/P95 reading 仍待下次：先 `ollama pull qwen2.5:7b`，再 warm-up 後連跑 5 次取中位數，記到 session log；超出 800ms 不視為 blocker（REF.7 才量化）。
- Run `npm run docs:refresh` before commit or handoff.
- Treat guard failures as routing feedback: current state stays here; history goes to sessions.
