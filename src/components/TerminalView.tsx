import { useEffect, useRef } from "react";
import { Terminal } from "@xterm/xterm";
import { FitAddon } from "@xterm/addon-fit";
import "@xterm/xterm/css/xterm.css";

interface TerminalViewProps {
  terminalId: string;
  onData: (data: string) => void;
  outputQueue: string[];
}

export function TerminalView({ terminalId, onData, outputQueue }: TerminalViewProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const termRef = useRef<Terminal | null>(null);
  const fitRef = useRef<FitAddon | null>(null);

  useEffect(() => {
    if (!containerRef.current) return;

    const term = new Terminal({
      cursorBlink: true,
      fontSize: 14,
      theme: { background: "#111827" },
    });
    const fitAddon = new FitAddon();
    term.loadAddon(fitAddon);
    term.open(containerRef.current);
    fitAddon.fit();
    termRef.current = term;
    fitRef.current = fitAddon;

    term.onData((data) => onData(data));

    const observer = new ResizeObserver(() => fitAddon.fit());
    observer.observe(containerRef.current);

    return () => {
      observer.disconnect();
      term.dispose();
    };
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [terminalId]);

  useEffect(() => {
    if (outputQueue.length > 0 && termRef.current) {
      for (const chunk of outputQueue) {
        termRef.current.write(chunk);
      }
    }
  }, [outputQueue]);

  return (
    <div
      ref={containerRef}
      className="w-full h-full bg-gray-900"
      style={{ minHeight: "300px" }}
    />
  );
}
