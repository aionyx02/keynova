// File and search-result action handlers.
//
// Bundles the IPC-backed file actions launched from the result row (primary
// launch + 9-way secondary action switch + first-secondary shortcut) into a
// single hook so the palette body stops being a 250-line `switch` followed by
// six standalone async functions.
//
// Render-time kill set: the `refreshAfterFileMutation`
// helper still calls `markPathDeleted` + `setSelected(0)`. Re-firing the
// search would resurrect trashed paths because Everything still indexes
// Recycle Bin entries; the kill set is the only authoritative gate. The
// behaviour is preserved verbatim — this extraction is a move, not a rewrite.

import { useCallback } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { revealItemInDir } from "@tauri-apps/plugin-opener";

import type { DispatchFn } from "../../../context/IPCContext";
import { IPC } from "../../../ipc/routes";
import type { ActionRef, SearchResult } from "../../../types/search";
import type { UnifiedResult } from "../../../types/unified-result";
import type { BuiltinCommandResult } from "../../../hooks/useCommands";
import {
  basenameFromPath,
  parentDirFromPath,
  type SecondaryActionId,
} from "../../../utils/secondaryActions";
import type { SecondaryInlineInput } from "./useSecondaryMenu";

interface SecondaryActionInfo {
  action_ref: ActionRef;
  label: string;
  risk: "low" | "medium" | "high";
}

function isCopyableLocationResult(result: SearchResult | null) {
  return result?.kind === "app" || result?.kind === "file" || result?.kind === "folder";
}

async function hideWindow() {
  try {
    await getCurrentWindow().hide();
  } catch {
    // non-Tauri env: no-op
  }
}

export interface UseFileActionsDeps {
  dispatch: DispatchFn;
  flashCopiedPath: (path: string) => void;
  flashCopyHint: (hint: string, durationMs?: number) => void;
  clearCopyHint: () => void;
  setQuery: (q: string) => void;
  /** Palette state now holds `UnifiedResult[]`; this hook only ever
   * clears results, so the prop matches the canonical setter signature. */
  setResults: React.Dispatch<React.SetStateAction<UnifiedResult[]>>;
  setCmdResult: (result: BuiltinCommandResult | null) => void;
  setSelected: React.Dispatch<React.SetStateAction<number>>;
  markPathDeleted: (path: string) => void;
  closeSecondaryMenu: () => void;
  setExpandedMetadata: React.Dispatch<React.SetStateAction<boolean>>;
  inlineInput: SecondaryInlineInput | null;
  setInlineInput: React.Dispatch<React.SetStateAction<SecondaryInlineInput | null>>;
  pendingConfirm: SecondaryActionId | null;
  setPendingConfirm: React.Dispatch<React.SetStateAction<SecondaryActionId | null>>;
}

export interface UseFileActions {
  launchResult: (result: SearchResult) => Promise<void>;
  copyResultLocation: (result: SearchResult) => Promise<void>;
  handleSecondaryAction: (id: SecondaryActionId, result: SearchResult) => Promise<void>;
  runFirstSecondary: (result: SearchResult) => Promise<void>;
}

export function useFileActions(deps: UseFileActionsDeps): UseFileActions {
  const {
    dispatch,
    flashCopiedPath,
    flashCopyHint,
    clearCopyHint,
    setQuery,
    setResults,
    setCmdResult,
    setSelected,
    markPathDeleted,
    closeSecondaryMenu,
    setExpandedMetadata,
    inlineInput,
    setInlineInput,
    pendingConfirm,
    setPendingConfirm,
  } = deps;

  const showHint = useCallback(
    (message: string, durationMs = 1500) => flashCopyHint(message, durationMs),
    [flashCopyHint],
  );

  const refreshAfterFileMutation = useCallback(
    (droppedPath: string) => {
      markPathDeleted(droppedPath);
      setSelected(0);
    },
    [markPathDeleted, setSelected],
  );

  const copyResultLocation = useCallback(
    async (result: SearchResult) => {
      if (!isCopyableLocationResult(result)) return;
      try {
        await navigator.clipboard.writeText(result.path);
        flashCopiedPath(result.path);
      } catch {
        clearCopyHint();
      }
    },
    [flashCopiedPath, clearCopyHint],
  );

  const copyText = useCallback(
    async (text: string, hint: string) => {
      try {
        await navigator.clipboard.writeText(text);
        flashCopyHint(hint);
      } catch {
        clearCopyHint();
      }
    },
    [flashCopyHint, clearCopyHint],
  );

  const revealResult = useCallback(
    async (result: SearchResult) => {
      if (!result.path) return;
      try {
        await revealItemInDir(result.path);
      } catch {
        // Plugin failure (e.g. missing permission, non-existent path) — surface in footer.
        flashCopyHint("Reveal failed", 1500);
      }
    },
    [flashCopyHint],
  );

  const launchResult = useCallback(
    async (result: SearchResult) => {
      try {
        await dispatch(IPC.SEARCH_RECORD_SELECTION, {
          source: result.source ?? result.kind,
          path: result.path,
        }).catch(() => {});
        if (result.primary_action) {
          const actionResult = await dispatch<{
            type: string;
            name?: string;
            initial_args?: string;
          }>("action.run", { action_ref: result.primary_action });
          if (actionResult.type === "panel" && actionResult.name) {
            setCmdResult({
              text: actionResult.initial_args ?? "",
              ui_type: { type: "Panel", value: actionResult.name },
            });
            return;
          }
        } else {
          await dispatch(IPC.LAUNCHER_LAUNCH, { path: result.path });
        }
      } finally {
        if (result.kind === "app" || result.kind === "file" || result.kind === "folder") {
          setQuery("");
          setResults([]);
          void hideWindow();
        }
      }
    },
    // dispatch is intentionally omitted — useIPC returns a fresh wrapper each render.
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [setCmdResult, setQuery, setResults],
  );

  const handleSecondaryAction = useCallback(
    async (id: SecondaryActionId, result: SearchResult) => {
      switch (id) {
        case "reveal":
          await revealResult(result);
          closeSecondaryMenu();
          break;
        case "copy_path":
          await copyResultLocation(result);
          if (!isCopyableLocationResult(result) && result.path) {
            // Notes / non-fs items still expose `path`; copy raw.
            await copyText(result.path, `Copied: ${result.path}`);
          }
          closeSecondaryMenu();
          break;
        case "copy_name": {
          const name = result.name || basenameFromPath(result.path);
          await copyText(name, `Copied name: ${name}`);
          closeSecondaryMenu();
          break;
        }
        case "show_metadata":
          setExpandedMetadata(true);
          closeSecondaryMenu();
          break;
        case "open_with":
          try {
            await dispatch(IPC.FILE_OPEN_WITH, { path: result.path });
            showHint(`Opened: ${basenameFromPath(result.path)}`);
          } catch (err) {
            showHint(`Open failed: ${(err as Error).message}`, 2500);
          }
          closeSecondaryMenu();
          break;
        case "open_as_text":
          try {
            await dispatch(IPC.FILE_OPEN_AS_TEXT, { path: result.path });
            showHint("Opened as text");
          } catch (err) {
            showHint(`Open failed: ${(err as Error).message}`, 2500);
          }
          closeSecondaryMenu();
          break;
        case "hash":
          try {
            showHint("Hashing…", 60_000);
            const res = await dispatch<{ hex: string; bytes: number }>(IPC.FILE_HASH, {
              path: result.path,
            });
            await navigator.clipboard.writeText(res.hex);
            showHint(`SHA-256 copied: ${res.hex.slice(0, 12)}… (${res.bytes} bytes)`, 2500);
          } catch (err) {
            showHint(`Hash failed: ${(err as Error).message}`, 2500);
          }
          closeSecondaryMenu();
          break;
        case "rename": {
          if (!inlineInput || inlineInput.for !== "rename") {
            setInlineInput({ for: "rename", value: basenameFromPath(result.path) });
            setPendingConfirm(null);
            return;
          }
          const confirm = pendingConfirm === "rename";
          try {
            const res = await dispatch<{ preview?: boolean; target?: string }>(IPC.FILE_RENAME, {
              path: result.path,
              new_name: inlineInput.value,
              confirm,
            });
            if (!confirm) {
              showHint(
                `⚠ Press Enter again to rename → ${res.target ?? inlineInput.value} · Esc cancel`,
                4000,
              );
              setPendingConfirm("rename");
            } else {
              showHint(`Renamed to ${inlineInput.value}`);
              closeSecondaryMenu();
              refreshAfterFileMutation(result.path);
            }
          } catch (err) {
            showHint(`Rename failed: ${(err as Error).message}`, 3000);
            setPendingConfirm(null);
          }
          break;
        }
        case "move": {
          if (!inlineInput || inlineInput.for !== "move") {
            setInlineInput({ for: "move", value: parentDirFromPath(result.path) });
            setPendingConfirm(null);
            return;
          }
          const confirm = pendingConfirm === "move";
          try {
            const res = await dispatch<{ preview?: boolean; target?: string }>(IPC.FILE_MOVE, {
              path: result.path,
              target_dir: inlineInput.value,
              confirm,
            });
            if (!confirm) {
              showHint(
                `⚠ Press Enter again to move → ${res.target ?? inlineInput.value} · Esc cancel`,
                4000,
              );
              setPendingConfirm("move");
            } else {
              showHint(`Moved to ${inlineInput.value}`);
              closeSecondaryMenu();
              refreshAfterFileMutation(result.path);
            }
          } catch (err) {
            showHint(`Move failed: ${(err as Error).message}`, 3000);
            setPendingConfirm(null);
          }
          break;
        }
        case "delete": {
          const confirm = pendingConfirm === "delete";
          try {
            const res = await dispatch<{
              preview?: boolean;
              size?: number | null;
              kind?: string;
              destination?: string;
            }>(IPC.FILE_DELETE, { path: result.path, confirm });
            if (!confirm) {
              const sizeStr = res.size != null ? `${res.size} bytes` : "unknown size";
              const name = result.name || basenameFromPath(result.path);
              showHint(
                `⚠ Press Enter again to delete ${name} · ${sizeStr} → ${res.destination ?? "recycle bin"} · Esc cancel`,
                4000,
              );
              setPendingConfirm("delete");
            } else {
              // 2026-05-19 Bug B1 — Windows Explorer desktop view does not always
              // re-enumerate on SHCNE_DELETE (OneDrive redirect, Defender scan,
              // multi-instance Explorer). File IS in Recycle Bin; the icon may
              // linger until F5. Hint surfaces that so user is not confused.
              showHint("Moved to recycle bin · Press F5 on desktop if icon lingers", 2500);
              closeSecondaryMenu();
              refreshAfterFileMutation(result.path);
            }
          } catch (err) {
            // 5000ms: backend now returns a longer diagnostic message when trash
            // returns Ok but the file remains on disk (in-use, Recycle Bin
            // disabled, network drive…). Give the user time to read.
            showHint(`Delete failed: ${(err as Error).message}`, 5000);
            setPendingConfirm(null);
          }
          break;
        }
      }
    },
    // dispatch intentionally omitted — fresh wrapper per render.
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [
      revealResult,
      closeSecondaryMenu,
      copyResultLocation,
      copyText,
      setExpandedMetadata,
      showHint,
      inlineInput,
      setInlineInput,
      pendingConfirm,
      setPendingConfirm,
      refreshAfterFileMutation,
    ],
  );

  const runFirstSecondary = useCallback(
    async (result: SearchResult) => {
      if (!result.primary_action || !result.secondary_action_count) return;
      const actions = await dispatch<SecondaryActionInfo[]>("action.list_secondary", {
        action_ref: result.primary_action,
      });
      const first = actions[0];
      if (first) {
        await dispatch(IPC.ACTION_RUN, { action_ref: first.action_ref });
      }
    },
    // dispatch intentionally omitted — fresh wrapper per render.
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [],
  );

  return { launchResult, copyResultLocation, handleSecondaryAction, runFirstSecondary };
}
