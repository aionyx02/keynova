---
type: task_detail
status: active
priority: p1
updated: 2026-06-13
owner: project
tags: [windows, icons, perf, ux]
---

# ICON.NATIVE + UX follow-up batch

Origin: 2026-06-13 session. After the `CREATE_NO_WINDOW` fix (SPAWN.NOWINDOW,
`sessions/2026-06-13.md`) removed the black-console flash + focus-steal crash, the
user asked to also address the remaining UX-affecting items ("都改").

## Primary (blocking close-out)

- [ ] `ICON.NATIVE` (ADR-0056) — replace the per-icon `powershell.exe` extraction in
  `platform/windows/icon.rs` with native `SHGetFileInfo` → `GetIconInfo` →
  `GetDIBits` → `png` crate → base64. Deletes `run_icon_script` + the embedded
  script; output contract (`data:image/png;base64`) and `.b64` disk cache unchanged.
  Pure pixel helpers (`bgra_to_rgba`, mask alpha, PNG encode) unit-tested; FFI path
  needs **user Windows-GUI verification** (icons render, transparency correct, no
  startup CPU spike). New dep: `png`; windows features `Win32_UI_Shell`,
  `Win32_Graphics_Gdi`.

## Secondary (non-blocking; investigated 2026-06-13)

- [~] `PERF.REOPEN` — `EmptyWorkingSet`-on-every-hide (`app/window.rs:57`) trades
  instant reopen for a leaner hidden-state working-set number (PERF.1.FU). **Decision
  (user, 2026-06-13): keep as-is, measure first.** No code change until a real-machine
  reopen-latency number (with/without trim) shows a perceptible hitch; if it does, gate
  the trim to low-memory mode only. Cannot be measured headlessly.
- [x] `STAB.4` — `嚙` mojibake: **no blind fix possible.** Prior audit (`ux-audit.md`
  §UX.AUDIT.5) already verified every live search read path is encoding-correct (Everything
  `…W` APIs + FFFD skip, `.lnk` `file_stem` + FFFD guard, fs walk); no current code path
  produces `嚙`, so the guard is vestigial or masks stale store data. User-facing harm is
  already mitigated by the raw-path fallback. Closing the root cause needs a concrete `嚙`
  filename/path from the user — not reproducible here.
- [~] `UX.AUDIT.2` — focused static UX/perf sweep done 2026-06-13. Core hot paths verified
  healthy (200ms-debounced + cancelable search, concurrent providers w/ 800ms timeout,
  background prescan/warm/index, bounded clipboard history, graceful lock handling; all
  `file.rs` unwraps are test-only). Findings:
  - [x] `F1` (fixed) — `ModelManager::detect_hardware()` spawned `wmic`/`nvidia-smi`/
    `powershell` on every call with no cache; `handlers/ai.rs:94` used the live path, so an
    AI setup check after the `CHECK_SETUP_TTL` lapse stalled ~1–2s on the dispatch thread.
    Fixed by session-memoizing a *successful* probe in ModelManager (zero/failed probe stays
    retryable via `hardware_probe_succeeded`). Preflight's once-per-boot `detect_hardware()`
    now warms the memo for all callers. `cargo test` 463 (+1).
  - [ ] `F2` (deferred, low) — `metadata_payload` (search.rs:623) does a synchronous stat +
    ≤64KB read for the focused-row preview on the dispatch thread; debounced via
    `useFilePreview` so not per-keystroke. Move to a worker / tighten the cap if it ever
    surfaces. Not worth the churn now.
  - Scope caveat: static review only; true per-frame latency needs runtime profiling on the
    real app.

## Done criteria

- `ICON.NATIVE`: `cargo test` + `cargo clippy -- -D warnings` green; powershell fully
  removed from `icon.rs`; user confirms icons render correctly on a real Windows run.
- Secondary items: each either fixed-and-verified or reported with a concrete reason
  to defer; nothing left implied-but-unstated.

## Non-goals

- Higher-resolution icons via system image list (`SHGFI_SYSICONINDEX`) — later.
- Touching the non-Windows icon paths (Linux/macOS unaffected).
