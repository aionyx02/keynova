#!/usr/bin/env node

import fs from "node:fs";
import path from "node:path";
import { ROOT } from "./docs-utils.mjs";

const LIMITS = new Map([
  ["CLAUDE.md", 8 * 1024],
  ["docs/CLAUDE.md", 8 * 1024],
  ["docs/index.md", 6 * 1024],
  ["docs/project.md", 2 * 1024],
  ["docs/memory/current.md", 5 * 1024],
  ["docs/tasks/active.md", 5 * 1024],
  ["docs/tasks/backlog.md", 8 * 1024],
  ["docs/tasks/blocked.md", 8 * 1024],
  ["docs/tasks/completed.md", 5 * 1024],
  ["docs/testing-edge-cases.md", 9 * 1024],
]);

const failures = [];

for (const [file, limit] of LIMITS) {
  const fullPath = path.join(ROOT, file);
  if (!fs.existsSync(fullPath)) {
    continue;
  }
  const size = fs.statSync(fullPath).size;
  if (size > limit) {
    failures.push({ file, size, limit });
  }
}

if (failures.length > 0) {
  console.error("[docs-guard-size] Markdown size limits failed:");
  for (const failure of failures) {
    console.error(`  - ${failure.file}: ${failure.size}B > ${failure.limit}B`);
  }
  console.error("[docs-guard-size] Move historical narrative to docs/memory/sessions/YYYY-MM-DD.md.");
  process.exit(1);
}

console.log("[docs-guard-size] Markdown size checks passed.");
