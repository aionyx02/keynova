---
type: task_detail
status: active
priority: p1
updated: 2026-06-13
owner: project
tags: [linux, macos, ci, stability, cross-platform]
---

# Linux / macOS stability (phased)

Origin: 2026-06-13. Developer is Windows-only and cannot test Linux/macOS, so
verification leans on CI + static review. User chose "both, phased": Phase 1
anti-crash + continuous verification (verifiable), Phase 2 fill platform features.

## Structural facts

- `platform/linux.rs` + `platform/macos.rs` are **empty skeletons**. All real
  platform code (icons, file index, app scan, input, clipboard) is Windows-only.
  Linux/macOS run on `cfg(not(windows))` fallbacks scattered across ~18 files.
- Before this batch, CI only compiled Linux/macOS at **release-tag time**
  (`release.yml` on `v*`). No per-push clippy/test for those OSes — so platform
  regressions only surfaced at release (e.g. the v0.7.0 Linux clippy break).

## Phase 1 — anti-crash + continuous verification

- [x] `XPLAT.CI` — add `.github/workflows/ci.yml`: matrix `ubuntu-22.04` +
  `macos-latest` + `windows-latest`, on every push + `workflow_dispatch`. Each runs
  `npm run build` (creates `../dist` for tauri-build) → `rust:clippy -D warnings` →
  `rust:test`. Reuses release.yml's proven Linux apt deps. Catches platform
  regressions on push instead of at release.
- [x] `XPLAT.REVIEW` — static review of all `cfg(not(windows))` / `cfg(target_os =
  "macos"|"linux")` / `cfg(unix)` paths. **Finding: code degrades gracefully — no
  panics found.** Verified surfaces:
  - `system_manager` stub_impl → `Err("not implemented on this platform")`.
  - `system_indexer` → `mdfind` (macOS) / `plocate`+`locate` (Linux) with
    binary-missing → `available:false` fallback + 2.5s timeout.
  - `sandbox_manager` → `bwrap` (Linux) / `sandbox-exec` (macOS) with
    `BinaryMissing` / `Unavailable` fallbacks.
  - `file` reveal/editor → Linux editor `None` (graceful); `control_plane` sets
    `0o600` on the unix control token.
  No crash bugs to fix. The real Phase-1 gap was the missing CI gate, now filled.

### Verification status
CI YAML validated locally (js-yaml parse OK, 3 platforms). **Real Linux/macOS
compile/test result requires a push** (CI triggers on push; `gh` is authed so the
run can be watched). First run may surface pre-existing macOS-only issues never
compiled outside release; fix from the run logs.

## Phase 2 — fill platform features (later, safety-netted by Phase 1 CI)

Not started. Candidates (each needs the OS to truly validate, so gated behind
Phase-1 CI going green): volume/brightness/wifi, app scan, icons, file index,
global-shortcut platform differences. Implement incrementally, smallest first.
