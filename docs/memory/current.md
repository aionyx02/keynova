---
type: working_memory
status: active
priority: p0
updated: 2026-05-20
context_policy: always_retrievable
owner: project
---

# Current Project Memory

## Current Strategy

- Keep the docs system retrieval-first: read the router, current memory, and active tasks before pulling any larger reference.
- Keep always-retrievable project docs small and state-oriented.
- Route execution history, debugging notes, and root-cause narratives to `docs/memory/sessions/`.
- Use guard scripts to enforce document size, schema, and frontmatter policy instead of relying on manual discipline.

## Current Focus

- Docs governance v1 is the active workflow baseline.
- `current.md` and `active.md` are current-state indexes only.
- `completed.md` is a compact archive index, with detail living in session logs.
- `backlog.md` is on-demand planning context, not startup context.

## Important Constraints

- LLM-driven execution must stay approval-gated for risky or system-affecting actions.
- Generic shell tool exposure stays blocked until the platform sandbox boundary is complete.
- Background Core memory targets exclude active WebView, loaded local LLM model memory, PTY terminal sessions, monitoring streams, and index rebuild tasks.
- Do not duplicate historical narrative across current memory, active tasks, and completed history.

## Next Step

- Use `npm run docs:new-session` before recording detailed work notes.
- Run `npm run docs:refresh` before commit or handoff.
- Treat guard failures as routing feedback: current state stays here; history goes to sessions.
