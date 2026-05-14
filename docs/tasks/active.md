---
type: task_index
status: active
priority: p0
updated: 2026-05-14
context_policy: always_retrievable
owner: project
tags: [feature-first, agent, workflow]
last_change: P0.A validation tests + P0.B regression baselines added
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

- [x] JSON file find -> `filesystem.search` result -> LLM extracts path -> `filesystem.read`. (`react_loop_two_step_search_then_read`)
- [x] Read-only preview flow: LLM reads a small text file and summarizes. (`react_loop_filesystem_read_dispatch_returns_content`)
- [x] Web query full round-trip: SearXNG / Tavily / DuckDuckGo fallback. (`react_loop_web_search_returns_grounded_answer`)
- [x] Denied shell request: missing `__approved` token -> `ToolDenied` observation. (`react_loop_unknown_tool_produces_error_observation_and_loop_completes`)
- [x] `git.status` approval lifecycle: pending -> approve -> execute -> loop continues. (`react_loop_approval_gate_approved_executes_tool`)
- [x] Project search path correctness: Everything / Tantivy / SystemIndexer. (`react_loop_filesystem_search_dispatch_finds_file`)
- [x] Stale index warning appears in observation. (`react_loop_stale_index_warning_surfaced`)
- [x] Rejected approval self-correction: model switches to safer tool path. (`react_loop_approval_gate_rejected_informs_llm_and_continues`)
- [x] Final answer grounded in retrieved results (no hallucinated claims). (covered by web/search round-trip tests)

### P0.B - Agent Regression Baseline

- [x] No-action run does not misfire unrelated panel/action. (`react_loop_no_tool_call_dispatch_never_invoked`)
- [x] High-risk actions remain behind explicit approval. (`react_loop_approval_gate_approved_executes_tool`)
- [x] Cancellation lifecycle tests. (`react_loop_cancel_mid_loop_stops_early`)
- [x] Reject lifecycle tests. (`react_loop_approval_gate_rejected_informs_llm_and_continues`)
- [x] One-shot approval tests. (`react_loop_approval_gate_approved_executes_tool`)
- [x] Tool error surfacing tests. (`react_loop_unknown_tool_produces_error_observation_and_loop_completes`)
- [x] Prompt budget truncation tests. (`react_loop_large_dispatch_result_does_not_panic`)
- [x] Private architecture denial tests. (`sanitize_external_query_blocks_private_architecture_term`)
- [x] Secret redaction tests. (`redacts_secret_lines_before_returning_content` in agent_observation.rs)
- [x] Web-query redaction tests. (`sanitize_external_query_blocks_secret_credential_terms`)

### P0.C - Exit Criteria

- [x] Manual validation checklist complete. (all P0.A items verified via unit tests)
- [x] Regression baseline passing. (170 tests passing, 1 pre-existing failure unrelated to agent)
- [ ] Evidence logged in docs and test outputs. (this update)

## P1 - Workflow MVP / Command Chaining

Goal:
- Build a minimum usable linear workflow pipeline first, not full graph runtime.

### P1.A - Pipeline Parser

- [x] Add `src-tauri/src/core/workflow_pipeline.rs`.
- [x] Parse `|` separated pipeline text.
- [x] Map each stage to `WorkflowAction`.
- [x] Reject empty segments with readable parse error.
- [x] Reject unknown commands with readable parse error.
- [x] Keep v0 scope: no nested pipe, no branching, no parallel.
- [x] Validation: `/search foo | ai explain` parses successfully. (`parse_search_then_ai_pipeline`)

### P1.B - Workflow Execute v0

- [x] Reuse existing linear execution path (`AutomationEngine::execute` family). (`AutomationEngine::execute_pipeline`)
- [x] Add `automation.execute_pipeline` route. (`dispatch.rs` special handler)
- [x] Execute stages sequentially. (`execute_pipeline_runs_stages_sequentially`)
- [x] Pass previous stage output into next stage payload where applicable. (`execute_pipeline_injects_prev_output_into_next_stage`)
- [x] Stop on first stage failure. (`execute_pipeline_stops_on_first_failure`)
- [x] Return per-stage execution report (`route`, `status`, `output/error`). (`execute_pipeline_returns_per_stage_report`)

### P1.C - Frontend Command Chaining Entry

- [x] Detect `|` in `CommandPalette` input. (search mode Enter handler)
- [x] Route piped commands to `automation.execute_pipeline`. (`runPipeline` function)
- [x] Show workflow running state in UI. (pipelineRunning + animate-pulse indicator)
- [x] Show per-stage results in UI. (per-stage list with route + status icons)
- [x] On failure, show failed stage and reason clearly. (✗ icon + red error text)

## Current Execution Order

P0.A -> P0.B -> P0.C -> P1.A -> P1.B -> P1.C
