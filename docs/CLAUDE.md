---
type: agent_policy
status: active
priority: p0
updated: 2026-05-15
context_policy: always_retrievable
owner: project
---

# AI Agent Governance and ADR Policy

Applies to project governance, ADR workflow, and documentation update behavior.

## 1. Core Principle

Safety > correctness > rollbackability > testability > performance > speed.

## 2. Retrieval-First Documentation Rule

- Never inject all docs into prompt at once.
- Always start from:
  - `docs/index.md`
  - `docs/memory/current.md`
  - `docs/tasks/active.md`
- Retrieve additional files only by user intent.
- Use smallest relevant heading section.
- Do not treat `docs/tasks/completed.md` or `docs/memory/archive/*` as current instructions.

## 3. ADR Authority Rule

- AI can create ADR with status `proposed` only.
- AI cannot mark ADR as accepted.
- Implementation that changes architecture/security/data contracts must wait for developer acceptance.

## 4. Documentation Conflict Resolution

When docs conflict, use this order:

1. Explicit developer instruction in current conversation
2. `docs/tasks/active.md`
3. `docs/memory/current.md`
4. Accepted ADR (`docs/adr/*`)
5. `docs/security.md`
6. `docs/architecture.md`
7. Session/archive logs

## 5. Mandatory Documentation Sync

After meaningful changes, update the right file immediately:

- Task status change -> `docs/tasks/*`
- Working strategy/next step -> `docs/memory/current.md`
- Architecture changes -> `docs/architecture.md`
- Security boundary changes -> `docs/security.md`
- Test strategy changes -> `docs/testing.md`
- Decision boundary changes -> `docs/adr/*` + `docs/decisions.md`

### 5a. Auto-Archive Completed Task Groups (mandatory)

A task group is a section header in `backlog.md` or `active.md` whose children are `[ ]` / `[x]` items (e.g. `## PERF.1`, `## AGENT.1`, `## FEAT.11`).

Rule: the moment a group becomes 100% `[x]`, in the **same change-set**:

1. Remove the entire group section from `backlog.md` (or `active.md`).
2. Append the same group to `docs/tasks/completed.md` with a `(COMPLETE YYYY-MM-DD)` suffix on the header; retain per-item implementation hints.
3. Update `backlog.md` "Mainline History" / `active.md` "Current Execution Order" lists to reflect the new state.
4. Re-run `npm run docs:refresh` so `docs/index.md` and frontmatter stay in sync.

Why: prevents `[x]` accumulation in `backlog.md` (which hides real remaining work), preserves a clean delivery history, and keeps `active.md` / `backlog.md` small enough to stay always-retrievable.

Do not:

- Leave a completed group sitting in `backlog.md` "for later cleanup".
- Move only part of a group; either it's fully complete (move the whole section) or partial (stay in place).

## 6. Auto-Update Guardrail

Before commit:

```bash
npm run docs:refresh
```

`docs:refresh` does two things:

1. `docs:sync` updates metadata/index automatically.
2. `docs:guard` blocks commit when code changed but project-state docs were not updated.

## 7. ADR Trigger Checklist

Create ADR before implementation when any apply:

- New/removed core dependency
- Async model / IPC / queue / actor boundary changes
- Security boundary or path/network permissions changes
- Data format/schema/public contract changes
- Major algorithmic/path/indexing model changes

## 8. Minimal Agent Workflow

1. Classify user intent.
2. Retrieve minimal relevant docs.
3. Implement smallest safe change.
4. Run relevant tests/checks.
5. Update docs via `docs:refresh`.
6. Report with file paths and remaining risks.
