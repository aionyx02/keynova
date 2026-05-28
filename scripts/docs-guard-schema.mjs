#!/usr/bin/env node

import fs from "node:fs";
import path from "node:path";
import { ROOT, parseFrontmatter } from "./docs-utils.mjs";

const SCHEMAS = {
  "docs/memory/current.md": {
    required: ["Current Strategy", "Current Focus", "Important Constraints", "Next Step"],
    allowed: ["Current Strategy", "Current Focus", "Important Constraints", "Next Step"],
    forbidden: [
      "Last Confirmed Progress",
      "Recent Execution Notes",
      "Session History",
      "Detailed Notes",
    ],
  },
  "docs/tasks/active.md": {
    required: ["Active Queue", "Strategy", "Next Phase Candidates"],
    allowed: ["Active Queue", "Strategy", "Next Phase Candidates"],
    forbidden: [
      "Recent Execution Notes",
      "Feasibility Verdict",
      "Session History",
      "Detailed Notes",
    ],
  },
  "docs/tasks/backlog.md": {
    required: ["P0", "P1", "P2", "Frozen"],
    allowed: ["P0", "P1", "P2", "Frozen"],
    forbidden: ["Recent Execution Notes", "Detailed Notes", "Session History"],
  },
};

const COMPLETED_PATH = "docs/tasks/completed.md";
const failures = [];

function h2Headings(body) {
  return [...body.matchAll(/^##\s+(.+)$/gm)].map((match) => match[1].trim());
}

function hasDatedNarrative(body) {
  return /^[-*]\s+\d{4}-\d{2}-\d{2}\b/m.test(body) || /^##+\s+\d{4}-\d{2}-\d{2}\b/m.test(body);
}

for (const [file, schema] of Object.entries(SCHEMAS)) {
  const fullPath = path.join(ROOT, file);
  if (!fs.existsSync(fullPath)) {
    failures.push(`${file}: missing file`);
    continue;
  }

  const { body } = parseFrontmatter(fs.readFileSync(fullPath, "utf8"));
  const headings = h2Headings(body);
  for (const required of schema.required) {
    if (!headings.includes(required)) {
      failures.push(`${file}: missing required section "${required}"`);
    }
  }
  for (const heading of headings) {
    if (!schema.allowed.includes(heading)) {
      failures.push(`${file}: section "${heading}" is not allowed by schema`);
    }
    if (schema.forbidden.includes(heading)) {
      failures.push(`${file}: forbidden section "${heading}"`);
    }
  }
  if (hasDatedNarrative(body)) {
    failures.push(`${file}: dated narrative belongs in docs/memory/sessions/`);
  }
}

const completedFullPath = path.join(ROOT, COMPLETED_PATH);
if (fs.existsSync(completedFullPath)) {
  const { body } = parseFrontmatter(fs.readFileSync(completedFullPath, "utf8"));
  for (const heading of h2Headings(body)) {
    if (!/^\d{4}-\d{2}$/.test(heading)) {
      failures.push(`${COMPLETED_PATH}: section "${heading}" must be a YYYY-MM archive bucket`);
    }
  }
}

if (failures.length > 0) {
  console.error("[docs-guard-schema] Markdown schema checks failed:");
  for (const failure of failures) {
    console.error(`  - ${failure}`);
  }
  process.exit(1);
}

console.log("[docs-guard-schema] Markdown schema checks passed.");
