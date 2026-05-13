---
type: working_memory
status: active
priority: p0
updated: 2026-05-13
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

- Phase 5 and Phase 6 baseline completed and merged.
- Docs were previously consolidated; now re-splitting for retrieval quality and context budget control.
- Task roadmap is now reorganized to Feature First, Refactor Second.

## Next Step

1. Finish docs auto-sync + guard setup.
2. Execute P0.A -> P0.B -> P0.C.
3. Start P1.A pipeline parser after P0 exit criteria is met.

## Known Risks

- If code changes land without task/memory updates, project-state drift occurs.
- Overly large memory/task docs can still cause context pollution if retrieval policy is ignored.
