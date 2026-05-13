#!/usr/bin/env node

import { execSync } from "node:child_process";

try {
  execSync("git config core.hooksPath .githooks", { stdio: "ignore" });
  console.log("[hooks] core.hooksPath set to .githooks");
} catch (error) {
  console.warn("[hooks] Unable to set core.hooksPath automatically.");
  console.warn(String(error));
}
