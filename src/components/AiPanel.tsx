import { useEffect, useRef, useState } from "react";
import { useAi } from "../hooks/useAi";
import { useI18n } from "../i18n/useI18n";

export function AiPanel() {
  const t = useI18n();
  const { messages, loading, send, clearHistory } = useAi();
  const [input, setInput] = useState("");
  const bottomRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLTextAreaElement>(null);

  useEffect(() => {
    bottomRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [messages]);

  useEffect(() => {
    inputRef.current?.focus();
  }, []);

  function handleKeyDown(e: React.KeyboardEvent<HTMLTextAreaElement>) {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      void handleSend();
    }
  }

  async function handleSend() {
    const prompt = input.trim();
    if (!prompt || loading) return;
    setInput("");
    await send(prompt);
  }

  return (
    <div className="bg-gray-900/95 backdrop-blur-md rounded-b-xl shadow-2xl flex flex-col" style={{ maxHeight: 420 }}>
      {/* Header */}
      <div className="flex items-center justify-between px-4 py-2 border-b border-gray-700/50">
        <span className="text-xs font-semibold text-blue-400 uppercase tracking-wide">
          {t.ai.title}
        </span>
        <button
          onClick={() => void clearHistory()}
          className="text-xs text-gray-600 hover:text-gray-400 transition-colors"
        >
          {t.ai.clear}
        </button>
      </div>

      {/* Messages */}
      <div className="flex-1 overflow-y-auto px-4 py-2 space-y-3" style={{ minHeight: 120, maxHeight: 280 }}>
        {messages.length === 0 && (
          <p className="text-xs text-gray-600 text-center mt-4">{t.ai.placeholder}</p>
        )}
        {messages.map((msg, i) => (
          <div key={i} className={`flex ${msg.role === "user" ? "justify-end" : "justify-start"}`}>
            <div
              className={`max-w-[85%] rounded-lg px-3 py-2 text-sm ${
                msg.role === "user"
                  ? "bg-blue-600/60 text-white"
                  : msg.error
                  ? "bg-red-900/40 text-red-300"
                  : "bg-gray-700/60 text-gray-200"
              }`}
            >
              {msg.pending ? (
                <span className="animate-pulse text-gray-400">{t.ai.sending}</span>
              ) : msg.error ? (
                <span>{t.ai.error}: {msg.error}</span>
              ) : (
                <pre className="whitespace-pre-wrap font-sans">{msg.content}</pre>
              )}
            </div>
          </div>
        ))}
        <div ref={bottomRef} />
      </div>

      {/* Input */}
      <div className="px-4 py-2 border-t border-gray-700/50">
        <div className="flex gap-2">
          <textarea
            ref={inputRef}
            value={input}
            onChange={(e) => setInput(e.target.value)}
            onKeyDown={handleKeyDown}
            placeholder={t.ai.placeholder}
            rows={1}
            disabled={loading}
            className="flex-1 bg-gray-800/60 text-gray-100 placeholder-gray-600 text-sm rounded px-3 py-2 outline-none resize-none disabled:opacity-50"
            style={{ minHeight: 36, maxHeight: 100 }}
          />
          <button
            onClick={() => void handleSend()}
            disabled={loading || !input.trim()}
            className="px-3 py-1 bg-blue-600/70 text-white text-xs rounded hover:bg-blue-500/70 disabled:opacity-40 transition-colors"
          >
            {loading ? "…" : "↵"}
          </button>
        </div>
        <div className="mt-1 text-[10px] text-gray-700">Enter 送出 · Shift+Enter 換行 · Esc 關閉</div>
      </div>
    </div>
  );
}