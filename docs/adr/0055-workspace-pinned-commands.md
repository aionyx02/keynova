---
type: adr
status: rejected
priority: p1
updated: 2026-06-10
context_policy: retrieve_only
owner: project
---

# Per-Workspace Pinned Commands

**Status:** rejected — feature removed 2026-06-10 (developer-directed); see §5.
**Date:** 2026-06-08
**Decision makers:** AI agent draft; developer rejected at PROFILE.2 review (never accepted)
**Related documents:**

- `docs/tasks/product-4-profiles.md` (§PROFILE.2)
- `docs/adr/0054-workspace-command-profile.md` (the profile surface this extends)

---

## 1. Context

PROFILE.1 (ADR-0054) gives each project a *computed* profile: its signature
commands ranked by frequency × success rate. That list is derived — it drifts as
usage changes and a command you rely on but run rarely can fall off it. PROFILE.2
wants a *curated* layer: a small set of commands the user deliberately keeps one
keystroke away in **this workspace**, always shown atop the computed profile.

`WorkspaceState` (the 3 manual slots in `workspaces.json`) already carries
per-slot state (`name`, `project_root`, recent actions/files). It is the natural
home for a curated pin set; no new store is needed.

## 2. Decision

Add a per-workspace-slot `pinned_commands` list and surface it atop the `profile`
capability list.

- **Storage (`workspaces.json` v2 → v3).** `WorkspaceState` gains
  `pinned_commands: Vec<String>` — replayable command titles (the `/name args`
  form already used as `action_label`). `#[serde(default)]` so existing files
  load as an empty list; the version bump only refreshes `version`. Additive and
  backward-compatible. Capped at `MAX_PINS = 8` per slot (a curated set, not a
  log). Pins are **per slot**, not per `project_root` — they ride with the slot
  the user pinned them in, matching how `name`/`query` already behave. (The
  computed profile underneath is still project-keyed via ADR-0054.)
- **IPC.** One additive `workspace.pin` command toggles a command string for the
  current slot (add if absent, remove if present), returning the new pinned set;
  the existing `workspace.get_current` already exposes `pinned_commands` for
  listing. No change to any capability contract.
- **Surface.** The `profile` surface prepends pinned commands to the
  `SuggestedNextAction[]` it already renders in `CapabilityListCard` — pinned rows
  carry a 📌 marker, confidence 1.0, and the same `cmd.run` replay descriptor
  (reconstructed from the title, mirroring `replay_for_row`). Computed rows that
  duplicate a pin are dropped so a pin appears once, atop. **Copy/replay-only** —
  no new execution path. Pinning has two affordances for discoverability: a
  clickable 📌 toggle on each pinnable (`cmd.run`) row and the **Ctrl+P** keyboard
  toggle on the selected row, plus a first-run onboarding banner shown until the
  workspace has any pin. The idle `next` surface passes no pin control and is
  untouched.

Keying pins by **slot** (not `project_root`) is the decision: pins are an
explicit user choice tied to the workspace they curate, so they stay stable even
before any project-tagged history exists, and the storage lives where the rest of
per-slot state already lives.

Rejected alternatives:

- *Project-keyed pins (new column/store).* Heavier, and a pin is an explicit act
  the user already scoped by choosing a slot — slot keying needs no new store.
- *Pin via a `pin <command>` prefix only.* Forces retyping the full command; a
  toggle on the already-shown profile row is the keyboard-first path. (The prefix
  can be added later additively if discoverability needs it.)

## 3. Consequences

- One additive config field + one additive IPC command; no capability or
  cross-process contract change. `workspace_profile` stays pure/unit-tested — the
  pin merge + dedupe is a client-side concern atop its output.
- A pin survives even if the command leaves the computed profile, which is the
  point; a pin to a command that no longer resolves simply replays as before
  (the existing replay path already guards execution).
- The `MAX_PINS` cap keeps the curated set small and the surface readable.

## 4. Rollback

Stop reading/writing `pinned_commands` and drop the `workspace.pin` command; the
field can remain unused in `workspaces.json`. No data loss — the computed profile
(ADR-0054) is unaffected.

## 5. Outcome — rejected and removed (2026-06-10)

Developer-directed: the pin feature was judged not worth its surface and removed
on branch `chore/drop-workspace-pins`. Rationale:

- **Surfacing was too weak for the value prop.** Pins only appeared inside the
  on-demand `profile` card, reachable only by typing the `profile` prefix — so a
  pinned command was never actually "one keystroke away," undercutting the whole
  premise.
- **Overlapped the automatic ranker.** PROFILE.1 (ADR-0054) already ranks by
  frequency × success rate, so a command you rely on surfaces on its own; manual
  pinning only helped the narrow "rely-on-but-rarely-run" slice.
- **Per-slot keying was the same shaky premise** that sank PROFILE.3 (slots are
  general-purpose; only the launch slot carries `project_root`).

Removed per the §4 rollback (full removal, not the "leave the field" variant):
the `pinned_commands` field + `toggle_pin` + `workspace.pin` IPC, the
`workspacePins`/`useWorkspacePins` frontend layer, the 📌 toggle / onboarding
banner / footer hint in `CapabilityListCard`, the Ctrl+P binding, and the pin
i18n strings. `workspaces.json` stays v3 — serde ignores any residual
`pinned_commands` in old files. The computed `profile` surface (ADR-0054) is
untouched.
