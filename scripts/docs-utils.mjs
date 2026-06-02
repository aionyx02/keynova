import fs from "node:fs";
import path from "node:path";

export const ROOT = process.cwd();

export function normalize(input) {
  return input.replace(/\\/g, "/");
}

export function repoPath(filePath) {
  return normalize(path.relative(ROOT, filePath));
}

export function todayTaipei() {
  return new Intl.DateTimeFormat("en-CA", { timeZone: "Asia/Taipei" }).format(new Date());
}

export function parseFrontmatter(text) {
  const normalized = text.replace(/^\uFEFF/, "").replace(/\r\n/g, "\n");
  const match = normalized.match(/^---\n([\s\S]*?)\n---\n?/);
  if (!match) {
    return { frontmatter: new Map(), body: normalized, hasFrontmatter: false };
  }

  const frontmatter = new Map();
  for (const line of match[1].split("\n")) {
    const kv = line.match(/^([a-zA-Z0-9_]+):\s*(.*)$/);
    if (kv) {
      frontmatter.set(kv[1], kv[2]);
    }
  }

  return {
    frontmatter,
    body: normalized.slice(match[0].length),
    hasFrontmatter: true,
  };
}

export function walkMarkdown(dirPath) {
  if (!fs.existsSync(dirPath)) {
    return [];
  }

  const files = [];
  for (const entry of fs.readdirSync(dirPath, { withFileTypes: true })) {
    const fullPath = path.join(dirPath, entry.name);
    if (entry.isDirectory()) {
      files.push(...walkMarkdown(fullPath));
    } else if (entry.isFile() && entry.name.endsWith(".md")) {
      files.push(fullPath);
    }
  }
  return files;
}

export function sessionTemplate(date) {
  return `---
type: session_log
status: archive
priority: p3
updated: ${date}
context_policy: on_demand
owner: project
---

# Session ${date}

## Highlights

- (fill in)

## Detailed Notes

## Commits

## Follow-ups
`;
}

export function ensureSessionFile(date = todayTaipei()) {
  const filePath = path.join(ROOT, "docs", "memory", "sessions", `${date}.md`);
  if (!fs.existsSync(filePath)) {
    fs.mkdirSync(path.dirname(filePath), { recursive: true });
    fs.writeFileSync(filePath, `${sessionTemplate(date)}\n`, "utf8");
  }
  return filePath;
}
