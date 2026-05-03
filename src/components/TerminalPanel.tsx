import { useEffect, useRef } from "react";
import { Terminal } from "@xterm/xterm";
import { FitAddon } from "@xterm/addon-fit";
import "@xterm/xterm/css/xterm.css";
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import { useTerminalTheme } from "../hooks/useTerminalTheme";

interface OutputPayload {
  id: string;
  output: string;
}

interface OpenResponse {
  id: string;
  initial_output: string;
}

interface Props {
  onExit: () => void | Promise<void>;
}

export function TerminalPanel({ onExit }: Props) {
  const containerRef = useRef<HTMLDivElement>(null);
  const xtermRef = useRef<Terminal | null>(null);
  const termOpts = useTerminalTheme();
  // Stable ref so attachCustomKeyEventHandler closure always sees latest onExit
  const onExitRef = useRef(onExit);
  useEffect(() => {
    onExitRef.current = onExit;
  }, [onExit]);

  useEffect(() => {
    const el = containerRef.current;
    if (!el) return;

    const xterm = new Terminal({
      theme: termOpts.theme,
      fontFamily: termOpts.fontFamily,
      fontSize: termOpts.fontSize,
      cursorStyle: termOpts.cursorStyle,
      cursorBlink: termOpts.cursorBlink,
      scrollback: termOpts.scrollback,
      allowTransparency: false,
      windowsPty: { backend: "conpty" },
    });
    const fitAddon = new FitAddon();
    xterm.loadAddon(fitAddon);
    xterm.open(el);
    fitAddon.fit();
    xtermRef.current = xterm;
    xterm.focus();

    let cancelled = false;
    let sessionId = "";
    let unlistenOutput: (() => void) | undefined;
    const pendingOutput = new Map<string, string>();

    // Intercept ESC before it reaches the PTY. Other keys, including Ctrl+C,
    // should keep normal terminal semantics.
    xterm.attachCustomKeyEventHandler((e: KeyboardEvent) => {
      if (e.type !== "keydown") return true;
      if (e.key === "Escape") {
        void Promise.resolve(onExitRef.current());
        return false;
      }
      return true;
    });

    const handleOutput = (id: string, output: string) => {
      if (id === sessionId) {
        xterm.write(output);
        return;
      }
      pendingOutput.set(id, `${pendingOutput.get(id) ?? ""}${output}`);
    };

    const fitAndResizeBackend = (id: string) => {
      fitAddon.fit();
      void invoke("cmd_dispatch", {
        route: "terminal.resize",
        payload: { id, rows: xterm.rows, cols: xterm.cols },
      });
    };

    const init = async () => {
      try {
        // Subscribe before opening the PTY so fast shell output cannot be missed.
        unlistenOutput = await listen<OutputPayload>("terminal-output", (e) => {
          handleOutput(e.payload.id, e.payload.output);
        });
        if (cancelled) {
          unlistenOutput();
          unlistenOutput = undefined;
          return;
        }

        const resp = await invoke<OpenResponse>("cmd_dispatch", {
          route: "terminal.open",
          payload: { rows: xterm.rows, cols: xterm.cols },
        });
        if (cancelled) {
          void invoke("cmd_dispatch", {
            route: "terminal.close",
            payload: { id: resp.id },
          });
          return;
        }

        const buffered = `${resp.initial_output}${pendingOutput.get(resp.id) ?? ""}`;
        pendingOutput.delete(resp.id);
        pendingOutput.clear();
        sessionId = resp.id;
        fitAndResizeBackend(resp.id);
        if (buffered) {
          xterm.write(buffered);
        }
        xterm.focus();
      } catch (err) {
        if (!cancelled) {
          const message = err instanceof Error ? err.message : String(err);
          xterm.write(`\r\n[terminal] failed to open: ${message}\r\n`);
        }
      }
    };

    void init();

    xterm.onData((data) => {
      if (!sessionId) return;
      void invoke("cmd_dispatch", {
        route: "terminal.send",
        payload: { id: sessionId, input: data },
      });
    });

    const ro = new ResizeObserver(() => {
      fitAddon.fit();
      if (sessionId) {
        fitAndResizeBackend(sessionId);
      }
    });
    ro.observe(el);

    return () => {
      cancelled = true;
      ro.disconnect();
      unlistenOutput?.();
      if (sessionId) {
        void invoke("cmd_dispatch", {
          route: "terminal.close",
          payload: { id: sessionId },
        });
      }
      xtermRef.current = null;
      xterm.dispose();
    };
    // intentionally run once; termOpts are stable values
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  return (
    <div className="w-full bg-gray-900 rounded-xl overflow-hidden shadow-2xl">
      <div className="flex items-center px-3 py-1.5 bg-gray-800/60 border-b border-gray-700/40">
        <span className="text-[11px] font-mono text-emerald-400 select-none">Terminal</span>
        <span className="ml-auto text-[11px] text-gray-600 select-none">Esc to exit</span>
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
