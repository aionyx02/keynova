import { useCallback, useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import { useI18n } from "../i18n/useI18n";

interface TranslationResponsePayload {
  request_id: string;
  ok: boolean;
  text?: string;
  error?: string;
  source_lang?: string;
  target_lang?: string;
}

async function ipcDispatch<T>(route: string, payload?: Record<string, unknown>): Promise<T> {
  return invoke<T>("cmd_dispatch", { route, payload: payload ?? null });
}

export function TranslationPanel() {
  const t = useI18n();
  const [src, setSrc] = useState("auto");
  const [dst, setDst] = useState("zh-TW");
  const [text, setText] = useState("");
  const [result, setResult] = useState("");
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState("");
  const [copied, setCopied] = useState(false);
  const pendingIdRef = useRef<string | null>(null);

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
    return () => { unlisten.then((fn) => fn()); };
  }, [t.translation.error]);

  const translate = useCallback(async () => {
    if (!text.trim() || loading) return;
    const requestId = `tr-${Date.now()}`;
    pendingIdRef.current = requestId;
    setLoading(true);
    setResult("");
    setError("");
    try {
      await ipcDispatch("translation.translate", {
        request_id: requestId,
        src,
        dst,
        text,
      });
    } catch (err) {
      pendingIdRef.current = null;
      setLoading(false);
      setError(String(err));
    }
  }, [src, dst, text, loading]);

  async function copyResult() {
    if (!result) return;
    await navigator.clipboard.writeText(result);
    setCopied(true);
    setTimeout(() => setCopied(false), 1500);
  }

  return (
    <div className="bg-gray-900/95 backdrop-blur-md rounded-b-xl shadow-2xl flex flex-col gap-2 p-4">
      <div className="flex items-center gap-2">
        <span className="text-xs font-semibold text-blue-400 uppercase tracking-wide">{t.translation.title}</span>
      </div>

      {/* Language row */}
      <div className="flex gap-2">
        <input
          value={src}
          onChange={(e) => setSrc(e.target.value)}
          placeholder={t.translation.srcPlaceholder}
          className="flex-1 bg-gray-800/60 text-gray-200 text-xs rounded px-2 py-1 outline-none placeholder-gray-600"
        />
        <span className="text-gray-600 self-center">→</span>
        <input
          value={dst}
          onChange={(e) => setDst(e.target.value)}
          placeholder={t.translation.dstPlaceholder}
          className="flex-1 bg-gray-800/60 text-gray-200 text-xs rounded px-2 py-1 outline-none placeholder-gray-600"
        />
      </div>

      {/* Source text */}
      <textarea
        value={text}
        onChange={(e) => setText(e.target.value)}
        onKeyDown={(e) => { if (e.key === "Enter" && e.ctrlKey) { e.preventDefault(); void translate(); } }}
        placeholder={t.translation.textPlaceholder}
        rows={3}
        className="bg-gray-800/60 text-gray-200 text-sm rounded px-3 py-2 outline-none resize-none placeholder-gray-600"
      />

      {/* Translate button */}
      <button
        onClick={() => void translate()}
        disabled={loading || !text.trim()}
        className="self-end px-4 py-1.5 bg-blue-600/70 text-white text-xs rounded hover:bg-blue-500/70 disabled:opacity-40 transition-colors"
      >
        {loading ? t.translation.translating : t.translation.translate}
      </button>

      {/* Result */}
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

      <div className="text-[10px] text-gray-700">Ctrl+Enter 翻譯 · Esc 關閉</div>
    </div>
  );
}