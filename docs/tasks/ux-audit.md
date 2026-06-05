---
type: task_plan
status: active
priority: p1
updated: 2026-06-05
context_policy: on_demand
owner: project
tags: [ux, i18n, a11y, frontend]
---

# UX.AUDIT — UI/UX consistency pass

Developer-directed (2026-06-05). Full code-level frontend walkthrough; findings
and completed work are detailed in `docs/memory/sessions/2026-06-05.md`. Branch:
`feature/personal-memory`.

## Done (committed)

- `UX.AUDIT.1` drop stale "AI Chat" naming → inline-AI wording (`cba9fd4`).
- `UX.AUDIT.2` first screen-reader live regions, `aria-live`/`role=status|alert`
  on palette search state + SettingRow/ModelPanel/NoteEditor status (`66ecab9`).
- `UX.AUDIT.3` full i18n conversion — **infra + 5 batches done**:
  - Infra: `src/i18n/format.ts` `fmt(template, vars)` `{token}` interpolation.
  - Batch 1 ModelPanel (`model.*`), Batch 2 SettingPanel/SettingRow (`settings.*`),
    Batch 3 five AI capability cards (`capability.*`), Batch 4 palette secondary
    actions + suggestion footers (`palette.*` + `buildSecondaryActions(result,
    labels)`), Batch 5 SearchResultsList (`search.*` kinds/fallbacks/footer).

## Open

### UX.AUDIT.3 (remaining i18n components)

Convert each to `useI18n` (default zh-TW); add keys to `src/i18n/zh-TW.ts` +
`src/i18n/en-US.ts`; verify `tsc` + `eslint` + `vitest` + `docs:refresh`. Pattern
is established — mechanical. Components (string counts approx):

- [ ] `shared/components/CheatsheetOverlay.tsx` (~19) — `?` overlay, highest count
- [ ] `shared/components/OnboardingTour.tsx` (~12) — first-run tour
- [ ] `features/terminal/TerminalPanel.tsx` (~11)
- [ ] `features/learning/LearningMaterialPanel.tsx` (~10)
- [ ] `features/system-monitor/SystemMonitoringPanel.tsx` (~9)
- [ ] `features/nvim/NvimDownloadPanel.tsx` (~9)
- [ ] `shared/components/PreviewPane.tsx` (~4)
- [ ] `components/FloatingWindow.tsx` (~2) — incl. `×` close, check `aria-label`
- [ ] `shared/components/ErrorBoundary.tsx` (~2)
- [ ] `features/command-palette/CommandResultArea.tsx` (~1)
- [ ] `shared/components/RankTooltip.tsx` (~1)
- [ ] Re-grep `MouseControlOverlay.tsx` (zh-only `title`) + `CapabilityHintLine.tsx`
  for any stragglers after the above.

### UX.AUDIT.4 — UTF-8 BOM cleanup (cosmetic)

9 source files carry a UTF-8 BOM (CalculatorPanel, HistoryPanel, NoteEditor,
TerminalPanel, LearningMaterialPanel, MouseControlOverlay, TranslationPanel,
OnboardingTour, WorkspaceIndicator). Strip BOM for consistency; verify no
encoding regressions.

### UX.AUDIT.5 — garbled-text (`嚙`) root cause — BLOCKED on repro

`SearchResultsList.hasEncodingError('嚙')` masks mojibake titles as "Unavailable
text". All live search-result read paths verified encoding-correct (Everything
wide `…W` APIs + `\u{FFFD}` skip; `.lnk` `file_stem().to_str()` + FFFD guard; fs
walk). No current code path produces `嚙`, so the guard is likely vestigial or
masking stale store data. **Needs a concrete garbled result (raw filename/path)
from the user to locate the source.** Fallback option if unrepro'd: show the raw
path instead of "Path unavailable" so the row stays actionable.

## Non-goals

- No new ADR (pure i18n reuse + additive locale keys).
- Don't touch `current.md`/`active.md` beyond the one-line index entry.