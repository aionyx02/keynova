---
type: adr
status: proposed
priority: p1
updated: 2026-06-06
context_policy: retrieve_only
owner: project
---

# Capability Source Display Contract

**Status:** proposed
**Date:** 2026-06-06
**Decision makers:** AI agent draft; developer acceptance requested by PRODUCT.2.D
**Related documents:**

- `docs/adr/0029-ai-capability-layer.md`
- `docs/adr/0030-backend-risk-tag-contract.md`
- `docs/adr/0043-personal-memory-capability.md`
- `docs/tasks/product-2-ai-capabilities.md`

---

## 1. Context

The inline capability layer already builds bounded `GroundingSource` values for
`explain`, `fix_error`, and `gen_command`, but `CapabilityResponse` does not say
which sources actually reached the prompt. Users therefore cannot distinguish a
model-only answer from one grounded in workspace, command, history, note, model,
or personal-memory context.

Returning the existing `GroundingSource` directly would expose snippets, scores,
and visibility metadata that the UI does not need. It could also report sources
that were later dropped by the prompt budget. Adding response metadata is an
additive public IPC contract change, so governance requires an ADR check.

## 2. Decision

Add `sources: CapabilitySource[]` to successful capability responses.
`CapabilitySource` contains only:

- stable `source_id`
- display `source_type`
- display `title`
- optional `uri`

The prompt builder returns the exact bounded source set that it included.
Capabilities derive display sources from that set, not from the larger candidate
list. Snippets, relevance scores, and visibility classifications remain
backend-only. Sources classified as `Secret` are omitted from the response.

The frontend renders a compact, non-interactive "Based on" row only when the
array is non-empty. There is no source click-through in PRODUCT.2.

Rejected alternative: return full `GroundingSource` objects. It is simpler
mechanically but exposes unnecessary private content and can misrepresent prompt
grounding after budget trimming.

## 3. Consequences

- Users can see whether an answer used local workspace or memory material.
- Older frontend builds ignore the additive field; newer builds treat a missing
  field as an empty list.
- Cloud providers still receive no personal-memory grounding under ADR-0043, so
  they cannot produce personal-memory source labels.
- The response contract grows slightly, but remains bounded by the capability
  prompt source limit.
- Source labels are informational only. They do not grant file access or add an
  execution path.

## 4. Rollback

Remove the frontend row and stop serializing `sources`. No database or persisted
data migration is involved. The prompt builder can keep its included-source
tracking internally, so rollback does not affect model behavior.
