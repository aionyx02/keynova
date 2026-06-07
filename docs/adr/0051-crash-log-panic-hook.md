---
type: adr
status: proposed
priority: p1
updated: 2026-06-07
context_policy: retrieve_only
owner: project
---

# Backend Crash-Log Panic Hook (Local, Redacted)

**Status:** proposed
**Date:** 2026-06-07
**Decision makers:** AI agent draft; developer acceptance requested by STAB.1
**Related documents:**

- `docs/tasks/stabilization.md` (§STAB.1)
- `docs/tasks/bug-followup.md` (Bug A — silent window-vanish / 閃退)
- `docs/adr/0047-diagnostics-export.md` (`/diag` bundle + redaction)
- `docs/security.md`

---

## 1. Context

The launcher has a class of "閃退 / window vanished" reports (`bug-followup.md`,
Bug A) with no reliable repro. The frontend already has an ErrorBoundary plus
global `error`/`unhandledrejection` handlers, but the **Rust** side installs no
`std::panic::set_hook`. A panic in an async task or command handler can drop the
process (or unwind a worker) with nothing written to disk, so the user has
nothing to attach to a bug report and we have nothing to diagnose.

## 2. Decision

Install a process-wide panic hook early in startup that appends one structured,
**redacted** line per panic to a local crash log, then chains to the previous
(default) hook so console/stderr behavior is unchanged.

- **Location:** `%LOCALAPPDATA%\Keynova\crash.log` on Windows (platform data dir
  via the same resolver `/diag` uses); same app-owned trust zone as `config.toml`.
- **Contents per line:** ISO timestamp, thread name, panic location
  (`file:line`), and the panic payload message. Home paths are collapsed
  (reusing `diagnostics::collapse_home`) and the message is length-capped.
- **Best-effort + safe:** a failure to open/write the log is swallowed; the hook
  never panics, never blocks, and is re-entrancy tolerant.
- **Bounded:** the log is truncated/rotated at a small byte cap so it cannot grow
  without limit.
- **Surfaced in `/diag`:** the last crash entry (timestamp + truncated message,
  or "none") is added to the diagnostics bundle under the existing redaction
  discipline, so a pasted bundle now carries crash evidence.

No new IPC command, no network, no new permission scope. This is a local
diagnostic sink, not telemetry — nothing leaves the machine unless the user
copies `/diag` or the file themselves.

Rejected alternative: a full crash-reporter (minidumps / Sentry-style upload).
Heavier, adds a network/privacy surface, and is premature before we even know the
panic class. A local line-log is the minimal step that makes the failure
diagnosable.

## 3. Consequences

- Backend panics become diagnosable instead of silent; pairs with the existing
  frontend handlers for full-stack coverage.
- A `crash.log` file may appear in the app data dir. A panic message can embed
  arbitrary runtime strings, so redaction (home collapse + length cap) is
  required; even so the file stays local in the already-protected data dir and is
  no more exposed than `config.toml`. `security.md` gets a row for it.
- The hook must avoid re-entrant panics (e.g. a poisoned lock inside the hook),
  so it uses only allocation-light, lock-free-ish writes.

## 4. Rollback

Remove the `set_hook` registration and the `/diag` crash row. No persisted state
or schema is involved; deleting `crash.log` is safe at any time.