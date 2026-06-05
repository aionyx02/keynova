import { useCallback, useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useI18n } from "../../i18n/useI18n";
import type { PanelProps } from "../../types/panel";

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
    void ipcDispatch<CalcEntry[]>("calculator.history")
      .then(setHistory)
      .catch(() => {});
  }, []);

  const evaluate = useCallback(async (value: string) => {
    if (!value.trim()) {
      setResult("");
      setError("");
      return;
    }
    try {
      const res = await ipcDispatch<{ result: string }>("calculator.eval", { expr: value });
      setResult(res.result);
      setError("");
      setHistory((prev) => [{ expr: value, result: res.result }, ...prev.slice(0, 49)]);
    } catch (err) {
      setResult("");
      setError(String(err));
    }
  }, []);

  function handleChange(value: string) {
    setExpr(value);
    if (debounceRef.current) clearTimeout(debounceRef.current);
    debounceRef.current = setTimeout(() => {
      void evaluate(value);
    }, 250);
  }

  async function copyResult() {
    if (!result) return;
    await navigator.clipboard.writeText(result);
    setCopied(true);
    setTimeout(() => setCopied(false), 1200);
  }

  return (
    <div className="kn-panel-shell flex min-h-[360px] flex-col rounded-t-none border-t-0">
      <div className="kn-panel-header">
        <div>
          <div className="kn-panel-title">{t.calculator.title}</div>
          <div className="kn-panel-subtitle">{t.calculator.subtitle}</div>
        </div>
        <button
          type="button"
          onClick={() => void copyResult()}
          disabled={!result}
          className="kn-button py-1 text-[10px] disabled:opacity-40"
        >
          {copied ? t.calculator.copied : t.calculator.copy}
        </button>
      </div>

      <div className="flex flex-1 flex-col gap-3 px-4 py-3">
        <input
          ref={inputRef}
          value={expr}
          onChange={(e) => handleChange(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Escape") {
              e.preventDefault();
              onClose();
              return;
            }
            if (e.key === "Enter") {
              e.preventDefault();
              if (debounceRef.current) clearTimeout(debounceRef.current);
              void evaluate(expr);
            }
          }}
          placeholder={t.calculator.placeholder}
          className="kn-field w-full font-mono text-sm"
        />

        {(result || error) && (
          <div
            className={`kn-muted-surface flex min-h-[72px] items-center justify-between gap-3 px-4 py-3 ${
              error ? "border-red-400/20 bg-[color:var(--kn-danger-wash)]" : ""
            }`}
          >
            {error ? (
              <span className="text-sm text-red-200">{error}</span>
            ) : (
              <>
                <span className="font-mono text-2xl font-semibold text-[color:var(--kn-success)]">
                  {result}
                </span>
                <span className="text-xs text-[color:var(--kn-text-faint)]">
                  {t.calculator.readyToCopy}
                </span>
              </>
            )}
          </div>
        )}

        <div className="flex min-h-0 flex-1 flex-col">
          <div className="kn-section-label mb-2">{t.calculator.history}</div>
          <div className="kn-scroll flex-1 overflow-y-auto">
            {history.length === 0 ? (
              <div className="kn-muted-surface flex h-full min-h-[140px] items-center justify-center px-4 text-center text-xs text-[color:var(--kn-text-faint)]">
                {t.calculator.emptyHistory}
              </div>
            ) : (
              <div className="space-y-1">
                {history.map((entry, index) => (
                  <button
                    key={`${entry.expr}-${entry.result}-${index}`}
                    type="button"
                    onClick={() => {
                      setExpr(entry.expr);
                      void evaluate(entry.expr);
                    }}
                    className="kn-result-row flex w-full items-center justify-between gap-3 px-3 py-2 text-left"
                    data-selected="false"
                  >
                    <span className="min-w-0 truncate font-mono text-sm text-[color:var(--kn-text-soft)]">
                      {entry.expr}
                    </span>
                    <span className="shrink-0 font-mono text-xs text-[color:var(--kn-text-muted)]">
                      = {entry.result}
                    </span>
                  </button>
                ))}
              </div>
            )}
          </div>
        </div>
      </div>

      <div className="kn-panel-footer">
        <span>{t.calculator.enterToEvaluate}</span>
        <span>{t.calculator.escCloses}</span>
      </div>
    </div>
  );
}
