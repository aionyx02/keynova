#!/usr/bin/env node

// Rejects commits that would put anyone but the maintainer on the repository's
// contributor list: a foreign author, or a `Co-authored-by:` line anywhere in
// the message. GitHub credits both, and once such a commit reaches `main` the
// only way to take the name back off is a history rewrite — see the
// 2026-09-17 entry in docs/state.md.
//
// The line is matched anywhere in the body, not only in the trailer block:
// a squash merge concatenates the branch's messages, so a trailer from an
// inner commit lands mid-message and GitHub still credits it.
//
// Usage: node scripts/check-commit-authors.mjs <base>..<head>

import { execFileSync } from "node:child_process";

const ALLOWED_AUTHORS = new Set(["aionyxhuang@gmail.com"]);
const CO_AUTHOR_LINE = /^[ \t]*co-authored-by:.*$/gim;

const range = process.argv[2];
if (!range || !range.includes("..")) {
  console.error("usage: node scripts/check-commit-authors.mjs <base>..<head>");
  process.exit(2);
}

// 0x1f between fields, 0x1e between commits: neither occurs in an email address
// or a commit message.
const output = execFileSync("git", ["log", "--format=%H%x1f%ae%x1f%B%x1e", range], {
  encoding: "utf8",
  maxBuffer: 64 * 1024 * 1024,
});
const commits = output
  .split("\x1e")
  .map((record) => record.replace(/^\n/, ""))
  .filter((record) => record.length > 0)
  .map((record) => {
    const [sha, email, body] = record.split("\x1f");
    return { sha: sha.slice(0, 7), email: email.toLowerCase(), body };
  });

const failures = [];
for (const { sha, email, body } of commits) {
  if (!ALLOWED_AUTHORS.has(email)) {
    failures.push(`${sha} is authored by ${email}`);
  }
  for (const [line] of body.matchAll(CO_AUTHOR_LINE)) {
    failures.push(`${sha} carries "${line.trim()}"`);
  }
}

if (failures.length > 0) {
  for (const failure of failures) {
    console.log(`::error::${failure}`);
  }
  console.error(
    `\n${failures.length} problem(s) across ${commits.length} commit(s) in ${range}.` +
      "\nReword or re-author these commits before merging; a branch cut before the" +
      "\n2026-09-17 history rewrite must be rebased onto the current base first.",
  );
  process.exit(1);
}

console.log(`${commits.length} commit(s) in ${range}: maintainer-authored, no co-author lines.`);
