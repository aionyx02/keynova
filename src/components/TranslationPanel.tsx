import { useCallback, useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import { useI18n } from "../i18n/useI18n";
import type { PanelProps } from "../types/panel";

interface TranslationResponsePayload {
  request_id: string;
  ok: boolean;
  text?: string;
  error?: string;
}

interface ParsedTrArgs {
  src: string;
  dst: string;
  text: string;
}

async function ipcDispatch<T>(route: string, payload?: Record<string, unknown>): Promise<T> {
  return invoke<T>("cmd_dispatch", { route, payload: payload ?? null });
}

function parseTranslationArgs(raw: string): ParsedTrArgs | null {
  const input = raw.trim();
  if (!input) return null;

  const parts = input.split(/\s+/);
  if (parts.length >= 3) {
    const [src, dst, ...rest] = parts;
    return {
      src: src || "auto",
      dst: dst || "zh-TW",
      text: rest.join(" ").trim(),
    };
  }

  return {
    src: "auto",
    dst: "zh-TW",
    text: input,
  };
}

export function TranslationPanel({ onClose, initialArgs }: PanelProps) {
  const t = useI18n();
  const parsedInitial = parseTranslationArgs(initialArgs ?? "");
  const [src, setSrc] = useState(parsedInitial?.src ?? "auto");
  const [dst, setDst] = useState(parsedInitial?.dst ?? "zh-TW");
  const [text, setText] = useState(parsedInitial?.text ?? "");
  const [result, setResult] = useState("");
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState("");
  const [copied, setCopied] = useState(false);
  const pendingIdRef = useRef<string | null>(null);
  const lastSentKeyRef = useRef("");
  const initialAutoTriggeredRef = useRef(false);

  useEffect(() => {
    if (!window.__TAURI_INTERNALS__) return;
    const unlisten = listen<TranslationResponsePayload>("translation-result", (event) => {
      const { request_id, ok, text: translated, error: err } = event.payload;
      if (request_id !== pendingIdRef.current) return;

      pendingIdRef.current = null;
      setLoading(false);
      if (ok && translated) {
        setResult(translated);
        setError("");
      } else {
        setError(err ?? t.translation.error);
      }
    });
    return () => {
      unlisten.then((fn) => fn());
    };
  }, [t.translation.error]);

  const translate = useCallback(
    async (nextSrc: string, nextDst: string, nextText: string, force = false) => {
      const normalizedText = nextText.trim();
      const normalizedDst = nextDst.trim();
      if (!normalizedText || !normalizedDst) {
        if (!normalizedText) {
          setLoading(false);
          setResult("");
          setError("");
        }
        return;
      }

      const requestKey = `${nextSrc}\u0000${normalizedDst}\u0000${normalizedText}`;
      if (!force && requestKey === lastSentKeyRef.current) return;
      lastSentKeyRef.current = requestKey;

      const requestId = `tr-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`;
      pendingIdRef.current = requestId;
      setLoading(true);
      setError("");
      try {
        await ipcDispatch("translation.translate", {
          request_id: requestId,
          src: nextSrc,
          dst: normalizedDst,
          text: normalizedText,
        });
      } catch (err) {
        if (pendingIdRef.current === requestId) {
          pendingIdRef.current = null;
          setLoading(false);
          setError(String(err));
        }
      }
    },
    [],
  );

  useEffect(() => {
    if (initialAutoTriggeredRef.current) return;
    initialAutoTriggeredRef.current = true;
    if (!text.trim()) return;
    const timer = window.setTimeout(() => {
      void translate(src, dst, text, false);
    }, 0);
    return () => window.clearTimeout(timer);
  }, [src, dst, text, translate]);

  async function copyResult() {
    if (!result) return;
    await navigator.clipboard.writeText(result);
    setCopied(true);
    setTimeout(() => setCopied(false), 1500);
  }

  return (
    <div className="min-h-[350px] bg-gray-900/95 backdrop-blur-md rounded-b-xl shadow-2xl flex flex-col p-4 gap-3">
      <div className="flex items-center gap-2">
        <span className="text-xs font-semibold text-blue-400 uppercase tracking-wide">
          {t.translation.title}
        </span>
      </div>

      <div className="flex gap-2">
        <input
          value={src}
          onChange={(e) => {
            const nextSrc = e.target.value;
            setSrc(nextSrc);
            void translate(nextSrc, dst, text, false);
          }}
          placeholder={t.translation.srcPlaceholder}
          className="flex-1 bg-gray-800/60 text-gray-200 text-xs rounded px-2 py-1 outline-none placeholder-gray-600"
        />
        <span className="text-gray-600 self-center">→</span>
        <input
          value={dst}
          onChange={(e) => {
            const nextDst = e.target.value;
            setDst(nextDst);
            void translate(src, nextDst, text, false);
          }}
          placeholder={t.translation.dstPlaceholder}
          className="flex-1 bg-gray-800/60 text-gray-200 text-xs rounded px-2 py-1 outline-none placeholder-gray-600"
        />
      </div>

      <textarea
        value={text}
        onChange={(e) => {
          const nextText = e.target.value;
          setText(nextText);
          void translate(src, dst, nextText, false);
        }}
        onKeyDown={(e) => {
          if (e.key === "Escape") {
            e.preventDefault();
            onClose();
            return;
          }
          if (e.key === "Enter" && e.ctrlKey) {
            e.preventDefault();
            void translate(src, dst, text, true);
          }
        }}
        placeholder={t.translation.textPlaceholder}
        rows={3}
        className="bg-gray-800/60 text-gray-200 text-sm rounded px-3 py-2 outline-none resize-none placeholder-gray-600"
      />

      <button
        onClick={() => void translate(src, dst, text, true)}
        disabled={!text.trim()}
        className="self-end px-4 py-1.5 bg-blue-600/70 text-white text-xs rounded hover:bg-blue-500/70 disabled:opacity-40 transition-colors"
      >
        {loading ? t.translation.translating : t.translation.translate}
      </button>

      {(result || error) && (
        <div className="relative rounded bg-gray-800/60 px-3 py-2">
          {error ? (
            <p className="text-xs text-red-400">{error}</p>
          ) : (
            <>
              <pre className="text-sm text-gray-200 whitespace-pre-wrap">{result}</pre>
              <button
                onClick={() => void copyResult()}
                className="absolute top-2 right-2 text-[10px] text-gray-500 hover:text-gray-300"
              >
                {copied ? t.translation.copied : t.translation.copy}
              </button>
            </>
          )}
        </div>
      )}

      <div className="text-[10px] text-gray-700">Auto translate on input change. Ctrl+Enter to retry. Esc to close.</div>
    </div>
  );
}
