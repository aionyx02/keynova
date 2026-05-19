#!/usr/bin/env node

import fs from "node:fs";
import path from "node:path";
import { execSync } from "node:child_process";

const ROOT = process.cwd();
const STAGED_ONLY = process.argv.includes("--staged");

const KEY_STATE_DOCS = new Set([
  "docs/tasks/active.md",
  "docs/tasks/backlog.md",
  "docs/tasks/blocked.md",
  "docs/tasks/completed.md",
  "docs/memory/current.md",
]);

const REQUIRED_FRONTMATTER = [
  "CLAUDE.md",
  "docs/CLAUDE.md",
  "docs/project.md",
  "docs/index.md",
  "docs/tasks.md",
  "docs/tasks/active.md",
  "docs/tasks/backlog.md",
  "docs/tasks/blocked.md",
  "docs/tasks/completed.md",
  "docs/memory.md",
  "docs/memory/current.md",
  "docs/architecture.md",
  "docs/security.md",
  "docs/testing.md",
  "docs/decisions.md",
];

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

function getChangedFiles() {
  const changed = new Set();
  const commands = STAGED_ONLY
    ? ["git diff --cached --name-only"]
    : ["git diff --name-only", "git diff --cached --name-only"];
  for (const command of commands) {
    run(command)
      .split(/\r?\n/)
      .map((line) => normalize(line.trim()))
      .filter(Boolean)
      .forEach((line) => changed.add(line));
  }
  return changed;
}

/** Return files added (not modified) in this diff. */
function getAddedFiles() {
  const added = new Set();
  const commands = STAGED_ONLY
    ? ["git diff --cached --name-only --diff-filter=A"]
    : ["git diff --name-only --diff-filter=A", "git diff --cached --name-only --diff-filter=A"];
  for (const command of commands) {
    run(command)
      .split(/\r?\n/)
      .map((line) => normalize(line.trim()))
      .filter(Boolean)
      .forEach((line) => added.add(line));
  }
  return added;
}

// Directories whose new files signal an architectural addition.
const ARCH_NEW_FILE_PREFIXES = [
  "src-tauri/src/handlers/",
  "src-tauri/src/managers/",
  "src-tauri/src/core/",
  "src/context/",
  "src/stores/",
];

// Files that, when modified, indicate structural wiring changes.
const ARCH_WIRING_FILES = new Set([
  "src-tauri/src/app/state.rs",
  "src-tauri/src/app/dispatch.rs",
  "src/ipc/routes.ts",
]);

function hasRequiredFrontmatter(filePath) {
  if (!fs.existsSync(filePath)) {
    return false;
  }
  const content = fs.readFileSync(filePath, "utf8").replace(/^\uFEFF/, "").replace(/\r\n/g, "\n");
  const match = content.match(/^---\n([\s\S]*?)\n---\n?/);
  if (!match) {
    return false;
  }
  const block = match[1];
  const required = ["type:", "status:", "updated:", "context_policy:"];
  return required.every((field) => block.includes(field));
}

const changed = getChangedFiles();
const added = getAddedFiles();

const codeChanged = [...changed].some(
  (file) => file.startsWith("src/") || file.startsWith("src-tauri/"),
);
const stateDocsChanged = [...changed].some((file) => KEY_STATE_DOCS.has(file));

if (codeChanged && !stateDocsChanged) {
  console.error(
    "[docs-guard] Code changed under src/ or src-tauri/, but no project-state docs were updated.",
  );
  console.error(
    "[docs-guard] Update at least one of: docs/tasks/{active,backlog,blocked,completed}.md or docs/memory/current.md",
  );
  process.exit(1);
}

// ── Architecture doc sync ────────────────────────────────────────────────────

const archDocChanged = changed.has("docs/architecture.md");

// Hard check: new module files added → architecture.md must be updated.
const newArchFiles = [...added].filter((file) =>
  ARCH_NEW_FILE_PREFIXES.some((prefix) => file.startsWith(prefix)),
);
if (newArchFiles.length > 0 && !archDocChanged) {
  console.error("[docs-guard] New architecture module(s) added but docs/architecture.md was not updated:");
  for (const file of newArchFiles) {
    console.error(`  + ${file}`);
  }
  console.error("[docs-guard] Add the new module to the relevant section in docs/architecture.md.");
  process.exit(1);
}

// Advisory: wiring files modified → remind but do not block.
const wiringChanged = [...changed].filter((file) => ARCH_WIRING_FILES.has(file));
if (wiringChanged.length > 0 && !archDocChanged) {
  console.warn("[docs-guard] Advisory: architecture wiring file(s) modified — update docs/architecture.md if the change is significant:");
  for (const file of wiringChanged) {
    console.warn(`  ~ ${file}`);
  }
}

const missingFrontmatter = REQUIRED_FRONTMATTER.filter(
  (file) => !hasRequiredFrontmatter(path.join(ROOT, file)),
);
if (missingFrontmatter.length > 0) {
  console.error("[docs-guard] Missing required frontmatter fields in:");
  for (const file of missingFrontmatter) {
    console.error(`  - ${file}`);
  }
  process.exit(1);
}

const indexPath = path.join(ROOT, "docs", "index.md");
if (!fs.existsSync(indexPath)) {
  console.error("[docs-guard] docs/index.md is missing.");
  process.exit(1);
}

const indexContent = fs.readFileSync(indexPath, "utf8");
if (!indexContent.includes("AUTO-GENERATED by scripts/docs-sync.mjs")) {
  console.error("[docs-guard] docs/index.md is not synchronized. Run npm run docs:sync.");
  process.exit(1);
}

console.log("[docs-guard] Documentation guard checks passed.");
