#!/usr/bin/env node

import fs from "node:fs";
import path from "node:path";
import { ROOT, repoPath } from "./docs-utils.mjs";

const sessionDir = path.join(ROOT, "docs", "memory", "sessions");
const archiveDir = path.join(ROOT, "docs", "memory", "archive");
const cutoffMs = Date.now() - 60 * 24 * 60 * 60 * 1000;
let moved = 0;

if (!fs.existsSync(sessionDir)) {
  console.log("[docs-archive-sessions] No session directory found.");
  process.exit(0);
}

for (const entry of fs.readdirSync(sessionDir, { withFileTypes: true })) {
  if (!entry.isFile() || !entry.name.endsWith(".md")) {
    continue;
  }

  const dateMatch = entry.name.match(/^(\d{4})-(\d{2})-(\d{2})/);
  if (!dateMatch) {
    continue;
  }

  const fileDate = new Date(`${dateMatch[1]}-${dateMatch[2]}-${dateMatch[3]}T00:00:00Z`);
  if (Number.isNaN(fileDate.getTime()) || fileDate.getTime() >= cutoffMs) {
    continue;
  }

  const month = `${dateMatch[1]}-${dateMatch[2]}`;
  const targetDir = path.join(archiveDir, `sessions-${month}`);
  const source = path.join(sessionDir, entry.name);
  const target = path.join(targetDir, entry.name);
  fs.mkdirSync(targetDir, { recursive: true });
  fs.renameSync(source, target);
  moved += 1;
  console.log(`[docs-archive-sessions] ${repoPath(source)} -> ${repoPath(target)}`);
}

if (moved === 0) {
  console.log("[docs-archive-sessions] No old sessions to archive.");
} else {
  console.log("[docs-archive-sessions] Run npm run docs:completed-regen after archiving.");
}
