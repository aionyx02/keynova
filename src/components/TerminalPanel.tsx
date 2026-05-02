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

interface Props {
  onExit: () => void;
}

export function TerminalPanel({ onExit }: Props) {
  const containerRef = useRef<HTMLDivElement>(null);
  const termOpts = useTerminalTheme();

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
    });
    const fitAddon = new FitAddon();
    xterm.loadAddon(fitAddon);
    xterm.open(el);
    fitAddon.fit();

    let cancelled = false;
    let sessionId = "";
    let unlistenOutput: (() => void) | undefined;

    const init = async () => {
      const id = await invoke<string>("cmd_dispatch", {
        route: "terminal.open",
        payload: null,
      });
      if (cancelled) {
        void invoke("cmd_dispatch", { route: "terminal.close", payload: { id } });
        return;
      }
      sessionId = id;
      unlistenOutput = await listen<OutputPayload>("terminal.output", (e) => {
        if (e.payload.id === id) xterm.write(e.payload.output);
      });
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
        void invoke("cmd_dispatch", {
          route: "terminal.resize",
          payload: { id: sessionId, rows: xterm.rows, cols: xterm.cols },
        });
      }
    });
    ro.observe(el);

    return () => {
      cancelled = true;
      ro.disconnect();
      unlistenOutput?.();
      if (sessionId) {
        void invoke("cmd_dispatch", { route: "terminal.close", payload: { id: sessionId } });
      }
      xterm.dispose();
    };
    // intentionally run once; termOpts are stable values
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        e.preventDefault();
        onExit();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [onExit]);

  return (
    <div className="w-full bg-gray-900 rounded-xl overflow-hidden shadow-2xl">
      <div className="flex items-center px-3 py-1.5 bg-gray-800/60 border-b border-gray-700/40">
        <span className="text-[11px] font-mono text-emerald-400 select-none">
          ● Terminal
        </span>
        <span className="ml-auto text-[11px] text-gray-600 select-none">Esc 退出</span>
      </div>
      <div ref={containerRef} className="w-full px-1 py-1" style={{ height: "318px" }} />
    </div>
  );
}
