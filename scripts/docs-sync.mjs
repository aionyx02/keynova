#!/usr/bin/env node

import fs from "node:fs";
import path from "node:path";
import { execSync } from "node:child_process";

const ROOT = process.cwd();
const STAGED_ONLY = process.argv.includes("--staged");
const ALL = process.argv.includes("--all");

const DOC_REGISTRY = [
  {
    path: "CLAUDE.md",
    type: "agent_bootstrap",
    status: "active",
    priority: "p0",
    contextPolicy: "always_retrievable",
    owner: "project",
    purpose: "Session bootstrap and high-level guardrails",
  },
  {
    path: "docs/CLAUDE.md",
    type: "agent_policy",
    status: "active",
    priority: "p0",
    contextPolicy: "always_retrievable",
    owner: "project",
    purpose: "ADR and governance policy",
  },
  {
    path: "docs/project.md",
    type: "project_overview",
    status: "active",
    priority: "p1",
    contextPolicy: "always_retrievable",
    owner: "project",
    purpose: "Stable project facts",
  },
  {
    path: "docs/tasks.md",
    type: "task_index_root",
    status: "active",
    priority: "p0",
    contextPolicy: "always_retrievable",
    owner: "project",
    purpose: "Task index entry",
  },
  {
    path: "docs/tasks/active.md",
    type: "task_index",
    status: "active",
    priority: "p0",
    contextPolicy: "always_retrievable",
    owner: "project",
    purpose: "Current execution tasks",
  },
  {
    path: "docs/tasks/backlog.md",
    type: "task_index",
    status: "backlog",
    priority: "p1",
    contextPolicy: "retrieve_when_planning",
    owner: "project",
    purpose: "Future tasks and roadmap",
  },
  {
    path: "docs/tasks/blocked.md",
    type: "task_blockers",
    status: "active",
    priority: "p0",
    contextPolicy: "retrieve_when_planning",
    owner: "project",
    purpose: "Blocking and safety constraints",
  },
  {
    path: "docs/tasks/completed.md",
    type: "task_history",
    status: "completed",
    priority: "p3",
    contextPolicy: "archive",
    owner: "project",
    purpose: "Completed historical records",
  },
  {
    path: "docs/memory.md",
    type: "memory_index_root",
    status: "active",
    priority: "p0",
    contextPolicy: "always_retrievable",
    owner: "project",
    purpose: "Memory index entry",
  },
  {
    path: "docs/memory/current.md",
    type: "working_memory",
    status: "active",
    priority: "p0",
    contextPolicy: "always_retrievable",
    owner: "project",
    purpose: "Current short-term working memory",
  },
  {
    path: "docs/architecture.md",
    type: "architecture_spec",
    status: "active",
    priority: "p1",
    contextPolicy: "retrieve_only",
    owner: "project",
    purpose: "System architecture reference",
  },
  {
    path: "docs/security.md",
    type: "security_policy",
    status: "active",
    priority: "p0",
    contextPolicy: "retrieve_when_planning",
    owner: "project",
    purpose: "Security and permission boundary",
  },
  {
    path: "docs/testing.md",
    type: "testing_policy",
    status: "active",
    priority: "p1",
    contextPolicy: "retrieve_when_debugging",
    owner: "project",
    purpose: "Testing strategy and checks",
  },
  {
    path: "docs/decisions.md",
    type: "decision_index",
    status: "active",
    priority: "p1",
    contextPolicy: "retrieve_only",
    owner: "project",
    purpose: "ADR index",
  },
];

//const MANAGED_PATHS = new Set(DOC_REGISTRY.map((item) => normalize(item.path)));

function normalize(input) {
  return input.replace(/\\/g, "/");
}

function run(command) {
  try {
    return execSync(command, {
      cwd: ROOT,
      stdio: ["ignore", "pipe", "ignore"],
      encoding: "utf8",
    });
  } catch {
    return "";
  }
}

function todayTaipei() {
  return new Intl.DateTimeFormat("en-CA", { timeZone: "Asia/Taipei" }).format(
    new Date(),
  );
}

function parseFrontmatter(text) {
  const normalized = text.replace(/^\uFEFF/, "");
  const match = normalized.match(/^---\n([\s\S]*?)\n---\n?/);
  if (!match) {
    return { frontmatter: new Map(), body: normalized };
  }
  const lines = match[1].split("\n");
  const map = new Map();
  for (const line of lines) {
    const kv = line.match(/^([a-zA-Z0-9_]+):\s*(.*)$/);
    if (kv) {
      map.set(kv[1], kv[2]);
    }
  }
  return { frontmatter: map, body: normalized.slice(match[0].length) };
}

function renderFrontmatter(map) {
  const preferred = [
    "type",
    "status",
    "priority",
    "updated",
    "context_policy",
    "owner",
    "tags",
  ];
  const keys = [...preferred, ...[...map.keys()].filter((k) => !preferred.includes(k))];
  const dedup = [...new Set(keys)].filter((key) => map.has(key));
  const lines = dedup.map((key) => `${key}: ${map.get(key)}`);
  return `---\n${lines.join("\n")}\n---\n\n`;
}

function setMetadata(filePath, metadata, updatedDate) {
  if (!fs.existsSync(filePath)) {
    return;
  }
  const originalRaw = fs.readFileSync(filePath, "utf8");
  const original = originalRaw.replace(/^\uFEFF/, "");
  const { frontmatter, body } = parseFrontmatter(original);
  frontmatter.set("type", metadata.type);
  frontmatter.set("status", metadata.status);
  frontmatter.set("priority", metadata.priority);
  frontmatter.set("updated", updatedDate);
  frontmatter.set("context_policy", metadata.contextPolicy);
  frontmatter.set("owner", metadata.owner);
  const next = `${renderFrontmatter(frontmatter)}${body.trimStart()}`;
  if (next !== original) {
    fs.writeFileSync(filePath, next, "utf8");
  }
}

function getChangedFiles() {
  const changed = new Set();
  const commands = STAGED_ONLY
    ? ["git diff --cached --name-only -- CLAUDE.md docs"]
    : [
        "git diff --name-only -- CLAUDE.md docs",
        "git diff --cached --name-only -- CLAUDE.md docs",
      ];
  for (const command of commands) {
    const output = run(command);
    output
      .split(/\r?\n/)
      .map((line) => normalize(line.trim()))
      .filter(Boolean)
      .forEach((line) => changed.add(line));
  }
  return changed;
}

function buildIndex(registry, updatedDate) {
  const mapRows = registry
    .filter((item) => item.path.startsWith("docs/"))
    .map(
      (item) =>
        `| \`${item.path}\` | \`${item.type}\` | \`${item.status}\` | \`${item.contextPolicy}\` | ${item.purpose} |`,
    )
    .join("\n");

  return `---
type: docs_index
status: active
priority: p0
updated: ${updatedDate}
context_policy: always_retrievable
owner: project
---

<!-- AUTO-GENERATED by scripts/docs-sync.mjs -->

# Keynova Documentation Index

## Why this exists

Use this file as the first lookup step. The goal is retrieval-first context, not full-doc prompt injection.

## Default Read Order

1. \`docs/index.md\`
2. \`docs/memory/current.md\`
3. \`docs/tasks/active.md\`
4. \`docs/tasks/blocked.md\` (only when needed)
5. Additional files by intent

## Retrieval Policy

- Do not read all docs recursively.
- Prefer smallest relevant heading section.
- \`completed\` and \`archive\` files are historical context only.
- If sources conflict: \`tasks/active.md\` + \`memory/current.md\` > \`decisions\` > \`sessions/archive\`.

## Intent Routing

| Intent | Primary docs |
|---|---|
| What should I do now? | \`docs/memory/current.md\`, \`docs/tasks/active.md\`, \`docs/tasks/blocked.md\` |
| Implementation | \`docs/tasks/active.md\`, related \`docs/architecture.md\`, targeted code |
| Security / permission | \`docs/tasks/blocked.md\`, \`docs/security.md\`, relevant ADR |
| Testing / regression | \`docs/testing.md\`, \`docs/tasks/active.md\` |
| Historical question | \`docs/memory/sessions/*\`, \`docs/memory/archive/*\` |

## Context Budget (12k example)

- User request: 1k
- Current memory: 1k
- Active tasks: 2k
- Relevant architecture: 2k
- Relevant code snippets: 4k
- Recent actions/audit: 1k
- Response reserve: 1k

## Document Map

| Path | Type | Status | Context policy | Purpose |
|---|---|---|---|---|
${mapRows}

## Automation Commands

\`\`\`bash
npm run docs:sync
npm run docs:guard
npm run docs:refresh
\`\`\`
`;
}

const changedFiles = getChangedFiles();
const touched = ALL
  ? DOC_REGISTRY
  : DOC_REGISTRY.filter((item) => changedFiles.has(normalize(item.path)));

const today = todayTaipei();
for (const entry of touched) {
  setMetadata(path.join(ROOT, entry.path), entry, today);
}

const indexPath = path.join(ROOT, "docs", "index.md");
const nextIndex = buildIndex(DOC_REGISTRY, today);
if (!fs.existsSync(indexPath) || fs.readFileSync(indexPath, "utf8") !== nextIndex) {
  fs.mkdirSync(path.dirname(indexPath), { recursive: true });
  fs.writeFileSync(indexPath, nextIndex, "utf8");
}

if (touched.length > 0 || ALL) {
  console.log(`[docs-sync] Updated ${touched.length} managed documents and docs/index.md.`);
} else {
  console.log("[docs-sync] No managed docs changed. docs/index.md regenerated if needed.");
}
