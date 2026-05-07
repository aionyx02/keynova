import React, { useEffect, useRef, useState } from "react";
import { useAgent } from "../hooks/useAgent";
import { useAi } from "../hooks/useAi";
import { useI18n } from "../i18n/useI18n";
import type { PanelProps } from "../types/panel";

export function AiPanel({ onClose, onRunCommandResult }: PanelProps) {
  const t = useI18n();
  const { messages, loading, send, clearHistory } = useAi();
  const agent = useAgent();
  const [input, setInput] = useState("");
  const [mode, setMode] = useState<"chat" | "agent">("chat");
  const bottomRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLTextAreaElement>(null);
  const deliveredResultRef = useRef<string | null>(null);

  useEffect(() => {
    bottomRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [messages, agent.runs]);

  useEffect(() => {
    inputRef.current?.focus();
  }, []);

  useEffect(() => {
    const latestRun = agent.runs[0];
    if (!latestRun?.command_result) return;
    if (latestRun.command_result.ui_type.type === "Inline") return;
    const key = `${latestRun.id}:${JSON.stringify(latestRun.command_result)}`;
    if (deliveredResultRef.current === key) return;
    deliveredResultRef.current = key;
    onRunCommandResult?.(latestRun.command_result);
  }, [agent.runs, onRunCommandResult]);

  function handleKeyDown(e: React.KeyboardEvent<HTMLTextAreaElement>) {
    if (e.key === "Escape") {
      e.preventDefault();
      onClose();
      return;
    }
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      void handleSend();
    }
  }

  async function handleSend() {
    const prompt = input.trim();
    if (!prompt || loading || agent.loading) return;
    setInput("");
    if (mode === "agent") {
      await agent.start(prompt);
    } else {
      await send(prompt);
    }
  }

  return (
    <div
      className="bg-gray-900/95 backdrop-blur-md rounded-b-xl shadow-2xl flex flex-col"
      style={{ maxHeight: 3000 }}
    >
      <div className="flex items-center justify-between px-4 py-2 border-b border-gray-700/50">
        <div className="flex items-center gap-2">
          <span className="text-xs font-semibold text-blue-400 uppercase tracking-wide">
            {t.ai.title}
          </span>
          <div className="flex overflow-hidden rounded border border-gray-700">
            {(["chat", "agent"] as const).map((item) => (
              <button
                key={item}
                onClick={() => setMode(item)}
                className={`px-2 py-0.5 text-[10px] uppercase ${
                  mode === item
                    ? "bg-blue-600/60 text-white"
                    : "text-gray-500 hover:text-gray-300"
                }`}
              >
                {item}
              </button>
            ))}
          </div>
        </div>
        <button
          onClick={() => void clearHistory()}
          className="text-xs text-gray-600 hover:text-gray-400 transition-colors"
        >
          {t.ai.clear}
        </button>
      </div>

      <div
        className="flex-1 overflow-y-auto px-4 py-2 space-y-3"
        style={{ minHeight: 560, maxHeight: 560 }}
      >
        {mode === "chat" && messages.length === 0 && (
          <p className="text-xs text-gray-600 text-center mt-4">{t.ai.placeholder}</p>
        )}
        {mode === "chat" &&
          messages.map((msg, i) => (
            <div
              key={i}
              className={`flex ${msg.role === "user" ? "justify-end" : "justify-start"}`}
            >
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
                  <span>
                    {t.ai.error}: {msg.error}
                  </span>
                ) : (
                  <pre className="whitespace-pre-wrap font-sans">{msg.content}</pre>
                )}
              </div>
            </div>
          ))}

        {mode === "agent" && agent.runs.length === 0 && (
          <p className="text-xs text-gray-600 text-center mt-4">
            Agent mode waits for approval before local actions.
          </p>
        )}

        {mode === "agent" &&
          agent.runs.map((run) => {
            const pendingApprovals = run.approvals.filter(
              (approval) => approval.status === "pending",
            );
            return (
              <div key={run.id} className="rounded bg-gray-800/60 px-3 py-2 text-sm text-gray-200">
                <div className="mb-2 flex items-center justify-between">
                  <span className="text-xs font-semibold uppercase text-blue-300">
                    {run.status}
                  </span>
                  {run.status === "waiting_approval" && (
                    <button
                      onClick={() => void agent.cancel(run.id)}
                      className="text-[10px] text-gray-500 hover:text-gray-300"
                    >
                      Cancel
                    </button>
                  )}
                </div>
                <p className="mb-2 text-xs text-gray-400">{run.prompt}</p>
                <ul className="list-disc space-y-1 pl-4 text-xs text-gray-300">
                  {run.plan.map((step) => (
                    <li key={step}>{step}</li>
                  ))}
                </ul>

                {pendingApprovals.length > 0 && (
                  <div className="mt-3 space-y-2">
                    {pendingApprovals.map((approval) => (
                      <div
                        key={approval.id}
                        className="rounded border border-amber-500/30 bg-amber-500/10 px-3 py-2"
                      >
                        <div className="flex items-center justify-between gap-3">
                          <div>
                            <p className="text-xs font-medium text-amber-200">
                              {approval.summary}
                            </p>
                            <p className="text-[10px] uppercase text-amber-300/80">
                              {approval.risk}
                            </p>
                          </div>
                          <div className="flex gap-2">
                            <button
                              onClick={() => void agent.approve(run.id, approval.id)}
                              className="rounded bg-emerald-600/70 px-2 py-1 text-[10px] font-medium text-white hover:bg-emerald-500/70"
                            >
                              Approve
                            </button>
                            <button
                              onClick={() => void agent.reject(run.id, approval.id)}
                              className="rounded bg-gray-700 px-2 py-1 text-[10px] font-medium text-gray-200 hover:bg-gray-600"
                            >
                              Reject
                            </button>
                          </div>
                        </div>
                      </div>
                    ))}
                  </div>
                )}

                {run.output && (
                  <pre className="mt-2 whitespace-pre-wrap text-xs text-gray-400">
                    {run.output}
                  </pre>
                )}
              </div>
            );
          })}
        <div ref={bottomRef} />
      </div>

      <div className="px-4 py-2 border-t border-gray-700/50">
        <div className="flex gap-2">
          <textarea
            ref={inputRef}
            value={input}
            onChange={(e) => setInput(e.target.value)}
            onKeyDown={handleKeyDown}
            placeholder={t.ai.placeholder}
            rows={1}
            disabled={loading || agent.loading}
            className="flex-1 bg-gray-800/60 text-gray-100 placeholder-gray-600 text-sm rounded px-3 py-2 outline-none resize-none disabled:opacity-50"
            style={{ minHeight: 36, maxHeight: 100 }}
          />
          <button
            onClick={() => void handleSend()}
            disabled={loading || agent.loading || !input.trim()}
            className="px-3 py-1 bg-blue-600/70 text-white text-xs rounded hover:bg-blue-500/70 disabled:opacity-40 transition-colors"
          >
            {loading || agent.loading ? "..." : "Send"}
          </button>
        </div>
        <div className="mt-1 text-[10px] text-gray-700">
          Enter to send, Shift+Enter for a new line, Esc to close.
        </div>
      </div>
    </div>
  );
}
