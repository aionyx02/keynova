---
type: working_memory
status: active
priority: p0
updated: 2026-05-14
context_policy: always_retrievable
owner: project
---

# Current Project Memory

## Current Strategy

- Feature-first, refactor-second.
- Retrieval-first docs workflow.
- Markdown remains source of truth, but no full-doc prompt dump.

## Current Focus

- P0 Agent Completion Baseline (manual validation + regression baseline).
- P1 Workflow MVP / Command Chaining (linear pipeline first).
- P2 Dev Workflow Pack and P3 Context Compiler Lite as next feature layers.

## Important Constraints

- LLM cannot directly operate unrestricted system commands.
- Approval-gated execution remains mandatory.
- Generic shell tool remains blocked until platform sandbox hardening is complete.

## Last Confirmed Progress

- P0 Agent Completion Baseline COMPLETE (2026-05-13).
- P1 Workflow MVP / Command Chaining COMPLETE (2026-05-13).
  - `workflow_pipeline.rs`: parse `|`-separated pipeline text, map to `WorkflowAction`, 11 tests.
  - `AutomationEngine::execute_pipeline`: sequential execution with prev-output chaining, 5 tests.
  - `dispatch.rs`: `automation.execute_pipeline` IPC route.
  - `CommandPalette.tsx`: pipeline detection on Enter, running indicator, per-stage results UI.
  - `safety.rs`: 6 new tests for sanitize_external_query + looks_sensitive_path.
  - Fixed bug: `sanitize_external_query` "CLAUDE.md" was uppercase, never matched.
- 186 tests passing. 1 pre-existing unrelated failure (`note_lazyvim_missing_nvim_returns_inline_guidance`).

## Next Step

1. Next mainline: P2 Dev Workflow Pack → define tasks in `docs/tasks/backlog.md`.
2. Investigate `note_lazyvim_missing_nvim_returns_inline_guidance` pre-existing failure if user wants it fixed.

## Known Risks

- If code changes land without task/memory updates, project-state drift occurs.
- Overly large memory/task docs can still cause context pollution if retrieval policy is ignored.
