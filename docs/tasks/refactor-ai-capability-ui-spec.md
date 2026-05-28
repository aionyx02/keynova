---
type: task_plan
status: proposed
priority: p0
updated: 2026-05-27
context_policy: on_demand
owner: project
tags: [refactor, ai-capability, ui-spec, search-first]
---

# UI Spec for REF.6.B–F (Prefix-Keyword Pattern)

Companion to `docs/tasks/refactor-ai-capability.md`. Authoritative UI
contract for the AI capability sub-batches. This spec **supersedes** the
docx §4.1–4.5 literal mockups for inline-AI invocation; rationale below.

Status: `proposed` — awaiting REF.6.B implementation start.

## 0. Pivot from docx (2026-05-27)

The docx §4.1–4.5 mockups invoke AI capabilities through per-row chips and
auto-detection (NL-query heuristic for `gen_command`, error regex for
`fix_error`, empty-state mount for `suggest_next`). Developer feedback this
session redirected the UI:

> "inline = type `explain <question>` in the palette → answer streams below,
> Gemini-style. Not every folder/file row needs an Explain chip."

Decisions recorded:
1. All 5 capabilities are invoked via **leading keyword prefix** in the palette
   input. No row chips, no auto-detect.
2. When a capability prefix is active, the palette enters **capability mode**
   and the result area is owned by the capability output card (no normal
   search results shown).
3. REF.6.A's per-row Explain chip and `Ctrl+E` binding are **removed**.
   `UnifiedResult` wire format from REF.6.A is retained — it stays useful for
   non-AI action data, just not for chip rendering on rows.

Rationale: prefix-keyword is consistent across capabilities, discoverable
through a single hint line, and avoids the "is this chip relevant to this
row?" cognitive load.

## 1. Prefix Grammar

Single parser, single source of truth.

| Prefix | Capability | Args shape | Empty-arg behavior |
| --- | --- | --- | --- |
| `explain <q>` | `explain` | `{ text: q }` | parser returns null; stay in search mode |
| `summarize <text>` | `summarize` | `{ text }` | parser returns null; stay in search mode |
| `cmd <intent>` | `gen_command` | `{ intent }` | parser returns null; stay in search mode |
| `fix <error>` | `fix_error` | `{ raw_output: error }` | parser returns null; stay in search mode |
| `next` or `next ` | `suggest_next` | `{}` | parser matches; empty trailing args OK |

Rules:
- Prefix match is case-insensitive on the keyword; query body preserved as
  typed.
- Prefix requires either (a) trailing space + non-empty rest, or (b) exact
  match for `next` (the only zero-arg capability).
- No nesting. The first matching prefix wins.
- Backspacing past the prefix returns the palette to search mode in the same
  frame.

Lives in `src/features/command-palette/utils/parseCapabilityPrefix.ts` (NEW).

## 2. Palette Mode Switch

`usePaletteMode(query)` (NEW, `src/features/command-palette/hooks/usePaletteMode.ts`):
- Returns `{ mode: "search" } | { mode: "capability"; id: CapabilityId; args }`.
- When in capability mode, the existing `useSearchBackend` / `useSearchStream`
  must NOT fire (early-return guard inside the hook, plus parent gating).
- Switching modes cancels any in-flight backend call.

Render dispatch in `CommandPalette.tsx`:

```
if (mode.mode === "search") {
  <SearchResultsList ... />          (existing path)
} else {
  <CapabilityResultArea kind={mode.id} args={mode.args} />
}
```

`CapabilityResultArea` (NEW, `src/features/command-palette/CapabilityResultArea.tsx`)
selects the per-shape card:
- `explain` / `summarize` / `fix_error` → `CapabilityAnswerCard` (streaming text)
- `gen_command` → `CapabilityCommandCard` (structured command + confidence + chips)
- `suggest_next` → `CapabilityListCard` (list of suggestion rows)

## 3. Shared Card: `CapabilityAnswerCard`

For streaming-text capabilities. Renders inside the existing palette body
slot (where `SearchResultsList` would have been).

```
┌──────────────────────────────────────────────────────────────┐
│  ✨ <CapabilityLabel> · <model> · <latency>           [×]    │
│                                                                │
│  <Markdown body — streaming, partial-tolerant>                │
│                                                                │
│  ┌──────────┐  ┌─────────────┐                                │
│  │ Copy md  │  │ Save to note│                                │
│  └──────────┘  └─────────────┘                                │
│                                                                │
└──────────────────────────────────────────────────────────────┘
  ↵ Copy   Esc Stop / Close
```

Path: `src/features/ai-capability/CapabilityAnswerCard.tsx` (NEW).

State source:
- `useCapabilityStream` (NEW thin wrapper around existing `useCapability`,
  adds `model`, `startedAtMs`, `firstChunkAtMs`, `completedAtMs`).
- Reuses existing `Markdown` shared component for body rendering.

Streaming presentation:
- Header `latency` ticks live until first chunk (`✨ explain · qwen2.5:7b ·
  0.3s..`); locks to final latency when stream completes.
- Body grows with each `capability.stream.chunk` event; existing `Markdown`
  component is already partial-tolerant (verify in 6.B impl).
- Cancel: `Esc` calls `cancel()`; body shows `Cancelled.` until prefix
  cleared.
- Error: body replaced by red-tinted message; header shows `· error`.

Footer chips (always rendered when state is `complete` or later):
- `[Copy md]` → writes the streamed markdown to clipboard (existing util).
- `[Save to note]` → calls existing note-create IPC (verify exact name in
  6.B impl, TODO marker in code).

No `[Pin]` action. Card unmounts when prefix is removed; no persistence
across mode switches in v1.

## 4. `CapabilityCommandCard` (for `cmd <intent>`)

```
┌──────────────────────────────────────────────────────────────┐
│  ✨ gen_command · qwen2.5:7b · 0.4s · confidence: high  [×]   │
│                                                                │
│    git push origin HEAD                                       │
│                                                                │
│  Rationale: <one-line rationale from capability output>       │
│                                                                │
│  ┌────────┐ ┌──────────────┐ ┌──────┐                         │
│  │ ↵ Run  │ │ Edit before  │ │ Copy │                         │
│  └────────┘ └──────────────┘ └──────┘                         │
│                                                                │
└──────────────────────────────────────────────────────────────┘
```

Path: `src/features/ai-capability/CapabilityCommandCard.tsx` (NEW).

Behavior:
- Confidence display: `low` (< 0.4) renders the row dimmed, `[Run]` requires
  explicit Tab focus before Enter. `medium` (0.4–0.75) is default state.
  `high` (≥ 0.75) auto-focuses `[Run]`.
- `[Edit before]` cycles the generated command back into the palette input,
  selects all, exits capability mode (so user sees normal search of the
  edited command).
- `[Run]` invokes existing terminal-run IPC (verify exact name in REF.6.F
  impl).
- `[Copy]` writes command to clipboard.

No streaming body — `gen_command` returns structured output, so card renders
fully on capability completion.

## 5. `CapabilityListCard` (for `next`)

```
┌──────────────────────────────────────────────────────────────┐
│  ⏱  next · workflow + qwen2.5:7b · 0.6s                [×]    │
│                                                                │
│   ▸  cargo test --package keynova                             │
│        Last run 12m ago · in keynova-main                     │
│                                                                │
│      git status                                                │
│        Last run 35m ago                                       │
│                                                                │
│      ✨ Open search.rs (suggested)                            │
│        3 opens in last hour                                   │
│                                                                │
└──────────────────────────────────────────────────────────────┘
   ↓↑ Navigate  ↵ Run / Open  Esc Close
```

Path: `src/features/ai-capability/CapabilityListCard.tsx` (NEW).

Behavior:
- Output is `Vec<SuggestedNextAction>` from `suggest_next` capability.
- Rows render as standard navigable list (reuses existing keyboard-nav
  pattern from `SearchResultsList` — extract into shared hook in 6.E).
- Items sourced from `workflow_memory` carry no `✨` prefix; items
  promoted by the AI re-ranker get `✨`.
- Enter on a row fires its `action_ref` via existing dispatch.

Replaces docx §4.4 empty-state Recent + Smart split: same data, different
invocation. `next` is opt-in via prefix; empty palette stays empty.

## 6. Keyboard Map (cross-mode)

| Key | Search mode | Capability mode |
| --- | --- | --- |
| `↑` `↓` | move row selection | `CapabilityListCard` only: move suggestion selection |
| `Enter` | open / run focused row | invoke focused card chip (default first chip) |
| `Tab` / `Shift+Tab` | (unbound; reserved) | cycle card footer chips |
| `Esc` | close palette | cancel stream + clear prefix + back to search (chained: first Esc cancels stream, second closes palette) |
| `Shift+Enter` | secondary action menu | (unbound) |

`Tab` claim in capability mode is local to the card; it does not affect
search-mode behavior. Search mode keeps `Tab` reserved.

## 7. Discovery Hint

Empty palette (query.trim() === "" AND no prior result) renders a one-line
hint at the result area top:

```
Try:  explain <q>   ·   cmd <intent>   ·   fix <error>   ·   summarize <text>   ·   next
```

Lives in `src/features/command-palette/CapabilityHintLine.tsx` (NEW).

Hideable via `settings.show_capability_hint` (NEW boolean, default `true`).

## 8. REF.6.A Reuse / Cleanup Inventory

Keep:
- `UnifiedResult` schema and wire format (still used for secondary action
  data + future REF.6.G ConfirmRequirement plumbing).
- `useCapability` hook core (still the IPC bridge; `useCapabilityStream`
  wraps it).
- `search.query` returning `UnifiedResult[]` on chunk events.

Remove:
- Per-row Explain chip rendering in `SearchResultsList` (delete the chip
  `<button>` and its props).
- `onExplain` / `explainLoading` / `explainText` plumbing in palette and
  SearchResultsList.
- `Ctrl+E` key binding in `CommandPalette`.
- `InlineCapabilityReply` component (replaced by `CapabilityAnswerCard`).

Migration path for REF.6.B impl: delete cleanly, then add the new path —
not a parallel build.

## 9. Per-Batch Scope (revised)

### REF.6.B — Prefix dispatcher + explain + summarize end-to-end

Includes:
- §1 parser, §2 mode switch, §3 `CapabilityAnswerCard`.
- Wire `explain` and `summarize` (both use `CapabilityAnswerCard` with the
  same `{ text }` payload).
- Remove REF.6.A row chip + `Ctrl+E` + `InlineCapabilityReply`.
- Add §7 discovery hint line.

Done:
- `explain rust hashmap` streams answer within 5 s on cold model.
- `summarize <paragraph>` returns concise answer.
- Backspacing past `explain ` instantly returns to search results.
- Bug A/B regression unaffected.

### REF.6.C — `gen_command` + `suggest_next` backend (no UI)

Unchanged from prior spec. Adds capabilities + IPC + hooks; no UI in this
batch.

### REF.6.D — `fix <error>` prefix wired to `CapabilityAnswerCard`

Includes:
- Add `fix` to §1 parser.
- Reuse §3 card (no new component).
- Special body rendering: when capability output includes a diff hint,
  render a small inline diff section above the markdown body. Keep simple —
  no apply-patch action in v1 (deferred until a clearer apply path exists).

Done:
- Paste a cargo error after `fix `: card streams a fix suggestion.
- No auto-detect; no terminal watcher.

### REF.6.E — `next` prefix + `CapabilityListCard`

Includes:
- Add `next` to §1 parser (zero-arg).
- §5 `CapabilityListCard`.
- Wire `suggest_next` capability output.
- Extract list-navigation keyboard handling into shared hook reused by
  `SearchResultsList`.

Done:
- Typing `next` shows ranked workflow suggestions within 1 s warm cache.
- Arrow keys navigate; Enter dispatches via existing action_ref.
- Empty result handled gracefully (`No recent workflows`).

### REF.6.F — `cmd <intent>` prefix + `CapabilityCommandCard`

Includes:
- Add `cmd` to §1 parser.
- §4 `CapabilityCommandCard`.
- Wire `gen_command` capability output.

Done:
- Typing `cmd 把當前 branch 推到 origin` returns a `git push origin HEAD`-
  shaped card within 1 s warm cache.
- `[Edit before]` returns command into palette input.
- Low-confidence does not auto-focus `[Run]`.

### REF.6.G / .H / .I — unchanged from prior spec

## 10. Open Questions

1. **`Save to note` IPC name**: REF.6.B implementation verifies exact name
   via grep before wiring. If no existing IPC fits, the chip ships disabled
   with a TODO and a follow-up sub-batch creates the note IPC.
2. **Streaming for `fix_error`**: current capability layer streams text.
   For diff-style fix output, do we stream raw or buffer until complete?
   Decision deferred to REF.6.D start.
3. **`Edit before` UX detail**: clicking `[Edit before]` exits capability
   mode and prefills the palette. Should the prefix be retained (`cmd git
   push origin HEAD`) or stripped (`git push origin HEAD`)? Default
   proposal: strip the prefix so the edited command is normal search /
   direct execution. Confirm in REF.6.F.

## 11. REF.6.B Intra-Batch Slicing

1. Add `parseCapabilityPrefix` parser + unit tests.
2. Add `usePaletteMode` mode-switch hook + tests.
3. Remove REF.6.A row chip + `Ctrl+E` + `InlineCapabilityReply` (clean cut).
4. Add `useCapabilityStream` wrapper with timing fields.
5. Add `CapabilityAnswerCard` + `CapabilityResultArea` dispatch.
6. Wire `explain` + `summarize` prefixes end-to-end.
7. Add `CapabilityHintLine` discovery hint.

Each slice is PR-shippable. Manual `tauri dev` smoke after slice 3 (clean
state) and slice 6 (working end-to-end).