---
type: adr
status: proposed
priority: p1
updated: 2026-06-03
context_policy: retrieve_only
owner: project
---

# Personal Memory Capability + Local-Only Grounding

**Status:** proposed
**Date:** 2026-06-03
**Decision makers:** AI agent draft; developer acceptance required
**Related documents:**

- `docs/adr/0029-ai-capability-layer.md`
- `docs/adr/0030-backend-risk-tag-contract.md`
- `docs/security.md`
- `docs/CLAUDE.md`

---

## 1. Context

The developer wants the AI workflow to support a locally-stored, AI-organized
**personal memory**: capture personal info / key points, store them on-device,
recall them from the palette, and — only with a local model — let them
personalize future answers. This extends the ADR-0029 capability layer with two
new capability ids and reuses existing storage:

- The `agent_memories` SQLite table and its CRUD (`try_store_agent_memory`,
  `agent_memories_blocking`, `read_agent_memories`) already exist but are written
  only by the dormant agent (`handlers/agent/lifecycle.rs`). They are reused via
  a new `scope = "personal"` value, so **no schema migration** is required.
- `local_context::LocalContextSearcher` already injects grounding sources into
  capability prompts (workspace/command/note/history/model) but not memory.

Adding capability ids and a grounding source is an ADR-0029/ADR-0030 contract
change (ADR-0030 §4 fixes capability metadata at registration; governance §7
covers public-contract/schema changes), so it is recorded here. This ADR is
`proposed` only; the implementation that ships on `feature/personal-memory` is
gated on developer acceptance.

## 2. Decision

If accepted, Keynova adds a personal-memory loop to the capability layer:

- **Two new capabilities** (`registry.rs`, append-only): `remember` (LLM
  organizes a raw note into `{title, content}`, audit = true) and `recall` (no
  LLM — local `agent_memories` read + term ranking, audit = false, mirroring
  `suggest_next`).
- **Storage:** reuse `agent_memories` with `scope = "personal"`,
  `visibility = "private"`. No new table, no migration.
- **Palette surfaces:** `remember <info>` and `recall <query>` prefixes (parsed
  by `parseCapabilityPrefix`), each with its own card; matching memories also
  surface inline in normal search results as a `memory` result kind (Phase 3).
- **Personalization is local-model-only (privacy boundary):** stored personal
  memory is auto-injected into `explain` / `fix_error` / `gen_command` prompts
  **only when the resolved provider is local (ollama)**. For cloud providers
  (claude / openai) memory is never auto-injected; store and recall still work.
  Enforced in `handlers/ai_capability.rs` by a `CapabilityDeps` flag set from the
  resolved `AiRuntimeConfig.provider`.

Rejected alternatives: (a) a brand-new `personal_memory` table — costs a schema
migration for no behavioural gain over a new scope; (b) always injecting memory
regardless of provider — sends personal data to a third-party cloud model,
contradicting the "store local" intent.

## 3. Consequences

Positive:

- Reuses existing storage + capability pipeline; the change is largely wiring.
- Personal data stays on-device unless the user is running a local model.
- `recall` adds no LLM cost (local read).

Negative / tradeoffs:

- `agent_memories` is shared between the dormant agent (`long_term`) and personal
  memory (`personal`); the scope value is the only separator. A later rename to a
  neutral table name is deferred.
- Memory grounding enlarges prompts, so the capability prompt budget must trim
  per-source instead of dropping all context (handled in Phase 2).
- Capability count in `capability.list` changes from 5 to 7; the frontend twin
  (`types.ts` `CapabilityId`) and any count assertions must track it.

## 4. Rollback

- Remove the `remember` / `recall` registry entries, capability modules, match
  arms, prefixes, hooks, and cards; the `agent_memories` table is unchanged and
  any `scope = "personal"` rows become inert.
- Remove the `push_memory_sources` call sites and the `CapabilityDeps`
  grounding flag to drop personalization; capture/recall can stay or go
  independently.
- No schema or data-format migration to reverse.
