#!/usr/bin/env node

import fs from "node:fs";
import path from "node:path";
import { ROOT, parseFrontmatter, repoPath, walkMarkdown } from "./docs-utils.mjs";

const ALLOWED_ALWAYS = new Set([
  "docs/memory/current.md",
  "docs/tasks/active.md",
  "docs/project.md",
]);

const violations = [];
const missingPolicy = [];

for (const filePath of walkMarkdown(path.join(ROOT, "docs"))) {
  const file = repoPath(filePath);
  const { frontmatter, hasFrontmatter } = parseFrontmatter(fs.readFileSync(filePath, "utf8"));
  if (!hasFrontmatter) {
    continue;
  }
  const policy = frontmatter.get("context_policy");
  if (!policy) {
    missingPolicy.push(file);
  } else if (policy === "always_retrievable" && !ALLOWED_ALWAYS.has(file)) {
    violations.push(file);
  }
}

if (missingPolicy.length > 0) {
  console.warn("[docs-audit-frontmatter] Missing context_policy in frontmatter:");
  for (const file of missingPolicy) {
    console.warn(`  - ${file}`);
  }
}

if (violations.length > 0) {
  console.error("[docs-audit-frontmatter] Unauthorized always_retrievable files:");
  for (const file of violations) {
    console.error(`  - ${file}`);
  }
  console.error(
    "[docs-audit-frontmatter] Use on_demand, retrieve_when_debugging, retrieve_only, or archive.",
  );
  process.exit(1);
}

console.log("[docs-audit-frontmatter] Frontmatter policy checks passed.");
