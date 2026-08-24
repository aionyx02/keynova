// Launcher settings that gate palette rendering + search behavior.
//
// Reads `setting.list_all` on mount and re-reads when a `config-reloaded`
// event fires for any of:
//   - launcher.max_results        → routed via `onMaxResultsChange`
//   - launcher.theme              → applied to the root element, not rendered
//   - search.preview_enabled      → kept in hook state
//   - search.show_rank_breakdown  → kept in hook state
//
// `onMaxResultsChange` is a callback (not a piece of returned state) so the
// search stream hook can keep ownership of the search limit ref.

import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";

import type { DispatchFn } from "../../../context/IPCContext";
import { IPC } from "../../../ipc/routes";
import type { SettingEntry } from "../../../ipc/types";
import { applyTheme, resolveTheme } from "../../../shared/theme";

interface ConfigReloadedPayload {
  changed_keys: string[];
}

const WATCHED_KEYS: ReadonlyArray<string> = [
  "launcher.theme",
  "launcher.max_results",
  "search.preview_enabled",
  "search.show_rank_breakdown",
  "launcher.show_capability_hint",
];

export interface UseLauncherSettingsDeps {
  dispatch: DispatchFn;
  /** Called whenever `launcher.max_results` resolves to a positive integer. */
  onMaxResultsChange: (limit: number) => void;
}

export interface UseLauncherSettings {
  previewEnabled: boolean;
  showRankBreakdown: boolean;
  /** Toggles the empty-palette capability hint line. */
  showCapabilityHint: boolean;
}

export function useLauncherSettings({
  dispatch,
  onMaxResultsChange,
}: UseLauncherSettingsDeps): UseLauncherSettings {
  const [previewEnabled, setPreviewEnabled] = useState(true);
  const [showRankBreakdown, setShowRankBreakdown] = useState(true);
  const [showCapabilityHint, setShowCapabilityHint] = useState(true);

  useEffect(() => {
    if (!window.__TAURI_INTERNALS__) return;

    async function refresh() {
      try {
        const entries = await dispatch<SettingEntry[]>(IPC.SETTING_LIST_ALL);
        const maxResults = entries.find((e) => e.key === "launcher.max_results")?.value;
        const parsed = Number.parseInt(maxResults ?? "", 10);
        if (Number.isFinite(parsed) && parsed > 0) {
          onMaxResultsChange(parsed);
        }
        const preview = entries.find((e) => e.key === "search.preview_enabled")?.value;
        if (preview !== undefined) {
          setPreviewEnabled(preview !== "false");
        }
        const rank = entries.find((e) => e.key === "search.show_rank_breakdown")?.value;
        if (rank !== undefined) {
          setShowRankBreakdown(rank !== "false");
        }
        const hint = entries.find((e) => e.key === "launcher.show_capability_hint")?.value;
        if (hint !== undefined) {
          setShowCapabilityHint(hint !== "false");
        }
        // Unlike every other key here, the theme has no React state: it is
        // written straight onto <html>. An absent or unknown value resolves to
        // the default rather than leaving the previous theme in place, so
        // clearing the setting actually reverts the palette.
        const theme = entries.find((e) => e.key === "launcher.theme")?.value;
        applyTheme(resolveTheme(theme));
      } catch {
        // keep current values
      }
    }

    void refresh();
    const unlisten = listen<ConfigReloadedPayload>("config-reloaded", (event) => {
      const keys = event.payload.changed_keys;
      if (keys.length === 0 || WATCHED_KEYS.some((k) => keys.includes(k))) {
        void refresh();
      }
    });

    return () => {
      unlisten.then((fn) => fn());
    };
    // dispatch / onMaxResultsChange intentionally omitted; same rationale as
    // other Tauri-listener hooks (useIPC dispatch is fresh per render and the
    // caller controls onMaxResultsChange stability via useCallback).
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  return { previewEnabled, showRankBreakdown, showCapabilityHint };
}
