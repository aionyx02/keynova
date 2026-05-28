#!/usr/bin/env node

import fs from "node:fs";
import path from "node:path";
import { ROOT, ensureSessionFile, repoPath, todayTaipei } from "./docs-utils.mjs";

const target = process.argv[2];

if (!target) {
  console.error("usage: npm run docs:extract-narrative -- <markdown-file>");
  process.exit(1);
}

const targetPath = path.resolve(ROOT, target);
if (!fs.existsSync(targetPath)) {
  console.error(`[docs-extract-narrative] File not found: ${target}`);
  process.exit(1);
}

const original = fs.readFileSync(targetPath, "utf8").replace(/\r\n/g, "\n");
const lines = original.split("\n");
const migrated = [];
const kept = [];

let current = null;

function startsDatedBlock(line) {
  return /^[-*]\s+\d{4}-\d{2}-\d{2}\b/.test(line) || /^##+\s+\d{4}-\d{2}-\d{2}\b/.test(line);
}

function belongsToCurrent(line) {
  return current && (line.trim() === "" || /^\s+/.test(line));
}

for (const line of lines) {
  if (startsDatedBlock(line)) {
    if (current) {
      migrated.push(current);
    }
    current = [line];
  } else if (belongsToCurrent(line)) {
    current.push(line);
  } else {
    if (current) {
      migrated.push(current);
      current = null;
    }
    kept.push(line);
  }
}

if (current) {
  migrated.push(current);
}

if (migrated.length === 0) {
  console.log(`[docs-extract-narrative] No dated narrative entries found in ${target}.`);
  process.exit(0);
}

const date = todayTaipei();
const sessionPath = ensureSessionFile(date);
const migratedText = migrated.map((block) => block.join("\n").trim()).join("\n\n");

fs.appendFileSync(
  sessionPath,
  `\n## Migrated From ${repoPath(targetPath)}\n\n${migratedText}\n`,
  "utf8",
);
fs.writeFileSync(targetPath, kept.join("\n").replace(/\n{3,}/g, "\n\n"), "utf8");

console.log(
  `[docs-extract-narrative] Migrated ${migrated.length} entries to ${repoPath(sessionPath)}.`,
);
