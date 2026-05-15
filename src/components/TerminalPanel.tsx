import { useEffect, useRef, type KeyboardEvent as ReactKeyboardEvent } from "react";
import { Terminal } from "@xterm/xterm";
import { FitAddon } from "@xterm/addon-fit";
import "@xterm/xterm/css/xterm.css";
import { listen } from "@tauri-apps/api/event";
import { useIPC } from "../hooks/useIPC";
import { useFeature } from "../context/FeatureContext";
import { useTerminalTheme } from "../hooks/useTerminalTheme";
import { IPC } from "../ipc/routes";
import type { SettingEntry, TerminalOpenResponse } from "../ipc/types";
import type { TerminalLaunchSpec } from "../types/terminal";

interface OutputPayload {
  id: string;
  output: string;
}

interface ConfigReloadedPayload {
  changed_keys: string[];
}

interface Props {
  isActive: boolean;
  onExit: () => void | Promise<void>;
  launchSpec?: TerminalLaunchSpec | null;
}

export function TerminalPanel({ isActive, onExit, launchSpec = null }: Props) {
  const { dispatch } = useIPC();
  const { activate } = useFeature();
  const containerRef = useRef<HTMLDivElement>(null);
  const xtermRef = useRef<Terminal | null>(null);
  const fitAddonRef = useRef<FitAddon | null>(null);
  const termOpts = useTerminalTheme();
  const onExitRef = useRef(onExit);
  const launchKey = launchSpec?.launch_id ?? "shell";
  const isEditorSession = Boolean(launchSpec?.editor);

  // Notify the feature gate on first mount so the backend can prewarm.
  useEffect(() => { activate("terminal"); }, [activate]);

  useEffect(() => {
    onExitRef.current = onExit;
  }, [onExit]);

  // Re-focus and re-fit whenever the panel becomes visible again
  useEffect(() => {
    if (!isActive) return;
    requestAnimationFrame(() => {
      fitAddonRef.current?.fit();
      xtermRef.current?.focus();
    });
  }, [isActive]);

  useEffect(() => {
    const el = containerRef.current;
    if (!el) return;

    let cancelled = false;
    let sessionId = "";
    let unlistenOutput: (() => void) | undefined;
    let unlistenConfig: (() => void) | undefined;
    const pendingOutput = new Map<string, string>();

    const handleOutput = (id: string, output: string) => {
      const xterm = xtermRef.current;
      if (!xterm) return;
      if (id === sessionId) { xterm.write(output); return; }
      pendingOutput.set(id, `${pendingOutput.get(id) ?? ""}${output}`);
    };

    const fitAndResizeBackend = (id: string) => {
      const xterm = xtermRef.current;
      const fit = fitAddonRef.current;
      if (!xterm || !fit) return;
      fit.fit();
      void dispatch(IPC.TERMINAL_RESIZE, { id, rows: xterm.rows, cols: xterm.cols });
    };

    const loadTerminalSettings = async () => {
      let fontSize = termOpts.fontSize;
      let scrollback = termOpts.scrollback;
      if (window.__TAURI_INTERNALS__) {
        try {
          const entries = await dispatch<SettingEntry[]>(IPC.SETTING_LIST_ALL);
          const get = (k: string) => entries.find((e) => e.key === k)?.value;
          fontSize = parseInt(get("terminal.font_size") ?? "") || fontSize;
          scrollback = parseInt(get("terminal.scrollback_lines") ?? "") || scrollback;
        } catch {
          // use theme defaults
        }
      }
      return { fontSize, scrollback };
    };

    const init = async () => {
      // Read terminal config before creating xterm so settings are applied immediately
      const { fontSize, scrollback } = await loadTerminalSettings();
      if (cancelled) return;

      const xterm = new Terminal({
        theme: termOpts.theme,
        fontFamily: termOpts.fontFamily,
        fontSize,
        cursorStyle: termOpts.cursorStyle,
        cursorBlink: termOpts.cursorBlink,
        scrollback,
        allowTransparency: false,
        windowsPty: { backend: "conpty" },
      });
      const fitAddon = new FitAddon();
      xterm.loadAddon(fitAddon);
      xterm.open(el);
      fitAddon.fit();

      if (cancelled) {
        xterm.dispose();
        return;
      }

      xtermRef.current = xterm;
      fitAddonRef.current = fitAddon;
      xterm.focus();

      xterm.attachCustomKeyEventHandler((e: KeyboardEvent) => {
        if (
          e.type === "keydown" &&
          ((e.ctrlKey && e.shiftKey && e.key.toLowerCase() === "q") ||
            (e.ctrlKey && e.altKey && (e.key === "Escape" || e.code === "Escape")))
        ) {
          void Promise.resolve(onExitRef.current());
          return false;
        }
        // In WebView2/xterm, a physical Escape can surface as keyup only.
        if (
          !isEditorSession &&
          (e.type === "keydown" || e.type === "keyup") &&
          (e.key === "Escape" || e.code === "Escape")
        ) {
          void Promise.resolve(onExitRef.current());
          return false;
        }
        return true;
      });

      xterm.onData((data) => {
        if (!sessionId) return;
        // On Windows/WebView2, a physical Escape can reach xterm as the
        // focus-out escape sequence instead of a DOM keydown.
        if (!isEditorSession && (data === "\x1b" || data === "\x1b[O")) {
          void Promise.resolve(onExitRef.current());
          return;
        }
        if (data === "\x1b[I") return;
        void dispatch(IPC.TERMINAL_SEND, { id: sessionId, input: data });
      });

      try {
        unlistenConfig = await listen<ConfigReloadedPayload>("config-reloaded", (event) => {
          if (
            event.payload.changed_keys.length !== 0 &&
            !event.payload.changed_keys.some((key) => key.startsWith("terminal."))
          ) {
            return;
          }
          void loadTerminalSettings().then((settings) => {
            const xterm = xtermRef.current;
            if (!xterm) return;
            xterm.options.fontSize = settings.fontSize;
            xterm.options.scrollback = settings.scrollback;
            if (sessionId) fitAndResizeBackend(sessionId);
          });
        });
        unlistenOutput = await listen<OutputPayload>("terminal-output", (e) => {
          handleOutput(e.payload.id, e.payload.output);
        });
        if (cancelled) { unlistenOutput(); unlistenOutput = undefined; return; }

        const resp = await dispatch<TerminalOpenResponse>(IPC.TERMINAL_OPEN, {
          rows: xterm.rows, cols: xterm.cols, launch_spec: launchSpec,
        });
        if (cancelled) {
          void dispatch(IPC.TERMINAL_CLOSE, { id: resp.id });
          return;
        }

        const buffered = `${resp.initial_output}${pendingOutput.get(resp.id) ?? ""}`;
        pendingOutput.delete(resp.id);
        pendingOutput.clear();
        sessionId = resp.id;
        fitAndResizeBackend(resp.id);
        if (buffered) xterm.write(buffered);
        xterm.focus();
      } catch (err) {
        if (!cancelled) {
          const message = err instanceof Error ? err.message : String(err);
          xterm.write(`\r\n[terminal] failed to open: ${message}\r\n`);
        }
      }
    };

    void init();

    const ro = new ResizeObserver(() => {
      fitAddonRef.current?.fit();
      if (sessionId) fitAndResizeBackend(sessionId);
    });
    ro.observe(el);

    return () => {
      cancelled = true;
      ro.disconnect();
      unlistenOutput?.();
      unlistenConfig?.();
      if (sessionId) {
        void dispatch(IPC.TERMINAL_CLOSE, { id: sessionId });
      }
      const xterm = xtermRef.current;
      xtermRef.current = null;
      fitAddonRef.current = null;
      xterm?.dispose();
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [launchKey]);

  const handleKeyDownCapture = (event: ReactKeyboardEvent<HTMLDivElement>) => {
    const isExitChord =
      (event.ctrlKey && event.shiftKey && event.key.toLowerCase() === "q") ||
      (event.ctrlKey &&
        event.altKey &&
        (event.key === "Escape" || event.key === "Esc" || event.code === "Escape"));
    if (isEditorSession) {
      if (!isExitChord) return;
    } else if (event.key !== "Escape" && event.key !== "Esc" && event.code !== "Escape") {
      return;
    }
    event.preventDefault();
    event.stopPropagation();
    void Promise.resolve(onExitRef.current());
  };

  return (
    <div
      className="w-full bg-gray-900 rounded-xl overflow-hidden shadow-2xl"
      onKeyDownCapture={handleKeyDownCapture}
    >
      <div className="flex items-center px-3 py-1.5 bg-gray-800/60 border-b border-gray-700/40">
        <span className="text-[11px] font-mono text-emerald-400 select-none">
          {launchSpec?.title ?? "Terminal"}
        </span>
        <span className="ml-auto text-[11px] text-gray-600 select-none">
          {isEditorSession ? "Ctrl+Shift+Q to exit" : "Esc to exit"}
        </span>
      </div>
      <div
        ref={containerRef}
        className="w-full px-1 py-1"
        style={{ height: "318px" }}
        onClick={() => xtermRef.current?.focus()}
      />
    </div>
  );
}
