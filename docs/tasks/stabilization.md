---
type: task_plan
status: active
priority: p1
context_policy: on_demand
owner: project
tags: [stabilization, hardening, reliability, product-3]
---

# STAB - Feature Stabilization & Hardening Pass

Branch: `hardening/stabilize-features` (forked from `main`, since `dev` is behind
`main`). Goal: converge the currently-shipped features — make crashes
diagnosable instead of silent, and close concrete edge-case correctness gaps —
without adding product surface.

Audit baseline (2026-06-07): clippy clean, 532 Rust + 206 FE tests green,
handlers carry zero `lock().unwrap()`, and the new PRODUCT.3 surfaces
(`/diag`, config rollback, updater) are defensively written. So this pass is
targeted hardening, not a rewrite.

Scope was developer-confirmed (STAB.1 + STAB.2 + STAB.3; STAB.4 deferred).

## STAB.1 - Global panic safety net (primary)

Problem: there is no backend `std::panic::set_hook`. An async-task or command
panic can take the process down (or poison silently) with no on-disk trace, which
is exactly the "閃退 / window vanished" class in `bug-followup.md` that has no
repro. The frontend already has an ErrorBoundary + global error handlers; the
Rust side has nothing equivalent.

Scope:

- Install a process-wide panic hook early in startup that appends a structured,
  **redacted** line to `%LOCALAPPDATA%\Keynova\crash.log` (platform data dir):
  timestamp, thread, location, and panic message. Chain to the default hook so
  console/stderr behavior is unchanged.
- Keep the writer best-effort and re-entrancy-safe: a failure to write a crash
  log must never itself panic or block shutdown.
- Surface "last crash" (timestamp + truncated message) in the `/diag` bundle so a
  bug report carries it, reusing the existing redaction discipline.
- Bound the log (truncate/rotate at a small cap) so it cannot grow unbounded.

Done:

- A deliberately-triggered panic in a test/dev path lands one line in `crash.log`
  and the app does not lose the log on the next launch.
- `/diag` shows the last crash entry (or "none").
- No secret/raw home path leaks into `crash.log` or the `/diag` crash row.

Decision record: ADR-0051 (proposed) — local crash-log sink + redaction posture.

## STAB.2 - Edge-case correctness hardening (primary)

Concrete bugs found in the `/diag` assembler audit:

- `core/diagnostics.rs::collapse_home` uses a raw string `strip_prefix`, so a
  home like `/home/al` would wrongly collapse `/home/alice/...`. Require the
  match to end on a path separator (or full-string equality) before collapsing.
- `core/diagnostics.rs::path_size` recurses with no depth/entry bound, so `/diag`
  over a large index/data dir can stall. Add a depth + visited-entry cap and
  return best-effort partial size past the cap.

Done:

- Added unit tests: partial-component home prefixes are left untouched; deep/wide
  directory trees stop at the cap without hanging.

## STAB.3 - Window / hot-path micro-hardening (secondary, non-blocking)

`app/window.rs` already waits 1500 ms, honors the keep-open guard, and re-checks
`is_focused()` before hiding, so the auto-hide path is largely solid. Marginal:

- Optional: require two consecutive unfocused observations before hide for very
  slow IME, only if it does not regress normal blur-to-hide latency.
- Sweep a small number of hot-path `unwrap()`/`expect()` reachable from IPC
  command entry points and downgrade to graceful errors where cheap and clear.

Done:

- Any change keeps existing window/IPC tests green; no normal-use latency
  regression. This batch is explicitly allowed to land partial (simplification-
  only items here do not block STAB.1/.2 close-out).

## Non-goals

- No new product feature, no execution path, no agent revival.
- STAB.4 (`嚙` garbled-text root cause) stays deferred — blocked on a repro.
- `feature/bugb-permanent-delete` (skip-trash delete) stays a separate track.

## Status (2026-06-07)

- STAB.1 **done**: `core/crash_log.rs` panic hook (redacted, bounded, chained)
  installed at the top of `bootstrap::run`; last crash surfaced in `/diag`.
  ADR-0051 proposed. Also captures async-task panics (global hook).
- STAB.2 **done**: `collapse_home` component-boundary fix + `path_size`
  depth/entry caps, with unit tests.
- STAB.3 **assessed → no-op**: IPC command entry points are already panic-free
  (all handler `unwrap`/`expect` live in `#[cfg(test)]`); `window.rs` is already
  hardened (1500 ms + keep-open guard + focus recheck), and a second pre-hide
  check would regress normal blur-to-hide latency, so it was deliberately not
  added. The async-panic robustness STAB.3 might have chased is now covered by
  STAB.1's global hook. No code change.

Detail: `docs/memory/sessions/2026-06-07.md`.

## Validation

```bash
npm run lint && npm run test && npm run build
cargo test --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
npm run docs:refresh
```