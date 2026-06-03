---
type: task_plan
status: active
priority: p1
updated: 2026-06-03
context_policy: on_demand
owner: project
tags: [ai-capability, personal-memory, mem]
---

# MEM.1 — Personal Memory Layer

Developer-directed AI-workflow optimization. Governing decision: ADR-0043
(`proposed`). Branch: `feature/personal-memory` (forked from `main`).

Goal: a locally-stored, AI-organized personal memory. Capture personal info /
key points, store on-device, recall from the palette, and (local model only)
ground future answers in it.

## Decisions (settled with developer)

- Personalization injection is **local-model-only** (privacy boundary): inject
  stored memory into `explain` / `fix_error` / `gen_command` prompts only when
  the resolved provider is local (ollama). Cloud providers never auto-inject.
- Surfaces: `remember <info>` prefix (AI organizes), `recall <query>` prefix +
  card, and memories mixed into normal search results. No "save to memory" chip
  on answer cards.
- Store: reuse `agent_memories` with `scope = "personal"` (no schema migration).
  Table rename deferred.

## MEM.1.A — Capture + Recall MVP  (done at unit level)

Backend:

- `core/ai_capability/parse.rs` (new): shared `extract_first_json_object`
  promoted out of `gen_command` so both it and `remember` reuse it.
- `registry.rs`: `CapabilityId::Remember` (audit) + `Recall` (no audit, no LLM).
- `capabilities/remember.rs` (new): LLM → `{title, content}` (robust JSON parse
  with original-note fallback) → `try_store_agent_memory(scope=personal)`.
- `capabilities/recall.rs` (new): local read + term ranking, mirrors
  `suggest_next`; empty query returns recent memories.
- `mod.rs` match arms; handler `capability.list` count 5 → 7.

Frontend:

- `parseCapabilityPrefix.ts`: `remember` / `recall` text prefixes.
- `types.ts`: `CapabilityId` + payload/output types + `parseRememberOutput` /
  `parseRecallOutput`.
- Hooks `useRemember` / `useRecall`; cards `CapabilityMemoryCard` /
  `CapabilityRecallCard`; `CapabilityResultArea` + `CommandPalette` wiring;
  hint line.

Tests: registry/handler counts; `remember` store roundtrip + fallback; `recall`
ranking + empty-query; frontend prefix + parser tests.

## MEM.1.B — Local-model personalization + prompt budget  (pending)

- `local_context::push_memory_sources(query, sources)` reading `scope=personal`.
- `CapabilityDeps` grounding flag set from `AiRuntimeConfig.provider == Ollama`
  in `handlers/ai_capability.rs`; called from `explain` / `fix_error` /
  `gen_command` only when set. Test: cloud provider injects nothing.
- `prompt.rs`: over-budget fallback trims lowest-priority sources / truncates
  per source instead of dropping all context (current 1400-char wholesale drop).

## MEM.1.C — Memory in search results  (pending; largest surface, can defer)

- Memory search provider emitting a `memory` `UnifiedResult` kind; frontend
  result-row handling (icon, primary action = expand/paste).

## Non-goals

- No chat-first / agent revival (per `ai_as_capability`).
- No new memory table or migration in MEM.1.
- No cloud-provider memory grounding.
