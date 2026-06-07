---
type: task_plan
status: completed
priority: p1
context_policy: on_demand
owner: project
tags: [product, ai-capability, v0.7, inline-ai]
updated: 2026-06-06
---

# PRODUCT.2 — v0.7 AI Capabilities That Are Useful

Detail home for the PRODUCT.2 batches. **Unfrozen 2026-06-06** (REF.7.D done +
latency direction finalized: tiered §8 + `qwen2.5:1.5b` reference default).
High-level roadmap: `docs/tasks/product-roadmap.md` §PRODUCT.2 (A–E).

Goal: make the five existing inline capabilities (`explain`, `summarize`,
`fix_error`/`fix`, `gen_command`/`cmd`, `suggest_next`/`next`) genuinely *useful*
for technical workflows — stable output, actionable error help, safe command
suggestion, source transparency, useful next steps — **without** AI becoming the
product center.

## Completion (2026-06-06)

All five batches landed:

- **2.A:** shared compiler/stack/config/long-text/command fixtures, typed
  empty/provider/malformed handling, and primary-output parser-leak invariants.
- **2.D:** additive `CapabilityResponse.sources`, exact prompt-included source
  tracking, compact UI labels, and proposed ADR-0046. Snippets/scores stay
  backend-only; secret sources are omitted.
- **2.B:** `fix_error` returns structured explanation + optional copy-only
  command suggestion, with the shared risk label and no run/edit affordance.
- **2.C:** `gen_command` echoes normalized cwd/shell/os assumptions and shows
  low-risk vs review-required labels. Runtime defaults fill omitted context.
- **2.E:** `suggest_next` suppresses duplicates, unknown targets, entries older
  than 30 days, and history-only rows older than 7 days.

The cross-cutting non-goal remains binding: PRODUCT.2 adds no generated-command
execution path.

## Cross-cutting NON-GOAL (binds every batch)

**Suggest, never execute.** `PRODUCT.1.E` (command execution / terminal handoff)
was deliberately **dropped** (developer decision, copy-only stays). So PRODUCT.2
adds **no run path**: every "command" a capability emits is copy/review-only.
The roadmap's 2.B/2.C wording about "run flow gated by risk" is reframed here —
`risk_tag` is **informational / for display**, not a gate on an execution path
that does not exist. A real run surface would need a fresh proposed ADR
(ref ADR-0027 generic-shell-sandbox, ADR-0022 approval boundary) and is out of
PRODUCT.2 scope. Privacy boundary stays: personal-memory grounding is local-only
(ADR-0043); cloud-provider context injection stays disabled.

## Audit — what already exists (2026-06-06)

The capability layer is already substantial (REF.4 / REF.6). Live today:

- **Output contract** (`core/ai_capability/contract.rs`): `CapabilityOutput`
  = `Text { text }` | `Structured { value }`; `CapabilityResponse { id, output,
  risk_tag }`; typed `CapabilityError` (`UnknownId` / `InvalidPayload` /
  `ProviderError` / `Cancelled` / `UnsupportedAction`). Single-shot, no session.
- **Graceful parse** (`parse.rs::extract_first_json_object` + per-capability
  fallbacks): `gen_command` never surfaces a raw parser failure as the main
  answer — it falls back to the first command-shaped line or empty + low
  confidence + explanatory rationale.
- **`gen_command`** (`capabilities/gen_command.rs`): structured
  `{ command, confidence, rationale }`, `risk_tag_for_command` (safe-prefix
  allowlist → `none`, else `confirm`), `GenCommandCtx { cwd, shell, os }` threaded
  into the prompt, local-context + (local-only) memory grounding, audit.
- **`fix_error`** (`capabilities/fix_error.rs`): Text output with fixed sections
  (Summary / Likely cause / Check / Next step / Optional command); a **read-only**
  re-run path (`RunCommand` limited to `cargo`/`npm`/`pnpm`/`yarn`/`tsc` via
  `dev_runner`, bounded), `RawOutput` (preferred, no exec), and `Apply` →
  `UnsupportedAction`.
- **`suggest_next`** (`capabilities/suggest_next.rs`): already **dedupes** on
  `route::action_label` (HashSet) and avoids `suggest_next` self-pollution
  (PRODUCT.1.H note). Emits replay descriptors, not auto-execution.
- **Frontend cards** (`src/features/ai-capability/`): `CapabilityAnswerCard`,
  `CapabilityCommandCard`, `CapabilityListCard`, `CapabilityMemoryCard`,
  `CapabilityRecallCard` + `types.ts`, each with copy affordances and tests.

### The real gaps PRODUCT.2 closes

1. **Sources are computed but not surfaced (2.D).** Capabilities build
   `Vec<GroundingSource>` for the prompt, but `CapabilityResponse` has **no
   sources field** and the frontend renders none. Users can't tell what local
   material informed an answer.
2. **`fix_error`'s suggested command is prose, not a structured card (2.B).** The
   "Optional command" line is free text, so it isn't a risk-labeled, one-click
   copy card like `gen_command` produces.
3. **`gen_command` doesn't echo its assumptions (2.C).** `cwd`/`shell`/`os` are
   *inputs* to the prompt but are not displayed on the card, so the user can't see
   the context the command assumes.
4. **Fixture breadth + invariant tests (2.A).** Coverage exists per-capability but
   not the full matrix (compiler errors, stack traces, config snippets, long text)
   nor a uniform "card never shows a raw parser failure as the main answer" test
   across all five.

## PRODUCT.2.A — Capability Output Contracts

### Scope / done
- Lock the output contract: every capability returns `Text` or `Structured` (never
  a raw parser-failure string as the main answer), copyable, with a predictable
  card shape.
- Fixture matrix: compiler errors, stack traces, config snippets, long text,
  command-generation requests → each capability has happy-path + malformed-output
  unit tests.
- Fail gracefully on invalid model JSON, provider errors, empty input (typed
  `CapabilityError`, no panic, no raw dump).

### Primary (this batch)
1. Add a shared test fixture module (`core/ai_capability/test_fixtures.rs` or
   per-capability `#[cfg(test)]` data) with the five input classes.
2. For each of `explain` / `summarize` / `fix_error` / `gen_command` /
   `suggest_next`: assert happy path + malformed/empty → graceful typed result,
   and that no raw parser-failure text leaks as the primary answer.
3. Document the contract invariant in `contract.rs` module docs.

### Deferred / simplification-only
- A formal JSON-schema export of `Structured` payloads for "future automation"
  (roadmap mentions it) — defer until a consumer exists.

### Done criteria
- Cards never expose raw parser failures as the main answer (tested).
- Each capability has happy + malformed unit tests. `cargo test ai_capability`
  green; frontend card tests green.

### ADR call
No ADR — hardening existing typed contracts, no boundary/schema-contract change.

## PRODUCT.2.B — Error-To-Action

### Scope / done
- `fix_error` accepts compiler output, stack traces, command failures; returns
  likely cause + repair steps + suggested command.
- Suggested command becomes a **copyable, risk-labeled command suggestion**
  (reuse the `gen_command` card shape), **copy-only** (NON-GOAL: no run).
- No auto-edit / auto-run (already enforced; keep `Apply → UnsupportedAction`).

### Primary (this batch)
1. Have `fix_error` optionally emit a structured "suggested command" alongside the
   text (or a hybrid output) so the command renders as a `CapabilityCommandCard`
   with `risk_tag` shown — reusing `risk_tag_for_command`.
2. Error fixtures: Rust (`E0308` etc.), TypeScript, npm, cargo → assert the output
   yields actionable next steps + a copyable command where appropriate.

### Deferred / simplification-only
- Multi-command repair sequences / patch suggestions — keep single suggested
  command for v0.7.
- Re-run-and-feed-output convenience (the `RunCommand` path exists but stays
  user-initiated, read-only).

### Done criteria
- Rust / TS / npm / cargo fixtures produce actionable next steps; any suggested
  command is copy-only and one confirmation-concept away from the (non-existent)
  run path — i.e. never executed. Tests green.

### ADR call
No ADR — copy-only, reuses existing risk-tag + card. (A *run* path would need a
new ADR; explicitly out of scope.)

## PRODUCT.2.C — Command Generation Display Safety

### Scope / done
- `cmd` returns `command`, `risk`, `rationale` (already) **and surfaces the
  cwd/shell/os assumptions** it was generated under, on the card.
- Risk shown clearly; copy-only flow stays fast. (Roadmap's "run flow gated by
  risk" → display-only here per NON-GOAL.)
- LLM output already treated as data (no action layer consumes it to run).

### Primary (this batch)
1. Echo `GenCommandCtx` (cwd/shell/os) into the structured output (or response
   metadata) so `CapabilityCommandCard` can render an "assumes: cwd=… shell=…"
   line.
2. Surface `risk_tag` prominently on the command card (label + rationale).
3. Tests: assumptions echoed; risk label rendered for safe vs confirm commands.

### Deferred / simplification-only
- Any direct-run / send-to-terminal affordance — **dropped** (PRODUCT.1.E), needs
  a fresh ADR.
- Per-shell command translation (powershell vs bash) beyond passing `shell` to the
  prompt.

### Done criteria
- Generated command card shows command + risk + rationale + cwd/shell/os
  assumptions; copy-only; risk cannot gate a run path because none exists. Tests
  green.

### ADR call
No ADR — display + copy only.

## PRODUCT.2.D — Local Context Source Display

### Scope / done
- Show sources when `explain` / `summarize` / `fix_error` (and `gen_command`) used
  local context.
- Don't read large private content unless explicitly selected / already in the
  active context bundle (current grounding already pulls bounded workspace /
  command / history / model sources).
- Cloud-provider memory/context injection stays disabled (ADR-0043 privacy
  boundary; `allow_memory_grounding` local-only — keep).

### Primary (this batch) — this is the biggest new surface
1. **Thread sources into the response contract**: add a `sources:
   Vec<GroundingSource>` (or a slim display shape) to `CapabilityResponse`
   (`contract.rs`), populated from the `Vec<GroundingSource>` each capability
   already builds. Mirror the TS type in `src/features/ai-capability/types.ts`.
2. **Render a sources line** on `CapabilityAnswerCard` (and command card): a
   compact "based on: <workspace> · <file> · <history>" affordance.
3. Keep it bounded + non-leaky: show source *labels/paths*, not full content;
   respect the existing local-only memory gate.
4. Tests: response carries sources; card renders them; empty sources → no row.

### Deferred / simplification-only
- Click-through to open a source — nice-to-have, defer.
- Per-source relevance scoring display — defer.

### Done criteria
- Users can tell what local material informed an answer; cloud vs local behavior
  follows the documented privacy boundary. Backend + frontend tests green.

### ADR call
Adding a `sources` field to `CapabilityResponse` is an **additive** change to a
public IPC contract. Per governance §7 (public data-contract change), **flag for
an ADR check before implementation** — likely a short proposed ADR (or an
amendment note to ADR-0029/0030) since it is additive and display-only. Confirm
with developer at 2.D start.

## PRODUCT.2.E — Next Suggestions

### Scope / done
- Use recent workspace actions + command success + context hash to suggest next
  steps (REF.5 workflow memory already feeds this).
- Keep suggestions as replay descriptors, not auto-execution (already true).
- De-duplicate noise and stale suggestions (dedup already exists; add staleness).

### Primary (this batch)
1. Staleness / noise pass on `suggest_next`: drop suggestions whose target no
   longer resolves (e.g. deleted path), age out very old entries, keep the
   existing `route::action_label` dedup.
2. Empty / low-query palette state shows useful `next` actions (mount already
   exists per PRODUCT.1.H; verify usefulness with the staleness pass).

### Deferred / simplification-only
- Learned ranking weights / per-context personalization — defer; keep the REF.5
  recency/frequency ordering.

### Done criteria
- Empty/low-query palette can show useful next actions; suggestions never execute
  without explicit user action; stale/dup suggestions suppressed. Tests green.

### ADR call
No ADR — refines existing suggestion logic, no new contract/boundary.

## Suggested order
-
`A` (contract + fixtures, de-risks the rest) → `D` (sources; biggest surface, has
the ADR check) → `B` → `C` → `E`. `D` is the highest-value gap; `A` is the safest
warm-up; `B`/`C` are card/display polish; `E` is a focused refinement.

## Validation gates (every batch)

`npm run lint` + `npm run build` + `cargo test` + `cargo clippy -- -D warnings` +
`npm run docs:refresh` green. New backend behavior → unit tests next to the
capability; new card UI → vitest. Live-Ollama checks via `bench:ai` /
`live_tests` stay optional (now default `qwen2.5:1.5b`).
