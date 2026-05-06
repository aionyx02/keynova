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

type FocusTarget = "command" | "source" | "output";

const DEFAULT_SRC = "auto";
const DEFAULT_DST = "zh-TW";
const LANGUAGE_TOKEN_PATTERN = /^(auto|[a-z]{2,3}(?:-[a-z0-9]+)?)$/i;

async function ipcDispatch<T>(route: string, payload?: Record<string, unknown>): Promise<T> {
  return invoke<T>("cmd_dispatch", { route, payload: payload ?? null });
}

function isLanguageToken(value: string): boolean {
  return LANGUAGE_TOKEN_PATTERN.test(value.trim());
}

function parseTranslationArgs(raw: string): ParsedTrArgs | null {
  const input = raw.trimStart().replace(/^\/?tr(?:anslate)?\s+/i, "");
  if (!input.trim()) return null;

  const separatedMatch = input.match(/^\s*(\S+)\s*(?:->|=>)\s*(\S+)(?:\s+([\s\S]*))?$/);
  if (separatedMatch && isLanguageToken(separatedMatch[1]) && isLanguageToken(separatedMatch[2])) {
    return {
      src: separatedMatch[1] || DEFAULT_SRC,
      dst: separatedMatch[2] || DEFAULT_DST,
      text: separatedMatch[3] ?? "",
    };
  }

  const spacedDashMatch = input.match(/^\s*(\S+)\s+-\s+(\S+)(?:\s+([\s\S]*))?$/);
  if (spacedDashMatch && isLanguageToken(spacedDashMatch[1]) && isLanguageToken(spacedDashMatch[2])) {
    return {
      src: spacedDashMatch[1] || DEFAULT_SRC,
      dst: spacedDashMatch[2] || DEFAULT_DST,
      text: spacedDashMatch[3] ?? "",
    };
  }

  const commandMatch = input.match(/^\s*(\S+)\s+(\S+)(?:\s+([\s\S]*))?$/);
  if (commandMatch && isLanguageToken(commandMatch[1]) && isLanguageToken(commandMatch[2])) {
    return {
      src: commandMatch[1] || DEFAULT_SRC,
      dst: commandMatch[2] || DEFAULT_DST,
      text: commandMatch[3] ?? "",
    };
  }

  return {
    src: DEFAULT_SRC,
    dst: DEFAULT_DST,
    text: input.trim(),
  };
}

function formatCommandLine(src: string, dst: string, text: string): string {
  const normalizedSrc = src.trim() || DEFAULT_SRC;
  const normalizedDst = dst.trim() || DEFAULT_DST;
  const oneLineText = text.replace(/\s+/g, " ").trim();
  return oneLineText ? `${normalizedSrc} ${normalizedDst} ${oneLineText}` : `${normalizedSrc} ${normalizedDst}`;
}

function isAtStart(textarea: HTMLTextAreaElement): boolean {
  return textarea.selectionStart === 0 && textarea.selectionEnd === 0 && textarea.scrollTop <= 0;
}

function isAtEnd(textarea: HTMLTextAreaElement): boolean {
  const scrollBottom = textarea.scrollTop + textarea.clientHeight;
  const isScrolledToBottom = scrollBottom >= textarea.scrollHeight - 1;
  return (
    textarea.selectionStart === textarea.value.length &&
    textarea.selectionEnd === textarea.value.length &&
    isScrolledToBottom
  );
}

export function TranslationPanel({ onClose, initialArgs }: PanelProps) {
  const t = useI18n();
  const parsedInitial = parseTranslationArgs(initialArgs ?? "");
  const [src, setSrc] = useState(parsedInitial?.src ?? DEFAULT_SRC);
  const [dst, setDst] = useState(parsedInitial?.dst ?? DEFAULT_DST);
  const [text, setText] = useState(parsedInitial?.text ?? "");
  const [commandLine, setCommandLine] = useState(() =>
    parsedInitial
      ? formatCommandLine(parsedInitial.src, parsedInitial.dst, parsedInitial.text)
      : formatCommandLine(DEFAULT_SRC, DEFAULT_DST, ""),
  );
  const [result, setResult] = useState("");
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState("");
  const commandRef = useRef<HTMLInputElement>(null);
  const sourceRef = useRef<HTMLTextAreaElement>(null);
  const outputRef = useRef<HTMLTextAreaElement>(null);
  const pendingIdRef = useRef<string | null>(null);
  const lastSentKeyRef = useRef("");
  const lastAppliedInitialArgsRef = useRef<string | null>(null);

  const focusSection = useCallback((target: FocusTarget) => {
    const element =
      target === "command"
        ? commandRef.current
        : target === "source"
          ? sourceRef.current
          : outputRef.current;

    element?.focus();
    if (!element) return;

    if (target === "output") {
      element.setSelectionRange(0, 0);
      element.scrollTop = 0;
      return;
    }

    const cursorPosition = element.value.length;
    element.setSelectionRange(cursorPosition, cursorPosition);
  }, []);

  useEffect(() => {
    if (!window.__TAURI_INTERNALS__) return;
    const unlisten = listen<TranslationResponsePayload>("translation-result", (event) => {
      const { request_id, ok, text: translated, error: err } = event.payload;
      if (request_id !== pendingIdRef.current) return;

      pendingIdRef.current = null;
      setLoading(false);
      if (ok && translated !== undefined) {
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
        pendingIdRef.current = null;
        lastSentKeyRef.current = "";
        setLoading(false);
        setResult("");
        setError("");
        return;
      }

      const requestKey = `${nextSrc}\u0000${normalizedDst}\u0000${normalizedText}`;
      if (!force && requestKey === lastSentKeyRef.current) return;
      lastSentKeyRef.current = requestKey;

      const requestId = `tr-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`;
      pendingIdRef.current = requestId;
      setLoading(true);
      setError("");
      setResult("");
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

  const updateCommandLine = useCallback(
    (nextCommandLine: string) => {
      setCommandLine(nextCommandLine);
      const parsed = parseTranslationArgs(nextCommandLine);
      if (!parsed) {
        setSrc(DEFAULT_SRC);
        setDst(DEFAULT_DST);
        setText("");
        void translate(DEFAULT_SRC, DEFAULT_DST, "", false);
        return;
      }

      setSrc(parsed.src);
      setDst(parsed.dst);
      setText(parsed.text);
      void translate(parsed.src, parsed.dst, parsed.text, false);
    },
    [translate],
  );

  const updateSourceText = useCallback(
    (nextText: string) => {
      setText(nextText);
      setCommandLine(formatCommandLine(src, dst, nextText));
      void translate(src, dst, nextText, false);
    },
    [dst, src, translate],
  );

  useEffect(() => {
    const nextInitialArgs = initialArgs ?? "";
    if (lastAppliedInitialArgsRef.current === nextInitialArgs) return;
    lastAppliedInitialArgsRef.current = nextInitialArgs;

    const parsed = parseTranslationArgs(nextInitialArgs);
    const timer = window.setTimeout(() => {
      if (!parsed) {
        setSrc(DEFAULT_SRC);
        setDst(DEFAULT_DST);
        setText("");
        setCommandLine(formatCommandLine(DEFAULT_SRC, DEFAULT_DST, ""));
        void translate(DEFAULT_SRC, DEFAULT_DST, "", false);
        return;
      }

      setSrc(parsed.src);
      setDst(parsed.dst);
      setText(parsed.text);
      setCommandLine(formatCommandLine(parsed.src, parsed.dst, parsed.text));
      void translate(parsed.src, parsed.dst, parsed.text, false);
    }, 0);

    return () => window.clearTimeout(timer);
  }, [initialArgs, translate]);

  return (
    <div className="min-h-[350px] bg-gray-900/95 backdrop-blur-md rounded-b-xl shadow-2xl flex flex-col p-4 gap-3">
      <div className="flex items-center justify-between gap-2">
        <span className="text-xs font-semibold text-blue-400 uppercase tracking-wide">
          {t.translation.title}
        </span>
        <span className="text-[10px] text-gray-500">
          Google Translate | {src || DEFAULT_SRC} -&gt; {dst || DEFAULT_DST}
          {loading ? ` | ${t.translation.translating}` : ""}
        </span>
      </div>

      <input
        ref={commandRef}
        value={commandLine}
        onChange={(e) => updateCommandLine(e.target.value)}
        onKeyDown={(e) => {
          if (e.key === "Escape") {
            e.preventDefault();
            onClose();
            return;
          }
          if (e.key === "ArrowDown") {
            e.preventDefault();
            focusSection("source");
            return;
          }
          if (e.key === "ArrowUp") {
            e.preventDefault();
            focusSection("output");
          }
        }}
        placeholder="auto zh-TW text"
        className="bg-gray-800/70 text-gray-100 text-sm rounded px-3 py-2 outline-none placeholder-gray-600"
      />

      <textarea
        ref={sourceRef}
        value={text}
        onChange={(e) => updateSourceText(e.target.value)}
        onKeyDown={(e) => {
          if (e.key === "Escape") {
            e.preventDefault();
            onClose();
            return;
          }
          if (e.key === "ArrowUp" && isAtStart(e.currentTarget)) {
            e.preventDefault();
            focusSection("command");
            return;
          }
          if (e.key === "ArrowDown" && isAtEnd(e.currentTarget)) {
            e.preventDefault();
            focusSection("output");
          }
        }}
        placeholder={t.translation.textPlaceholder}
        rows={5}
        className="bg-gray-800/60 text-gray-200 text-sm rounded px-3 py-2 outline-none resize-none placeholder-gray-600 overflow-y-auto"
      />

      {error && (
        <p className="rounded bg-red-950/30 border border-red-900/40 px-3 py-2 text-xs text-red-300">
          {error}
        </p>
      )}

      <textarea
        ref={outputRef}
        value={result}
        readOnly
        onKeyDown={(e) => {
          if (e.key === "Escape") {
            e.preventDefault();
            onClose();
            return;
          }
          if (e.key === "ArrowUp" && isAtStart(e.currentTarget)) {
            e.preventDefault();
            focusSection("source");
            return;
          }
          if (e.key === "ArrowDown" && isAtEnd(e.currentTarget)) {
            e.preventDefault();
            focusSection("command");
          }
        }}
        placeholder={loading ? t.translation.translating : "Google Translate output"}
        rows={5}
        className="bg-gray-800/60 text-gray-100 text-sm rounded px-3 py-2 outline-none resize-none placeholder-gray-600 overflow-y-auto selection:bg-blue-500/40 selection:text-white"
      />

      <div className="text-[10px] text-gray-700">
        Command format: source destination text. Arrow keys move between command, source, and output.
      </div>
    </div>
  );
}
