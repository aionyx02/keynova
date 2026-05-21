---
type: agent_policy
status: active
priority: p1
updated: 2026-05-21
context_policy: on_demand
owner: project
---

# AI Agent Governance And ADR Policy

Applies to project governance, ADR workflow, and documentation update behavior.

## 1. Core Principle

Safety > correctness > rollbackability > testability > performance > speed.

## 2. Retrieval-First Documentation Rule

- Do not inject all docs into prompt at once.
- Start from `docs/index.md`, then read `docs/memory/current.md` and `docs/tasks/active.md`.
- Retrieve additional files only by task intent.
- Use the smallest relevant heading section.
- Do not treat `docs/tasks/completed.md`, `docs/memory/sessions/*`, or `docs/memory/archive/*` as current instruction.

## 3. ADR Authority Rule

- AI can create ADRs with status `proposed` only.
- AI cannot mark ADRs as accepted.
- Implementation that changes architecture, security, or public data contracts must wait for developer acceptance.

## 4. Documentation Conflict Resolution

When docs conflict, use this order:

1. Explicit developer instruction in the current conversation
2. `docs/tasks/active.md`
3. `docs/memory/current.md`
4. Accepted ADRs in `docs/adr/*`
5. `docs/security.md`
6. `docs/architecture.md`
7. Session or archive logs

## 5. Mandatory Documentation Sync

After meaningful changes, update the smallest matching document:

- Current strategy, focus, or constraint -> `docs/memory/current.md`
- Open task queue or phase status -> `docs/tasks/active.md`
- Detailed implementation notes, debugging narrative, command output, or root cause -> `docs/memory/sessions/YYYY-MM-DD.md`
- Completed task summary -> `docs/memory/sessions/YYYY-MM-DD.md` using `## COMPLETED: TASK_ID - summary`
- Future work -> `docs/tasks/backlog.md`
- Blocked or approval-gated work -> `docs/tasks/blocked.md`
- Architecture/security/testing behavior -> the matching reference doc
- Decision boundary -> `docs/adr/*` plus `docs/decisions.md`

Before commit, run:

```bash
npm run docs:refresh
```

## 5a. Completed Task Archive Rule

Completed task details belong in session logs. `docs/tasks/completed.md` is an index only.

When a task group is complete:

1. Add a single session marker: `## COMPLETED: TASK_ID - short summary`.
2. Put detailed notes under the same session file's `Detailed Notes` section.
3. Remove the completed group from `active.md` or `backlog.md`.
4. Run `npm run docs:completed-regen` when the completed index needs refresh.
5. Run `npm run docs:refresh`.

Do not copy full completed task sections into `completed.md`.

## 5b. Documentation Bloat Prevention

Before writing to any `.md` file, answer three questions:

1. Is the target file marked `context_policy: always_retrievable`?
2. Is the content current state or historical narrative?
3. Does the content already exist elsewhere?

Strict routing:

- Strategy, focus, or durable constraint -> `docs/memory/current.md`
- Open task and task status -> `docs/tasks/active.md`
- Debugging narrative or root-cause analysis -> `docs/memory/sessions/YYYY-MM-DD.md`
- Completed task summary -> session `## COMPLETED:` marker; regenerated into `completed.md`
- Edge case as reusable structured row -> `docs/testing-edge-cases.md`
- Edge case as story or incident -> session log, with a short reference if needed
- Architectural decision -> `docs/adr/NNNN-*.md`

Forbidden in `current.md` and `active.md`:

- `Recent Execution Notes`
- `Last Confirmed Progress`
- `Session History`
- `Detailed Notes`
- `Bugfix Round`
- Dated headings or dated bullet narratives

Size discipline:

- `docs/memory/current.md` must stay under 5 KB.
- `docs/tasks/active.md` must stay under 5 KB.
- If an edit would exceed a limit, move narrative to a session file first.
- When in doubt, default to `docs/memory/sessions/YYYY-MM-DD.md`.

## 5c. Workbench Shadow State Rule

`docs/state/*` and `docs/workbench/*` are generated convenience views for planning, sorting, ADR previews, and UI simulation.

- Markdown remains authoritative for agent instructions, task status, ADR status, and conflict resolution.
- Workbench files must not be treated as startup context or mandatory retrieval targets.
- If Markdown, JSON, and HTML disagree, follow Markdown first and regenerate the workbench.
- Do not block P0 implementation only because a workbench view is missing or stale; run `npm run docs:workbench-sync` or `npm run docs:refresh` to refresh it.
- Generate or refresh concrete workbench previews when the user is choosing an ADR direction, task order, or UI layout.
- Put AI-generated suggestions and user-confirmation questions in `docs/state/workbench-suggestions.json`; the generated HTML should expose them as user-confirmed options, preferably checkboxes or radio choices.
- When the user returns a proposal copied from HTML, validate it against Markdown first, then update only the smallest matching Markdown source.

## 6. Auto-Update Guardrail

`docs:refresh` runs metadata sync and the docs guard suite:

```bash
npm run docs:workbench-sync
npm run docs:sync
npm run docs:guard-size
npm run docs:guard-schema
npm run docs:audit-frontmatter
npm run docs:narrative-check
npm run docs:guard
```

Guard failures are routing feedback, not optional warnings.

## 7. ADR Trigger Checklist

Create an ADR before implementation when any apply:

- New or removed core dependency
- Async model, IPC, queue, or actor boundary changes
- Security boundary or path/network permission changes
- Data format, schema, or public contract changes
- Major algorithmic, path, or indexing model changes

## 8. Minimal Agent Workflow

1. Classify user intent.
2. Retrieve minimal relevant docs.
3. Implement the smallest safe change.
4. Run relevant tests/checks.
5. Update docs via the routing rules above.
6. Run `npm run docs:refresh`.
7. Report changed file paths and remaining risk.
