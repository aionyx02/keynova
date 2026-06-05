# Human-Driven Interactive Terminal

**狀態：** proposed
**日期：** 2026-06-05
**決策者：** 開發者（AI 起草）
**相關文件：**

- docs/adr/0027-generic-shell-sandbox.md
- docs/adr/0020-terminal-launch-specs.md
- docs/adr/0022-agent-approval-boundary.md
- docs/memory/sessions/2026-06-02.md（REF.9 security hardening）
- docs/security.md

---

## 1. Context

REF.9 security hardening (`68ff44e`, 2026-06-02) removed the bare default-shell
terminal: `terminal.open` now requires a backend-issued, registered, one-time
`TerminalLaunchSpec` and rejects a `None` spec; the renderer no longer builds
launch specs; automation/pipeline dispatch blocks `terminal.*`. As a side effect,
typing `>` in the palette no longer opens a terminal at all (`setTerminalMounted`
is never called, and a null-spec open is rejected).

The developer considers an interactive terminal an essential engineer tool and
wants it back **without** weakening the security posture.

Key distinction: **ADR-0027 prohibits exposing a generic shell as an AI/agent
*tool* (LLM- or automation-triggered command execution).** A terminal a **human**
types into, in the focused launcher window, is a different risk class — the same
as any terminal emulator. The danger REF.9 guarded against is a shell reachable by
**non-human** actors (agent, automation/pipeline, remote control-plane), not a
human-driven PTY.

## 2. Decision

Re-enable a **human-driven** interactive terminal, opened **only** by an explicit
user gesture (typing `>` in the focused palette), while keeping every REF.9
invariant for non-human actors:

- New backend command `terminal.request_shell`: builds a default-shell
  `TerminalLaunchSpec` (program = `%COMSPEC%`/`cmd.exe` on Windows, `$SHELL`/`sh`
  on Unix; `cwd` = active workspace `project_root`; `editor = false`), **registers
  it** via the existing one-time registry, and returns it. The frontend then opens
  it through the unchanged `terminal.open` (consume + spawn) path.
- So every PTY is still a **backend-issued, registered, one-time** spec — the
  REF.9 invariant is preserved; there is still no `create_pty(None)` bare shell.

Unchanged (still blocked for non-human actors):

- Automation/pipeline allowlist continues to block the whole `terminal.*`
  namespace (including `request_shell`).
- The control-plane token still gates remote/CLI requests.
- No generic `execute_shell_command` agent tool exists — **ADR-0027 is
  unaffected.** The capability/agent layer cannot reach `terminal.*` and cannot
  write to a PTY (`terminal.send` stays renderer-only, driven by xterm keystrokes).
- PRODUCT.1.E (Keynova *sending/executing* discovered commands) remains dropped;
  project commands stay copy-only.

## 3. Consequences

- Positive: engineers regain a first-class interactive terminal (cwd = project
  root) via `>`, keyboard-first, no mouse.
- Residual risk: a human can again run arbitrary commands in the PTY — but only
  the human, by their own keystrokes, in the focused window. This is the accepted
  risk of any terminal emulator and is categorically different from
  agent/automation-triggered execution, which stays blocked.
- ADR-0027's generic-shell-as-agent-tool prohibition is **not** relaxed.

## 4. Validation

- Unit: `terminal.request_shell` returns a spec that `terminal.open` accepts
  (registered once); a non-issued/mutated spec is still rejected.
- Guard: automation allowlist still blocks `terminal.*` (existing test +
  `request_shell` covered by the namespace prefix).
- Manual: `>` opens an interactive shell at the workspace root; commands run;
  Esc / Ctrl+Shift+Q exits.

## 5. Rollback

Remove `terminal.request_shell` and stop calling `setTerminalMounted` on `>`; the
terminal returns to backend-issued-spec-only (editor sessions). No data migration.
