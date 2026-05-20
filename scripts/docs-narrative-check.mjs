#!/usr/bin/env node

import fs from "node:fs";
import path from "node:path";
import { ROOT, parseFrontmatter } from "./docs-utils.mjs";

const TARGETS = [
  "docs/memory/current.md",
  "docs/tasks/active.md",
  "docs/tasks/backlog.md",
  "docs/testing-edge-cases.md",
];

const FORBIDDEN_PHRASES = [
  "Recent Execution Notes",
  "Last Confirmed Progress",
  "Session History",
  "Bugfix Round",
];

let warnings = 0;

for (const file of TARGETS) {
  const fullPath = path.join(ROOT, file);
  if (!fs.existsSync(fullPath)) {
    continue;
  }
  const { body } = parseFrontmatter(fs.readFileSync(fullPath, "utf8"));
  const datedBullets = [...body.matchAll(/^[-*]\s+\d{4}-\d{2}-\d{2}\b/gm)].length;
  const datedHeadings = [...body.matchAll(/^##+\s+\d{4}-\d{2}-\d{2}\b/gm)].length;
  const forbidden = FORBIDDEN_PHRASES.filter((phrase) => body.includes(phrase));
  if (datedBullets || datedHeadings || forbidden.length) {
    warnings += 1;
    console.warn(`[docs-narrative-check] ${file}: possible narrative accumulation`);
    if (datedBullets || datedHeadings) {
      console.warn(`  dated entries: ${datedBullets + datedHeadings}`);
    }
    for (const phrase of forbidden) {
      console.warn(`  forbidden phrase: ${phrase}`);
    }
    console.warn(`  suggestion: npm run docs:extract-narrative -- ${file}`);
  }
}

if (warnings === 0) {
  console.log("[docs-narrative-check] No narrative accumulation detected.");
}
