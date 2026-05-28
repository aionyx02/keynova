import { useEffect, useMemo, useRef, useState, type KeyboardEvent as ReactKeyboardEvent } from "react";
import { Terminal } from "@xterm/xterm";
import { FitAddon } from "@xterm/addon-fit";
import "@xterm/xterm/css/xterm.css";
import { listen } from "@tauri-apps/api/event";
import { UiIcon } from "./icons/UiIcon";
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
  attached?: boolean;
}

type ConnectionState = "connecting" | "running" | "error";

function formatWorkingDirectory(cwd?: string): string {
  if (!cwd) return "Default shell";
  const normalized = cwd.replace(/\\/g, "/");
  const parts = normalized.split("/").filter(Boolean);
  if (parts.length === 0) return cwd;
  const leaf = parts[parts.length - 1];
  return leaf ? `./${leaf}` : cwd;
}

export function TerminalPanel({ isActive, onExit, launchSpec = null, attached = false }: Props) {
  const { dispatch } = useIPC();
  const { activate } = useFeature();
  const containerRef = useRef<HTMLDivElement>(null);
  const xtermRef = useRef<Terminal | null>(null);
  const fitAddonRef = useRef<FitAddon | null>(null);
  const termOpts = useTerminalTheme();
  const onExitRef = useRef(onExit);
  const launchKey = launchSpec?.launch_id ?? "shell";
  const isEditorSession = Boolean(launchSpec?.editor);
  const [connectionState, setConnectionState] = useState<ConnectionState>("connecting");
  const terminalTitle = launchSpec?.title ?? (isEditorSession ? "Editor session" : "Terminal");
  const cwdLabel = useMemo(() => formatWorkingDirectory(launchSpec?.cwd), [launchSpec?.cwd]);
  const statusClass =
    connectionState === "running"
      ? "border-emerald-400/20 bg-emerald-400/10 text-emerald-200"
      : connectionState === "error"
        ? "border-rose-400/20 bg-rose-400/10 text-rose-200"
        : "border-amber-400/20 bg-amber-400/10 text-amber-100";
  const statusLabel =
    connectionState === "running"
      ? "Running"
      : connectionState === "error"
        ? "Needs attention"
        : "Connecting";
  const exitShortcut = isEditorSession ? "Ctrl+Shift+Q" : "Esc";

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
      setConnectionState("connecting");
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
        setConnectionState("running");
        fitAndResizeBackend(resp.id);
        if (buffered) xterm.write(buffered);
        xterm.focus();
      } catch (err) {
        if (!cancelled) {
          setConnectionState("error");
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
      className={`kn-terminal-shell w-full overflow-hidden ${
        attached ? "rounded-t-none border-t-0" : ""
      }`}
      onKeyDownCapture={handleKeyDownCapture}
    >
      <div className="kn-terminal-header">
        <div className="flex min-w-0 items-center gap-3">
          <div className="hidden shrink-0 items-center gap-1.5 sm:flex" aria-hidden="true">
            <span className="h-2.5 w-2.5 rounded-full bg-rose-400/80" />
            <span className="h-2.5 w-2.5 rounded-full bg-amber-300/80" />
            <span className="h-2.5 w-2.5 rounded-full bg-emerald-400/80" />
          </div>

          <span className="flex h-8 w-8 shrink-0 items-center justify-center rounded-[12px] border border-cyan-400/20 bg-cyan-400/10 text-cyan-100 shadow-[inset_0_1px_0_rgba(255,255,255,0.08)]">
            <UiIcon name="terminal" className="h-4 w-4" />
          </span>

          <div className="min-w-0">
            <div className="truncate text-[12px] font-semibold text-[color:var(--kn-text)]">
              {terminalTitle}
            </div>
            <div className="mt-0.5 flex min-w-0 items-center gap-2 text-[10px]">
              <span
                className={`inline-flex shrink-0 items-center gap-1.5 rounded-full border px-2 py-0.5 font-medium ${statusClass}`}
              >
                <span className="h-1.5 w-1.5 rounded-full bg-current" />
                {statusLabel}
              </span>
              <span
                className="min-w-0 truncate font-mono text-[color:var(--kn-text-muted)]"
                title={launchSpec?.cwd ?? cwdLabel}
              >
                {cwdLabel}
              </span>
            </div>
          </div>
        </div>

        <button
          type="button"
          onMouseDown={(event) => event.preventDefault()}
          onClick={() => void Promise.resolve(onExitRef.current())}
          className="flex h-8 w-8 shrink-0 items-center justify-center rounded-[12px] border border-[color:var(--kn-border)] bg-white/[0.035] text-[color:var(--kn-text-muted)] transition-colors hover:border-[color:var(--kn-border-strong)] hover:bg-white/[0.06] hover:text-[color:var(--kn-text)]"
          title={`Close terminal (${exitShortcut})`}
          aria-label="Close terminal"
        >
          <UiIcon name="x" className="h-4 w-4" />
        </button>
      </div>

      <div className="kn-terminal-body" onClick={() => xtermRef.current?.focus()}>
        <div ref={containerRef} className="h-full w-full" />
      </div>
    </div>
  );
}
