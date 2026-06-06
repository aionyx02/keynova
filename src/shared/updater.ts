// In-app update check (PRODUCT.3 / ADR-0050).
//
// Thin wrapper over @tauri-apps/plugin-updater that degrades gracefully when the
// updater is not yet configured (no `plugins.updater` endpoints/pubkey in
// tauri.conf.json). This keeps the `/update` command safe to ship before the
// signing keypair exists: it reports "not configured" instead of throwing.
//
// MVP is check-only ("suggest before execute"): it reports availability and
// release notes but does not auto-download/install. Install + relaunch can be a
// follow-up once the signed update flow is exercised end-to-end.

import { check } from "@tauri-apps/plugin-updater";
import { getVersion } from "@tauri-apps/api/app";

export type UpdateCheckResult =
  | { status: "available"; version: string; currentVersion: string; notes?: string }
  | { status: "current"; currentVersion: string }
  | { status: "unconfigured" }
  | { status: "error"; message: string };

export interface UpdateStrings {
  available: string; // uses {version} and {current}
  current: string; // uses {current}
  unconfigured: string;
  error: string; // uses {message}
}

export async function checkForUpdate(): Promise<UpdateCheckResult> {
  let currentVersion = "";
  try {
    currentVersion = await getVersion();
  } catch {
    // Version is best-effort for display only.
  }

  try {
    const update = await check();
    if (update) {
      return {
        status: "available",
        version: update.version,
        currentVersion: update.currentVersion || currentVersion,
        notes: update.body || undefined,
      };
    }
    return { status: "current", currentVersion };
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    // The plugin throws when `plugins.updater` is absent or missing
    // endpoints/pubkey — treat that as "not configured", not a hard error.
    if (/updater|endpoint|config|pubkey/i.test(message)) {
      return { status: "unconfigured" };
    }
    return { status: "error", message };
  }
}

export function formatUpdateMessage(result: UpdateCheckResult, strings: UpdateStrings): string {
  switch (result.status) {
    case "available": {
      const head = strings.available
        .replace("{version}", result.version)
        .replace("{current}", result.currentVersion || "?");
      return result.notes ? `${head}\n\n${result.notes}` : head;
    }
    case "current":
      return strings.current.replace("{current}", result.currentVersion || "?");
    case "unconfigured":
      return strings.unconfigured;
    case "error":
      return strings.error.replace("{message}", result.message);
  }
}
