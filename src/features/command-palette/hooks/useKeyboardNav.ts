// REF.2.P5 — Keyboard navigation handler for the palette input.
//
// Owns the full `onKeyDown` switch routed from the <input>: the Bug A
// focus-guard throttle (200 ms), the cheatsheet `?` open, the search-mode
// branches (secondary menu arrows / Tab / Enter / ArrowLeft, copy shortcut,
// open-menu via ArrowRight/Tab at end-of-input, ArrowDown/Up selection,
// Enter -> pipeline or launch / shift+Enter -> first secondary), and the
// command-mode branches (ArrowDown/Up over cmd or args suggestions, Tab fill,
// Enter execute).
//
// Bug A — 2026-05-19 round 2 — root cause was that English typing also
// triggers `WindowEvent::Focused(false)` blips. The 200 ms throttled
// `keepLauncherOpen` renewal here is the only thing keeping the backend
// launcher_focus_guard alive across keystrokes. Preserve verbatim.

import { useRef } from "react";

import type { SearchResult } from "../../../types/search";
import type { CommandMeta } from "../../../hooks/useCommands";
import type { SecondaryActionId } from "../../../utils/secondaryActions";
import { buildSecondaryActions } from "../../../utils/secondaryActions";

function isCopyShortcut(e: React.KeyboardEvent) {
  return (e.ctrlKey || e.metaKey) && !e.altKey && e.key.toLowerCase() === "c";
}

function isCopyableLocationResult(result: SearchResult | null) {
  return result?.kind === "app" || result?.kind === "file" || result?.kind === "folder";
}

export interface UseKeyboardNavDeps {
  /** Current parsed mode (search / command / terminal). */
  mode: "search" | "command" | "terminal";
  query: string;
  cmdResult: unknown;
  visibleResults: SearchResult[];
  safeSelected: number;
  setSelected: React.Dispatch<React.SetStateAction<number>>;
  // Secondary menu state.
  secondaryMenuOpen: boolean;
  setSecondaryMenuOpen: React.Dispatch<React.SetStateAction<boolean>>;
  menuFocusedIndex: number;
  setMenuFocusedIndex: React.Dispatch<React.SetStateAction<number>>;
  closeSecondaryMenu: () => void;
  // Cheatsheet.
  setCheatsheetOpen: React.Dispatch<React.SetStateAction<boolean>>;
  // Command mode.
  cmdName: string;
  cmdArgs: string;
  cmdSuggestions: CommandMeta[];
  selectedCmd: number;
  setSelectedCmd: React.Dispatch<React.SetStateAction<number>>;
  isArgsPhase: boolean;
  argSuggestions: string[];
  selectedArg: number;
  setSelectedArg: React.Dispatch<React.SetStateAction<number>>;
  setQuery: (q: string) => void;
  // Action callbacks.
  copyResultLocation: (r: SearchResult) => Promise<void>;
  handleSecondaryAction: (id: SecondaryActionId, r: SearchResult) => Promise<void>;
  launchResult: (r: SearchResult) => Promise<void>;
  runFirstSecondary: (r: SearchResult) => Promise<void>;
  runPipeline: (text: string) => Promise<void>;
  execCommand: (name: string, args?: string) => Promise<void>;
  /** REF.6.B — true when palette is in capability mode (prefix matched). */
  capabilityMode: boolean;
  /** REF.6.B — invoke the active capability stream's submit (Enter-to-ask). */
  onCapabilitySubmit: () => void;
  // Bug A focus-guard renewal (must be invoked on every keydown, throttled).
  keepLauncherOpen: () => Promise<void> | void;
}

export function useKeyboardNav(deps: UseKeyboardNavDeps) {
  const lastGuardRef = useRef<number>(0);

  const onKeyDown = (e: React.KeyboardEvent<HTMLInputElement>) => {
      // Bug-fix 2026-05-19 (round 2) — Bug A 真根因：英文打字也會觸發
      // WindowEvent::Focused(false) blip。每次 keydown 都 renew guard (200 ms
      // throttle) → backend grace 期內 guard 必有效 → 不 hide。
      // eslint-disable-next-line react-hooks/purity -- event handler, not render path
      const now = Date.now();
      if (now - lastGuardRef.current > 200) {
        lastGuardRef.current = now;
        void deps.keepLauncherOpen();
      }

      // ONBOARD.1.B — `?` opens cheatsheet when input is empty, so it doesn't
      // collide with typing `?` as part of a query.
      if (
        e.key === "?" &&
        deps.query === "" &&
        !deps.secondaryMenuOpen &&
        !deps.cmdResult
      ) {
        e.preventDefault();
        deps.setCheatsheetOpen(true);
        return;
      }

      // REF.6.B — capability mode owns Enter: fire `submit()` on the active
      // stream. Predates the search-mode branch so the search row launcher
      // does not steal the keystroke. Esc cancel is handled in useEscapeKey.
      // The `isComposing` guard lets a Chinese / Japanese IME commit its
      // candidate on the first Enter without triggering submit — the
      // subsequent Enter (post-commit) reaches us with isComposing === false.
      if (
        deps.capabilityMode &&
        e.key === "Enter" &&
        !e.shiftKey &&
        !e.nativeEvent.isComposing
      ) {
        e.preventDefault();
        deps.onCapabilitySubmit();
        return;
      }

      if (deps.mode === "search") {
        // Menu open: route arrows/Enter/Left to menu actions; let typed text fall through.
        if (deps.secondaryMenuOpen) {
          const r = deps.visibleResults[deps.safeSelected] ?? null;
          const enabled = r
            ? buildSecondaryActions(r).filter((it) => !it.disabled)
            : [];
          if (e.key === "ArrowDown") {
            e.preventDefault();
            deps.setMenuFocusedIndex((i) =>
              Math.min(i + 1, Math.max(enabled.length - 1, 0)),
            );
            return;
          }
          if (e.key === "ArrowUp") {
            e.preventDefault();
            deps.setMenuFocusedIndex((i) => Math.max(i - 1, 0));
            return;
          }
          if (e.key === "Tab") {
            e.preventDefault();
            deps.setMenuFocusedIndex((i) => {
              const next = e.shiftKey ? i - 1 : i + 1;
              const max = Math.max(enabled.length - 1, 0);
              return Math.min(Math.max(next, 0), max);
            });
            return;
          }
          if (e.key === "Enter") {
            e.preventDefault();
            const item = enabled[deps.menuFocusedIndex];
            if (item && r) void deps.handleSecondaryAction(item.id, r);
            return;
          }
          if (e.key === "ArrowLeft") {
            // Only close menu if input caret is at start; otherwise let cursor move.
            const target = e.currentTarget;
            if (target.selectionStart === 0 && target.selectionEnd === 0) {
              e.preventDefault();
              deps.closeSecondaryMenu();
              return;
            }
          }
          // Other keys fall through to input.
        }

        if (isCopyShortcut(e)) {
          const target = e.currentTarget;
          if (target.selectionStart !== target.selectionEnd) return;
          const r = deps.visibleResults[deps.safeSelected] ?? null;
          if (isCopyableLocationResult(r) && r) {
            e.preventDefault();
            void deps.copyResultLocation(r);
            return;
          }
        }

        // Open secondary menu when cursor at end of input and a result is selected.
        if (!deps.secondaryMenuOpen && (e.key === "ArrowRight" || e.key === "Tab")) {
          const target = e.currentTarget;
          const atEnd =
            target.selectionStart === target.value.length &&
            target.selectionEnd === target.value.length;
          const r = deps.visibleResults[deps.safeSelected] ?? null;
          if (atEnd && r) {
            e.preventDefault();
            deps.setSecondaryMenuOpen(true);
            deps.setMenuFocusedIndex(0);
            return;
          }
        }

        if (e.key === "ArrowDown") {
          e.preventDefault();
          deps.setSelected((i) => Math.min(i + 1, deps.visibleResults.length - 1));
        } else if (e.key === "ArrowUp") {
          e.preventDefault();
          deps.setSelected((i) => Math.max(i - 1, 0));
        } else if (e.key === "Enter") {
          e.preventDefault();
          if (deps.query.includes("|")) {
            void deps.runPipeline(deps.query);
            return;
          }
          const r = deps.visibleResults[deps.safeSelected];
          if (r) {
            if (e.shiftKey) void deps.runFirstSecondary(r);
            else void deps.launchResult(r);
          }
        }
      } else if (deps.mode === "command") {
        if (e.key === "ArrowDown") {
          e.preventDefault();
          if (deps.isArgsPhase) {
            deps.setSelectedArg((i) => Math.min(i + 1, deps.argSuggestions.length - 1));
          } else {
            deps.setSelectedCmd((i) =>
              Math.min(i + 1, deps.cmdSuggestions.length - 1),
            );
          }
        } else if (e.key === "ArrowUp") {
          e.preventDefault();
          if (deps.isArgsPhase) {
            deps.setSelectedArg((i) => Math.max(i - 1, 0));
          } else {
            // At index 0 or already unselected (-1): deselect all (visual cursor
            // returns to the input bar). Prevents the 0↔-1 oscillation that
            // occurred when Math.max(-2, 0) kept snapping back to 0.
            deps.setSelectedCmd((i) => (i <= 0 ? -1 : i - 1));
          }
        } else if (e.key === "Tab") {
          e.preventDefault();
          if (deps.isArgsPhase && deps.argSuggestions.length > 0) {
            // Fill in the selected config key and add a trailing space for value input.
            const arg = deps.argSuggestions[deps.selectedArg];
            if (arg) deps.setQuery(`/${deps.cmdName} ${arg} `);
          } else if (!deps.isArgsPhase) {
            const cmd = deps.cmdSuggestions[deps.selectedCmd];
            if (cmd) deps.setQuery("/" + cmd.name);
          }
        } else if (e.key === "Enter") {
          e.preventDefault();
          if (deps.isArgsPhase) {
            void deps.execCommand(deps.cmdName, deps.cmdArgs);
          } else {
            const cmd = deps.cmdSuggestions[deps.selectedCmd];
            if (cmd) void deps.execCommand(cmd.name, deps.cmdArgs);
          }
        }
      }
    };

  return { onKeyDown };
}
