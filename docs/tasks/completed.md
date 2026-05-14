---
type: task_history
status: completed
priority: p3
updated: 2026-05-14
context_policy: archive
owner: project
tags: [history]
---

# Completed Tasks History

## Baseline Completed

- Phase 0-6.7 baseline delivered and merged.
- Agent ReAct foundation, audit boundary, and search/index fallback paths delivered.
- Translation, system monitoring, settings, notes, and terminal core flows delivered.
- ADR set (0000-0027) established.
- Rust test baseline exceeds 150+ cases.

## Documentation and Context Refactor Completed

- [x] Split task sources into `docs/tasks/{active,backlog,blocked,completed}.md`.
- [x] Split memory sources into `docs/memory/current.md` + sessions/archive.
- [x] Added retrieval-first docs router (`docs/index.md`).
- [x] Added docs auto-sync and guardrail (`docs:sync`, `docs:guard`, `docs:refresh`).

## Planning Shift Completed (2026-05-13)

- [x] Reorganized task strategy around Feature First, Refactor Second.
- [x] Moved TD streams into refactor backlog (non-mainline).
- [x] Set current execution order to P0 Agent baseline -> P1 Workflow MVP.

## P0 - Agent Completion Baseline (COMPLETE 2026-05-13)

Goal: Move `/agent` from demo behavior to reliable, verifiable task completion.

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
- [x] Regression baseline passing. (196 tests passing, 1 pre-existing failure unrelated to agent)
- [x] Evidence logged in docs and test outputs. (2026-05-14: 196 tests, P2 complete)

## P1 - Workflow MVP / Command Chaining (COMPLETE 2026-05-13)

Goal: Build a minimum usable linear workflow pipeline first, not full graph runtime.

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

## P2 - Dev Workflow Pack (COMPLETE 2026-05-14)

Goal: Make daily developer workflows executable in Keynova with safe, approval-gated primitives.

### P2.A - Git Status Tool Hardening

- [x] Restrict `git.status` to workspace-scoped cwd only. (`scoped_cwd_for_dev` / canonicalize+starts_with check)
- [x] Add approval preview before execution. (`preview` field in dispatch response)
- [x] Bound stdout/stderr size in observation. (`GIT_STATUS_OUTPUT_LIMIT` = 8 KB, `bound_output`)
- [x] Add timeout handling and clear timeout error response. (poll+kill, `GIT_STATUS_TIMEOUT_SECS` = 10 s)

### P2.B - Cargo/NPM Read-only Commands

- [x] Add `dev.cargo_test`. (`dispatch_dev_cargo_test`, 120 s timeout)
- [x] Add `dev.cargo_check`. (`dispatch_dev_cargo_check`, 120 s timeout)
- [x] Add `dev.npm_build`. (`dispatch_dev_npm_build`, 60 s timeout)
- [x] Add `dev.npm_lint`. (`dispatch_dev_npm_lint`, 60 s timeout)
- [x] Keep all commands approval-gated and non-destructive. (all require `__approved` token)

### P2.C - Explain Compiler Error

- [x] Extract compiler/runtime errors from command outputs. (`extract_compiler_errors`, cargo + eslint formats)
- [x] Add AI explain flow for errors. (`dev.explain_compiler_error` tool, returns structured error list)
- [x] Keep behavior read-only (no direct file modification). (text inspection only)
