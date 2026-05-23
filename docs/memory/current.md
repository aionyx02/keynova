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

- `REF.0` / `REF.1` / `REF.2` / `REF.3` / `REF.4` / `REF.5` / `REF.6.A` 已完成。`REF.6.A` 把 `search.query` wire format 切到 `UnifiedResult[]`（後端在 `SearchHandler` emit 邊界用 `From<UiSearchItem>` shim 轉），前端 `useSearchStream` + `utils/search` helpers + `SearchResultsList` 全部消費 UnifiedResult；每個 result row 永久顯示 Explain `ActionChip`、`Ctrl+E` 觸發 `useCapability("explain")`、`InlineCapabilityReply` 串流在結果列下方。`SourceMetadata.secondary_action_count: Option<u32>` 為加性 schema 變更（ADR-0030 §4）。`CommandPalette.tsx` god-component 暫不壓行數（化簡目標 drop per [[feedback-task-persistence]]）。
- 下一步是 `REF.6.B`：從 palette hot path 移除嵌入式 `AiPanel` / `TerminalPanel` mount，把 `AiPanel` 降級為 legacy fallback；之後 `REF.6.C` 把 destructive secondary actions 切到 `ConfirmRequirement` 驅動。
- Live Ollama smoke 對 `explain` + `fix_error` 已通（`qwen2.5:0.5b` cold-start 4.6–5.0s），ADR-0029 §8 對 `qwen2.5:7b` 的正式 P50/P95 reading 仍待下次。
- Run `npm run docs:refresh` before commit or handoff.
- Treat guard failures as routing feedback: current state stays here; history goes to sessions.
