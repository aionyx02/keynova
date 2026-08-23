#!/usr/bin/env node

// REF.7.B — Hard-fail file-size drift detector for the REF.6 refactor.
//
// This is a source-file guard and has nothing to do with documentation; the
// Markdown size guard it once mirrored was removed with the rest of the docs
// tooling. Ceilings are NOT the docx §8 hard
// targets (any handler < 600 lines / any component < 400 lines). Instead each
// file's ceiling is its current size rounded up to the next 1 KiB boundary +
// a 10 % buffer, so the script catches regressive bloat without forcing an
// inline refactor of files that REF.6 documented as accepted overshoots
// (CommandPalette.tsx, agent/mod.rs, the migrated panels, etc.).
//
// REF.8 ratchets these ceilings down once the AiPanel deletion + agent
// runtime trim decisions land.

import fs from "node:fs";
import path from "node:path";
import { ROOT } from "./docs-utils.mjs";

// Keep entries sorted by relative path. Bytes — not lines — because the
// underlying detection (`fs.statSync(path).size`) is byte-based and avoids a
// per-file read for line counting.
const LIMITS = new Map([
  ["src-tauri/src/handlers/builtin_cmd.rs", 44_032],
  // Bumped 43008→46080: post-v0.6.0 PROFILE.1 + PRODUCT.4 ranking wiring grew it;
  // the 2026-06-10 PROFILE.2 pin removal clawed back ~1.7 KB but not under 43008.
  // Recompression still declined per developer decision — the split is the deferred
  // path ([[project_ref2_p5_landing]]) — so bless current size + ~1 KiB headroom.
  ["src/components/CommandPalette.tsx", 46_080],
  ["src/features/settings/SettingPanel.tsx", 18_432],
  ["src/features/translation/TranslationPanel.tsx", 21_504],
]);

const failures = [];
const skipped = [];

for (const [file, limit] of LIMITS) {
  const fullPath = path.join(ROOT, file);
  if (!fs.existsSync(fullPath)) {
    // The file may have been deleted (REF.8 path). Record it so the operator
    // can prune the entry, but do not fail the build for an intentional removal.
    skipped.push(file);
    continue;
  }
  const size = fs.statSync(fullPath).size;
  if (size > limit) {
    failures.push({ file, size, limit });
  }
}

if (failures.length > 0) {
  console.error("[code-guard-size] File size ceilings exceeded:");
  for (const failure of failures) {
    const overBy = failure.size - failure.limit;
    console.error(`  - ${failure.file}: ${failure.size}B > ${failure.limit}B (+${overBy}B)`);
  }
  console.error(
    "[code-guard-size] Either trim the file or raise its ceiling in scripts/code-guard-size.mjs with a one-line justification.",
  );
  process.exit(1);
}

if (skipped.length > 0) {
  console.warn(
    `[code-guard-size] Skipped ${skipped.length} missing file(s); consider removing from LIMITS:`,
  );
  for (const file of skipped) {
    console.warn(`  - ${file}`);
  }
}

console.log("[code-guard-size] File size ceilings passed.");
