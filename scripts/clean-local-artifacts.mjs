#!/usr/bin/env node

import fs from "node:fs";
import path from "node:path";

const ROOT = process.cwd();

const DIRS_TO_REMOVE = ["dist", "coverage"];
const FILE_PREFIXES = [".codex-", ".keynova-terminal-debug"];
const FILE_SUFFIXES = [".log"];
const NESTED_FILE_DIRS = ["src-tauri"];

function removePath(relativePath) {
  const target = path.resolve(ROOT, relativePath);
  if (!target.startsWith(ROOT)) {
    throw new Error(`Refusing to delete path outside repo: ${target}`);
  }
  if (!fs.existsSync(target)) {
    return false;
  }
  fs.rmSync(target, { recursive: true, force: true });
  return true;
}

function shouldRemoveFile(fileName) {
  return (
    FILE_PREFIXES.some((prefix) => fileName.startsWith(prefix)) &&
    FILE_SUFFIXES.some((suffix) => fileName.endsWith(suffix))
  );
}

function removeMatchingFiles(relativeDir) {
  const targetDir = path.resolve(ROOT, relativeDir);
  if (!targetDir.startsWith(ROOT) || !fs.existsSync(targetDir)) {
    return [];
  }

  const removed = [];
  for (const entry of fs.readdirSync(targetDir, { withFileTypes: true })) {
    if (!entry.isFile() || !shouldRemoveFile(entry.name)) {
      continue;
    }
    const fullPath = path.join(targetDir, entry.name);
    fs.rmSync(fullPath, { force: true });
    removed.push(path.relative(ROOT, fullPath));
  }
  return removed;
}

const removedDirs = DIRS_TO_REMOVE.filter(removePath);
const removedFiles = [".", ...NESTED_FILE_DIRS].flatMap(removeMatchingFiles);

const totalRemoved = removedDirs.length + removedFiles.length;
if (totalRemoved === 0) {
  console.log("[clean-local] No local artifacts found.");
  process.exit(0);
}

for (const dir of removedDirs) {
  console.log(`[clean-local] Removed ${dir}/`);
}
for (const file of removedFiles) {
  console.log(`[clean-local] Removed ${file}`);
}
