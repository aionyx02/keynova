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
- P2 Dev Workflow Pack COMPLETE (2026-05-14).
  - P2.A: `dispatch_git_status` hardened: workspace scope, 10 s timeout, 8 KB output bound, preview field.
  - P2.B: 4 approval-gated dev tools added: `dev.cargo_test`, `dev.cargo_check`, `dev.npm_build`, `dev.npm_lint`.
    - Shared `run_bounded_dev_cmd` helper: piped I/O threads, poll-with-kill timeout, 64 KB output bound.
  - P2.C: `dev.explain_compiler_error` tool: `extract_compiler_errors` parses cargo + ESLint format, capped at 20.
  - Bonus: fixed pre-existing `unused import Value` in `workflow_pipeline.rs` and clippy `manual_checked_ops` in `portable_nvim_manager.rs`.
- 196 tests passing. 1 pre-existing unrelated failure (`note_lazyvim_missing_nvim_returns_inline_guidance`).

## Next Step

1. Next mainline: P3 Context Compiler Lite → define `ContextBundle` (see `docs/tasks/backlog.md` P3.A).
2. Optionally investigate `note_lazyvim_missing_nvim_returns_inline_guidance` pre-existing failure.

## Known Risks

- If code changes land without task/memory updates, project-state drift occurs.
- Overly large memory/task docs can still cause context pollution if retrieval policy is ignored.
