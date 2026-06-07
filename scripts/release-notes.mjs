import crypto from "node:crypto";
import fs from "node:fs";
import path from "node:path";

import { parseFrontmatter, ROOT } from "./docs-utils.mjs";

function fail(message) {
  console.error(`[release-notes] ${message}`);
  process.exit(1);
}

function readJson(relativePath) {
  const filePath = path.join(ROOT, relativePath);
  return JSON.parse(fs.readFileSync(filePath, "utf8"));
}

const packageVersion = readJson("package.json").version;
const tauriVersion = readJson("src-tauri/tauri.conf.json").version;

if (!packageVersion || packageVersion !== tauriVersion) {
  fail(
    `package.json (${packageVersion ?? "missing"}) and tauri.conf.json ` +
      `(${tauriVersion ?? "missing"}) versions must match`,
  );
}

const releaseTag = `v${packageVersion}`;
const writesGitHubOutput = process.argv.includes("--github-output");
if (writesGitHubOutput && process.env.GITHUB_REF_TYPE !== "tag") {
  fail("release output must be generated from a tag ref");
}
if (
  process.env.GITHUB_REF_TYPE === "tag" &&
  process.env.GITHUB_REF_NAME !== releaseTag
) {
  fail(`tag ${process.env.GITHUB_REF_NAME} does not match app version ${releaseTag}`);
}

const notesPath = path.join(ROOT, "docs", "release-notes", `${releaseTag}.md`);
if (!fs.existsSync(notesPath)) {
  fail(`missing docs/release-notes/${releaseTag}.md`);
}

const { body, frontmatter, hasFrontmatter } = parseFrontmatter(
  fs.readFileSync(notesPath, "utf8"),
);
const releaseBody = body.trim();

if (!hasFrontmatter || frontmatter.get("type") !== "release_notes") {
  fail(`${path.relative(ROOT, notesPath)} must have type: release_notes frontmatter`);
}
if (!releaseBody.startsWith(`# Keynova ${releaseTag}`)) {
  fail(`${path.relative(ROOT, notesPath)} must start with "# Keynova ${releaseTag}"`);
}

if (writesGitHubOutput) {
  const outputPath = process.env.GITHUB_OUTPUT;
  if (!outputPath) {
    fail("GITHUB_OUTPUT is required with --github-output");
  }

  const delimiter = `RELEASE_NOTES_${crypto.randomUUID()}`;
  fs.appendFileSync(
    outputPath,
    [
      `release_tag=${releaseTag}`,
      `release_body<<${delimiter}`,
      releaseBody,
      delimiter,
      "",
    ].join("\n"),
    "utf8",
  );
} else {
  console.log(`[release-notes] ${releaseTag} ready (${releaseBody.length} chars)`);
}
