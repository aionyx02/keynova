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

## Secondary (non-blocking; investigate, fix-or-report)

- [ ] `PERF.REOPEN` — measure overlay re-open latency from `EmptyWorkingSet`-on-hide
  (PERF.1.FU tradeoff). If show latency is materially hurt, gate/remove the trim.
  Pure measurement first; no change without a number.
- [ ] `STAB.4` — `嚙` garbled-path mojibake. Documented blocked-on-repro in
  `active.md`/`ux-audit.md`. Investigate root cause (likely ANSI/UTF-8 codepage
  decode on a Win32 path API); fix if a deterministic repro is found, else keep the
  existing fallback and report findings.
- [ ] `UX.AUDIT.2` — the broader focused UX/perf sweep the user requested. Produce a
  verified findings list (not impression-based) before proposing further changes.

## Done criteria

- `ICON.NATIVE`: `cargo test` + `cargo clippy -- -D warnings` green; powershell fully
  removed from `icon.rs`; user confirms icons render correctly on a real Windows run.
- Secondary items: each either fixed-and-verified or reported with a concrete reason
  to defer; nothing left implied-but-unstated.

## Non-goals

- Higher-resolution icons via system image list (`SHGFI_SYSICONINDEX`) — later.
- Touching the non-Windows icon paths (Linux/macOS unaffected).
