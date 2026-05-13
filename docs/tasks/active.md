---
type: task_index
status: active
priority: p0
updated: 2026-05-13
context_policy: always_retrievable
owner: project
tags: [feature-first, agent, workflow]
---

# Active Tasks

## Strategy

Feature First, Refactor Second.

Current mainline is:
P0 Agent Completion Baseline -> P1 Workflow MVP / Command Chaining -> P2 Dev Workflow Pack -> P3 Context Compiler Lite.

TD tasks are important but are not the current mainline unless they block P0-P3 execution.

## P0 - Agent Completion Baseline

Goal:
- Move `/agent` from demo behavior to reliable, verifiable task completion.

### P0.A - ReAct Manual Validation

- [ ] JSON file find -> `filesystem.search` result -> LLM extracts path -> `filesystem.read`.
- [ ] Read-only preview flow: LLM reads a small text file and summarizes.
- [ ] Web query full round-trip: SearXNG / Tavily / DuckDuckGo fallback.
- [ ] Denied shell request: missing `__approved` token -> `ToolDenied` observation.
- [ ] `git.status` approval lifecycle: pending -> approve -> execute -> loop continues.
- [ ] Project search path correctness: Everything / Tantivy / SystemIndexer.
- [ ] Stale index warning appears in observation.
- [ ] Rejected approval self-correction: model switches to safer tool path.
- [ ] Final answer grounded in retrieved results (no hallucinated claims).

### P0.B - Agent Regression Baseline

- [ ] No-action run does not misfire unrelated panel/action.
- [ ] High-risk actions remain behind explicit approval.
- [ ] Cancellation lifecycle tests.
- [ ] Reject lifecycle tests.
- [ ] One-shot approval tests.
- [ ] Tool error surfacing tests.
- [ ] Prompt budget truncation tests.
- [ ] Private architecture denial tests.
- [ ] Secret redaction tests.
- [ ] Web-query redaction tests.

### P0.C - Exit Criteria

- [ ] Manual validation checklist complete.
- [ ] Regression baseline passing.
- [ ] Evidence logged in docs and test outputs.

## P1 - Workflow MVP / Command Chaining

Goal:
- Build a minimum usable linear workflow pipeline first, not full graph runtime.

### P1.A - Pipeline Parser

- [ ] Add `src-tauri/src/core/workflow_pipeline.rs`.
- [ ] Parse `|` separated pipeline text.
- [ ] Map each stage to `WorkflowAction`.
- [ ] Reject empty segments with readable parse error.
- [ ] Reject unknown commands with readable parse error.
- [ ] Keep v0 scope: no nested pipe, no branching, no parallel.
- [ ] Validation: `/search foo | ai explain` parses successfully.

### P1.B - Workflow Execute v0

- [ ] Reuse existing linear execution path (`AutomationEngine::execute` family).
- [ ] Add `automation.execute_pipeline` route.
- [ ] Execute stages sequentially.
- [ ] Pass previous stage output into next stage payload where applicable.
- [ ] Stop on first stage failure.
- [ ] Return per-stage execution report (`route`, `status`, `output/error`).

### P1.C - Frontend Command Chaining Entry

- [ ] Detect `|` in `CommandPalette` input.
- [ ] Route piped commands to `automation.execute_pipeline`.
- [ ] Show workflow running state in UI.
- [ ] Show per-stage results in UI.
- [ ] On failure, show failed stage and reason clearly.

## Current Execution Order

P0.A -> P0.B -> P0.C -> P1.A -> P1.B -> P1.C
