---
type: project_overview
status: active
priority: p1
updated: 2026-06-05
context_policy: always_retrievable
owner: project
---

# Keynova Project Overview

## Product

Keynova is a keyboard-first local workflow entry for technical workers: a
search-first dispatcher over apps, files, workspaces, commands, and built-in
tools. AI is an inline single-step capability, not the product core.

Non-goals: general launcher parity, an AI chat center, an autonomous-agent
mainline, and broad productivity-suite expansion. See
`docs/tasks/product-roadmap.md` for core-vs-parked scope and the v0.6 → v1.0 line.

## Goals

- Complete 90%+ workflows without mouse.
- Keep local-first architecture and explicit approval boundaries.
- Preserve predictable behavior while evolving architecture.

## Stack

- Tauri 2.x
- React + TypeScript
- Rust + Tokio
- Tantivy + SQLite

## Platform Targets

- Windows 10+
- Linux (X11/Wayland)
- macOS 11+

## Engineering Priorities

1. Safety and correctness
2. Rollback and testability
3. Performance and developer speed

## Current Strategy

- Search-first: the palette is a dispatcher; AI is a stateless capability layer.
- Approval-aware: risky or system-affecting actions stay confirmation-gated.
- Retrieval-first docs workflow.
- Markdown remains source of truth, but no full-doc prompt dump.
