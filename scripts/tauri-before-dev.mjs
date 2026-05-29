#!/usr/bin/env node

import net from "node:net";
import fs from "node:fs";
import path from "node:path";
import { spawn, spawnSync } from "node:child_process";

const DEV_PORT = 1420;
const DEV_HOST = process.env.TAURI_DEV_HOST;
const ROOT_DIR = process.cwd();
const DEBUG_BIN_DIR = path.join(ROOT_DIR, "src-tauri", "target", "debug");
const DEBUG_APP = path.join(
  DEBUG_BIN_DIR,
  process.platform === "win32" ? "tauri-app.exe" : "tauri-app",
);
const DEBUG_CLI = path.join(
  DEBUG_BIN_DIR,
  process.platform === "win32" ? "keynova.exe" : "keynova",
);
const DEV_ORIGINS = [
  DEV_HOST ? `http://${DEV_HOST}:${DEV_PORT}` : null,
  `http://127.0.0.1:${DEV_PORT}`,
  `http://localhost:${DEV_PORT}`,
].filter((value, index, list) => value && list.indexOf(value) === index);
const DEV_MARKER = '<meta name="keynova-dev-server" content="true" />';

function sleep(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

async function fetchText(url, timeoutMs = 1200) {
  const controller = new AbortController();
  const timer = setTimeout(() => controller.abort(), timeoutMs);
  try {
    const response = await fetch(url, {
      signal: controller.signal,
      headers: { Accept: "text/html" },
    });
    return await response.text();
  } finally {
    clearTimeout(timer);
  }
}

async function portAcceptsConnections(host, port, timeoutMs = 800) {
  return new Promise((resolve) => {
    const socket = net.createConnection({ host, port });
    const finish = (value) => {
      socket.destroy();
      resolve(value);
    };
    socket.setTimeout(timeoutMs);
    socket.once("connect", () => finish(true));
    socket.once("timeout", () => finish(false));
    socket.once("error", () => finish(false));
  });
}

async function detectServer() {
  let sawUnexpectedHttp = false;

  for (const origin of DEV_ORIGINS) {
    try {
      const text = await fetchText(origin);
      if (text.includes(DEV_MARKER)) {
        return { kind: "keynova", origin };
      }
      sawUnexpectedHttp = true;
    } catch {
      // Try the next origin; TCP checks below will distinguish "free" vs "busy".
    }
  }

  if (sawUnexpectedHttp) {
    return { kind: "other_http" };
  }

  for (const host of ["127.0.0.1", "localhost", DEV_HOST].filter(Boolean)) {
    if (await portAcceptsConnections(host, DEV_PORT)) {
      return { kind: "occupied" };
    }
  }

  return { kind: "missing" };
}

async function waitForKeynovaServer(retries = 40, delayMs = 250) {
  for (let attempt = 0; attempt < retries; attempt += 1) {
    const detected = await detectServer();
    if (detected.kind === "keynova") {
      return detected.origin;
    }
    await sleep(delayMs);
  }
  return null;
}

function maybeRequestRunningAppShutdown() {
  if (!fs.existsSync(DEBUG_CLI)) {
    return false;
  }

  const result = spawnSync(DEBUG_CLI, ["down"], {
    stdio: "pipe",
    encoding: "utf8",
    timeout: 2500,
  });

  return result.status === 0;
}

async function maybeStopExistingDebugApp() {
  if (!fs.existsSync(DEBUG_APP)) {
    return;
  }

  if (!maybeRequestRunningAppShutdown()) {
    return;
  }

  await sleep(1500);
}

function spawnDevServer() {
  const command = process.platform === "win32" ? "npm.cmd" : "npm";
  const child = spawn(command, ["run", "dev"], {
    stdio: "inherit",
    shell: false,
  });

  const forwardSignal = (signal) => {
    if (!child.killed) {
      child.kill(signal);
    }
  };

  process.once("SIGINT", () => forwardSignal("SIGINT"));
  process.once("SIGTERM", () => forwardSignal("SIGTERM"));

  child.once("exit", async (code) => {
    if (code === 0) {
      process.exit(0);
    }

    const reusedOrigin = await waitForKeynovaServer(8, 200);
    if (reusedOrigin) {
      console.log(`[tauri-before-dev] Reusing existing Keynova dev server at ${reusedOrigin}`);
      process.exit(0);
    }

    process.exit(code ?? 1);
  });
}

await maybeStopExistingDebugApp();

const detected = await detectServer();
if (detected.kind === "keynova") {
  console.log(`[tauri-before-dev] Reusing existing Keynova dev server at ${detected.origin}`);
  process.exit(0);
}

if (detected.kind === "other_http" || detected.kind === "occupied") {
  console.error(
    `[tauri-before-dev] Port ${DEV_PORT} is already in use by a non-Keynova service. ` +
      `Stop that process or free the port, then retry.`,
  );
  process.exit(1);
}

spawnDevServer();
