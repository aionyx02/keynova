import { useCallback, useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useI18n } from "../i18n/useI18n";
import type { PanelProps } from "./panel/PanelRegistry";

interface CalcEntry {
  expr: string;
  result: string;
}

async function ipcDispatch<T>(route: string, payload?: Record<string, unknown>): Promise<T> {
  return invoke<T>("cmd_dispatch", { route, payload: payload ?? null });
}

export function CalculatorPanel({ onClose }: PanelProps) {
  const t = useI18n();
  const [expr, setExpr] = useState("");
  const [result, setResult] = useState("");
  const [error, setError] = useState("");
  const [history, setHistory] = useState<CalcEntry[]>([]);
  const [copied, setCopied] = useState(false);
  const inputRef = useRef<HTMLInputElement>(null);
  const debounceRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(() => {
    inputRef.current?.focus();
    if (!window.__TAURI_INTERNALS__) return;
    ipcDispatch<CalcEntry[]>("calculator.history")
      .then(setHistory)
      .catch(() => {});
  }, []);

  const evaluate = useCallback(async (e: string) => {
    if (!e.trim()) { setResult(""); setError(""); return; }
    try {
      const res = await ipcDispatch<{ result: string }>("calculator.eval", { expr: e });
      setResult(res.result);
      setError("");
      setHistory((prev) => [{ expr: e, result: res.result }, ...prev.slice(0, 49)]);
    } catch (err) {
      setResult("");
      setError(String(err));
    }
  }, []);

  function handleChange(value: string) {
    setExpr(value);
    if (debounceRef.current) clearTimeout(debounceRef.current);
    debounceRef.current = setTimeout(() => { void evaluate(value); }, 300);
  }

  async function copyResult() {
    if (!result) return;
    await navigator.clipboard.writeText(result);
    setCopied(true);
    setTimeout(() => setCopied(false), 1200);
  }

  return (
    <div className="bg-gray-900/95 backdrop-blur-md rounded-b-xl shadow-2xl flex flex-col p-4 gap-3">
      <div className="flex items-center gap-2">
        <span className="text-xs font-semibold text-blue-400 uppercase tracking-wide">{t.calculator.title}</span>
      </div>

      {/* Input */}
      <div className="relative">
        <input
          ref={inputRef}
          value={expr}
          onChange={(e) => handleChange(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Escape") { e.preventDefault(); onClose(); return; }
            if (e.key === "Enter") { if (debounceRef.current) clearTimeout(debounceRef.current); void evaluate(expr); }
          }}
          placeholder={t.calculator.placeholder}
          className="w-full bg-gray-800/60 text-gray-200 text-sm rounded px-3 py-2 outline-none placeholder-gray-600 font-mono"
        />
      </div>

      {/* Result display */}
      {(result || error) && (
        <div className="flex items-center justify-between bg-gray-800/40 rounded px-3 py-2">
          {error ? (
            <span className="text-red-400 text-sm">{error}</span>
          ) : (
            <>
              <span className="text-2xl font-mono font-semibold text-green-400">{result}</span>
              <button
                onClick={() => void copyResult()}
                className="text-[10px] text-gray-600 hover:text-gray-300 ml-3"
              >
                {copied ? "已複製" : t.calculator.copy}
              </button>
            </>
          )}
        </div>
      )}

      {/* History */}
      {history.length > 0 && (
        <div>
          <div className="text-[10px] text-gray-600 mb-1">{t.calculator.history}</div>
          <div className="max-h-[120px] overflow-y-auto space-y-0.5">
            {history.map((h, i) => (
              <button
                key={i}
                onClick={() => { setExpr(h.expr); void evaluate(h.expr); }}
                className="w-full flex items-center justify-between px-2 py-1 text-xs text-gray-500 hover:bg-white/5 rounded font-mono text-left"
              >
                <span className="truncate">{h.expr}</span>
                <span className="text-gray-400 ml-2 shrink-0">= {h.result}</span>
              </button>
            ))}
          </div>
        </div>
      )}

      <div className="text-[10px] text-gray-700">支援 + - * / ^ () sqrt sin cos pi · 單位換算 · 進位換算</div>
    </div>
  );
}