#!/usr/bin/env node

// REF.7.B — Bench harness for the `ai_capability` live-Ollama smoke tests.
//
// Shells out `cargo test --features live-ai ...` N times, parses the
// `[ai_capability_live] model=<X> <test_name> latency = <ms> ms` lines that
// `src-tauri/src/core/ai_capability/live_tests.rs` already emits, aggregates
// per-capability P50 / P95 / min / max, and prints either a human-readable
// table (default) or a JSON payload (`--json`) for CI ingest.
//
// Usage:
//   node scripts/bench-ai-capability.mjs                          # 10 runs, model = $KEYNOVA_LIVE_AI_MODEL or qwen2.5:7b
//   node scripts/bench-ai-capability.mjs --runs 3 --model qwen3:0.6b
//   node scripts/bench-ai-capability.mjs --runs 10 --json > bench.json
//
// REF.7.D records the formal qwen2.5:7b reading; REF.7.C copies it into
// ADR-0029 §8.

import { spawnSync } from "node:child_process";

const args = parseArgs(process.argv.slice(2));
const runs = args.runs ?? 10;
const model = args.model ?? process.env.KEYNOVA_LIVE_AI_MODEL ?? "qwen2.5:7b";
const jsonOutput = Boolean(args.json);

if (!Number.isInteger(runs) || runs <= 0) {
  console.error(`[bench-ai] --runs must be a positive integer (got ${args.runs})`);
  process.exit(2);
}

// Per-capability raw latency samples across all runs.
/** @type {Map<string, number[]>} */
const samples = new Map();

for (let i = 0; i < runs; i++) {
  if (!jsonOutput) {
    process.stderr.write(`[bench-ai] run ${i + 1}/${runs} (model=${model})\n`);
  }
  const result = spawnSync(
    "cargo",
    [
      "test",
      "--features",
      "live-ai",
      "--manifest-path",
      "src-tauri/Cargo.toml",
      "--",
      "--ignored",
      "ai_capability_live",
      "--nocapture",
    ],
    {
      env: { ...process.env, KEYNOVA_LIVE_AI_MODEL: model },
      encoding: "utf8",
      // The cargo test harness can take a while on cold model load; give it
      // generous headroom rather than hanging the parent on a fixed timeout.
      timeout: 10 * 60 * 1000,
      maxBuffer: 32 * 1024 * 1024,
    },
  );

  if (result.status !== 0) {
    console.error(`[bench-ai] cargo test failed on run ${i + 1}:`);
    console.error(result.stderr ?? "");
    console.error(result.stdout ?? "");
    process.exit(result.status ?? 1);
  }

  const stdout = result.stdout ?? "";
  // Lazy capture of the capability name so the streaming explain line
  // ("explain STREAM latency = X ms, chunks = N") matches alongside the
  // non-streaming variants. The trailing portion after "ms" is intentionally
  // ignored so the parser is forward-compatible with any extra metrics the
  // harness may append.
  const lineRegex = /^\[ai_capability_live\] model=(\S+) (.+?) latency = (\d+) ms/gm;
  let match;
  let matched = 0;
  while ((match = lineRegex.exec(stdout)) !== null) {
    // Normalize "explain STREAM" -> "explain_stream" so the table keys stay
    // consistent across runs and JSON keys are CI-friendly.
    const capability = match[2].trim().toLowerCase().replace(/\s+/g, "_");
    const ms = Number.parseInt(match[3], 10);
    if (Number.isFinite(ms)) {
      const bucket = samples.get(capability) ?? [];
      bucket.push(ms);
      samples.set(capability, bucket);
      matched += 1;
    }
  }

  if (matched === 0) {
    console.error(`[bench-ai] no latency lines parsed on run ${i + 1}; stdout tail:`);
    console.error(stdout.slice(-2000));
    process.exit(1);
  }
}

const results = [];
for (const [capability, raw] of [...samples.entries()].sort()) {
  raw.sort((a, b) => a - b);
  results.push({
    capability,
    runs: raw.length,
    p50_ms: percentile(raw, 0.5),
    p95_ms: percentile(raw, 0.95),
    min_ms: raw[0],
    max_ms: raw[raw.length - 1],
    raw_ms: raw,
  });
}

if (jsonOutput) {
  process.stdout.write(
    `${JSON.stringify({ model, runs, results }, null, 2)}\n`,
  );
} else {
  printTable(model, runs, results);
}

function parseArgs(argv) {
  const out = {};
  for (let i = 0; i < argv.length; i++) {
    const token = argv[i];
    if (token === "--json") {
      out.json = true;
    } else if (token === "--runs") {
      out.runs = Number.parseInt(argv[++i], 10);
    } else if (token === "--model") {
      out.model = argv[++i];
    } else if (token === "--help" || token === "-h") {
      console.log(
        "Usage: bench-ai-capability.mjs [--runs N] [--model NAME] [--json]",
      );
      process.exit(0);
    } else {
      console.error(`[bench-ai] unknown arg: ${token}`);
      process.exit(2);
    }
  }
  return out;
}

function percentile(sortedAsc, p) {
  if (sortedAsc.length === 0) return 0;
  // Linear interpolation between closest ranks, matching the formula used by
  // most observability tooling. Matches `numpy.percentile(linear)`.
  const rank = (sortedAsc.length - 1) * p;
  const lo = Math.floor(rank);
  const hi = Math.ceil(rank);
  if (lo === hi) return sortedAsc[lo];
  const fraction = rank - lo;
  return Math.round(sortedAsc[lo] + (sortedAsc[hi] - sortedAsc[lo]) * fraction);
}

function printTable(modelName, runCount, rows) {
  const headers = ["capability", "runs", "p50_ms", "p95_ms", "min_ms", "max_ms"];
  const data = rows.map((r) => [
    r.capability,
    String(r.runs),
    String(r.p50_ms),
    String(r.p95_ms),
    String(r.min_ms),
    String(r.max_ms),
  ]);
  const widths = headers.map((h, idx) =>
    Math.max(h.length, ...data.map((row) => row[idx].length)),
  );
  const fmt = (cells) =>
    cells.map((c, idx) => c.padEnd(widths[idx])).join("  ");

  process.stdout.write(`\n[bench-ai] model=${modelName}  runs=${runCount}\n`);
  process.stdout.write(`${fmt(headers)}\n`);
  process.stdout.write(`${widths.map((w) => "-".repeat(w)).join("  ")}\n`);
  for (const row of data) {
    process.stdout.write(`${fmt(row)}\n`);
  }
}
