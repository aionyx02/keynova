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

- Feature-first with safety-first runtime lifecycle.
- Retrieval-first docs workflow.
- Markdown remains source of truth without full-doc prompt dumping.

## Current Focus

- P0 Agent Completion Baseline COMPLETE (2026-05-13).
- P1 Workflow MVP COMPLETE (2026-05-13).
- P2 Dev Workflow Pack COMPLETE (2026-05-14).
- Mainline shifted to runtime stability track:
  - PERF.1 Low Memory Background Mode
  - PERF.2 Search Execution Bound
  - PERF.3 Heavy Feature Lazy Runtime
- P3 Context Compiler Lite remains planned after PERF track.
- FEAT.11 Learning Material Review remains blocked.

## Important Constraints

- LLM cannot directly operate unrestricted system commands.
- Approval-gated execution remains mandatory.
- Generic shell tool remains blocked until platform sandbox hardening is complete.
- "Background Core < 100 MB" excludes active WebView, loaded local LLM model memory, PTY terminal sessions, monitoring streams, and index rebuild tasks.

## Last Confirmed Progress

- P2.A: `dispatch_git_status` hardened with workspace scope, timeout, output bound, and preview field.
- P2.B: 4 approval-gated dev tools added: `dev.cargo_test`, `dev.cargo_check`, `dev.npm_build`, `dev.npm_lint`.
- P2.C: `dev.explain_compiler_error` tool added with bounded structured extraction.
- 196 tests passing with 1 pre-existing unrelated failure (`note_lazyvim_missing_nvim_returns_inline_guidance`).
- 2026-05-14: task planning updated from feasibility review (`Keynova_Agent_Architecture_Tasks_Review.docx`) and normalized into `docs/tasks/{active,backlog,blocked}.md`.

## Next Step

1. Keep TD.5.A verify baseline as gate for incoming PERF work.
2. Start PERF.1 implementation slices in execution order (prewarm off, startup index lazy, setup TTL, keep_alive policy).
3. Keep FEAT.11 at planning/ADR level only until blockers clear.

## Known Risks

- If lifecycle refactor lands without measurement scripts, memory goals cannot be validated.
- If preview/read boundaries are mixed, private file exposure risk increases.
- If docs are not synchronized during refactor, task-state drift will reappear.
