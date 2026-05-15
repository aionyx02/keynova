---
type: task_blockers
status: active
priority: p0
updated: 2026-05-15
context_policy: retrieve_when_planning
owner: project
tags: [security, approval, shell, blockers]
---

# Blocked Tasks and Safety Constraints

## Generic Shell Tool (Still Blocked)

Status: blocked

Reason:
- Product-level sandbox boundary is still incomplete for unrestricted shell execution.
- Arbitrary shell tool remains high destructive risk even with approval flow.
- Current audit/approval path is not sufficient for full generic execution.

Do not:
- Add generic shell execution tools.
- Allow direct LLM-triggered arbitrary system commands.

Allowed:
- Add deterministic typed tools.
- Keep approval-gated read-only or narrowly scoped commands.
- Keep bounded output and safety checks in tool observations.

## FEAT.11 Learning Material Review (Blocked)

Status: blocked

Blocked by:
- ADR-028 accepted with explicit local-context security boundary.
- TD.5.A verify baseline execution gate.
- PERF.1 low-memory runtime baseline.
- TD.1.A IPC boundary.
- TD.2.A composition root extraction.
- TD.3.A typed DTO baseline.

Until unblocked, do not:
- Add runtime local-context scanning features that touch private user files.
- Add agent tool paths that bypass approval-gated scan scope selection.
- Add full-content recursive document indexing for this feature.

Allowed before unblock:
- ADR drafting and review.
- Config/DTO design scaffolding that does not activate runtime scanning behavior.
- Unit tests for denylist/redaction logic in isolation.

## RAM Budget Scope Guard

Status: constrained

Rule:
- The target "Background Core < 100 MB" excludes active WebView, loaded local LLM model memory, PTY terminal sessions, monitoring streams, and index rebuild tasks.

Do not:
- Claim end-to-end "Keynova + loaded local model" under 100 MB.
- Regress startup by enabling heavy runtime services in background by default.

## Refactor Backlog Execution Gate

Only execute TD.1-TD.5 early when they are required by current mainline tasks.

Current known dependency gates:
- PERF.2 aligns with TD.4.B (SearchService actor boundary).
- PERF.3 aligns with TD.4.C (Terminal actor boundary).

## Post-FEAT.11 Tracks Pending ADR (2026-05-15)

Status: planning-allowed, implementation-blocked until ADR accepted.

Each track below has been scoped in `backlog.md` but requires the listed ADR to reach `接受` before runtime implementation may begin. ADR drafting, schema design, and test scaffolding are allowed in advance.

| Track | Required ADR | Reason for ADR |
|---|---|---|
| AGENT.3 Tool Surface Rebalance | ADR-029 | New approval-gated action tools change agent dispatch contract and approval boundary |
| AGENT.4 Context Awareness | ADR-030 | OS-level selection capture introduces global hotkey + clipboard staging with privacy implications |
| CLIP.1 Clipboard History | ADR-031 | Continuous OS clipboard capture is a new system-wide data ingress with sensitive content risk |
| SNIP.1 Snippet / Text Expansion | ADR-032 | Stage 2 全域鍵盤 hook 是新的 input monitoring 邊界 (stage 1 launcher-only 可不卡 ADR) |
| WIN.1 Window Switcher | ADR-033 | Platform window enumeration touches Accessibility (macOS) and X11/Wayland permissions |
| UTIL.3 Reminder & Timer | ADR-034 | New ScheduleManager 持久化背景觸發改變 runtime lifecycle |
| DEV.1.C/D External Provider Auth | ADR-035 | GitHub/GitLab token storage 與 OS keyring 整合是新 secret boundary |
| SYNC.1.D/E/F Git-Backed Sync | ADR-036 | 對使用者控制路徑做 git push/pull 屬於新的 write/execute 邊界 |
| AI.1 Inline AI Surfaces | ADR-037 | AI 進入 launcher / terminal / note 改變 ai.chat trigger 表面與 approval gate 配置 |

Do not (until corresponding ADR is `接受`):
- Activate runtime behaviour of the gated subsystem.
- Wire global hotkeys / OS hooks for capture / monitoring.
- Persist user secrets to disk outside existing config paths.

Allowed before ADR acceptance:
- ADR drafting and review.
- DTO / schema scaffolding without runtime wiring.
- Pure-logic unit tests (no OS I/O).
- UI mockups in storybook-equivalent isolated panels.
