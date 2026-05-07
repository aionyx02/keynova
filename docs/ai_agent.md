# Keynova AI Agent Runtime

## Goal

`/ai` keeps its normal chat mode, and adds an explicit Agent mode for local work planning. Agent mode may inspect approved workspace context, prepare plans, call low-risk read-only tools, and request approval before any local side effect.

## Non-goals

- The agent must not directly call shell, filesystem, clipboard, system control, or model lifecycle APIs.
- The agent must not receive raw private architecture docs, secrets, API keys, or unfiltered debug/cache content.
- The first implementation is not an autonomous background worker. It produces safe plans and uses controlled tools.

## Runtime Loop

1. Understand the user request.
2. Classify all candidate context by visibility.
3. Build a bounded prompt from allowlisted context only.
4. Produce a plan that can be cancelled.
5. Use read-only tools first: `web.search`, `keynova.search`, list actions, read workspace summary, read notes/history summary, ask model.
6. Request approval for medium-risk actions such as opening a panel, creating a note draft, updating a setting draft, or running a safe built-in command.
7. Require per-action approval for high-risk actions such as terminal commands, file writes, system control, or model download/delete.
8. Stream status events and write an audit log.

## Risk Levels

- `low`: Read-only, bounded output, visibility-filtered. No approval required.
- `medium`: Local UI or draft changes. Approval required before execution.
- `high`: Process, filesystem, system, clipboard, network target expansion, model lifecycle. Approval required every time.

## Events

- `agent.run.started`
- `agent.step.started`
- `agent.tool.requested`
- `agent.approval.required`
- `agent.run.completed`
- `agent.run.failed`

Legacy Tauri aliases are emitted during migration by replacing dots with hyphens.

## Current Implementation

Batch 4 now adds:

- prompt audit with bounded included sources plus filtered private/secret source metadata
- pending approval state for medium/high-risk actions
- approved panel/draft execution for notes, settings, safe built-in commands, and panel opens
- approved terminal launch for explicit high-risk command requests
- Knowledge Store audit logging and opt-in long-term memory references

Generic file writes remain scaffolded rather than directly executed.
