import React, { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Markdown } from "./Markdown";
import { classifyAiError, type ClassifiedAiError } from "../utils/aiErrors";
import { useAgent } from "../hooks/useAgent";
import { useLocalHistory } from "../hooks/useLocalHistory";
import { useTextareaAutosize } from "../hooks/useTextareaAutosize";
import { IPC } from "../ipc/routes";
import type {
  AgentApproval,
  AgentFilteredSource,
  AgentPlannedAction,
  AgentRun,
  GroundingSource,
  ReactStep,
} from "../hooks/useAgent";
import { useAi } from "../hooks/useAi";
import type { AiSetupStatus } from "../hooks/useAi";
import { useI18n } from "../i18n/useI18n";
import type { PanelProps } from "../types/panel";

function riskTone(risk: AgentApproval["risk"]) {
  if (risk === "high") return "border-red-500/30 bg-red-500/10 text-red-200";
  if (risk === "medium") return "border-amber-500/30 bg-amber-500/10 text-amber-200";
  return "border-emerald-500/30 bg-emerald-500/10 text-emerald-200";
}

function ApprovalCard({
  approval,
  onApprove,
  onReject,
}: {
  approval: AgentApproval;
  onApprove: (remember: boolean) => void;
  onReject: () => void;
}) {
  const [remember, setRemember] = useState(false);
  const isReactGate =
    approval.planned_action === null || approval.planned_action === undefined;

  return (
    <div className={`rounded border px-3 py-2 ${riskTone(approval.risk)}`}>
      <div className="flex items-center justify-between gap-3">
        <div className="min-w-0">
          <p className="truncate text-xs font-medium text-current">{approval.summary}</p>
          <p className="text-[10px] uppercase text-current opacity-80">
            {isReactGate
              ? `react tool gate${approval.tool_name ? `: ${approval.tool_name}` : ""}`
              : (approval.planned_action?.kind ?? "planned_action")}{" "}
            · {approval.risk}
          </p>
        </div>
        <div className="flex shrink-0 items-center gap-2">
          <CountdownPill
            deadlineMs={approval.deadline_unix_ms ?? null}
            status={approval.status}
          />
        </div>
      </div>

      {approval.planned_action && (
        <div className="mt-2 rounded bg-gray-950/30 px-2 py-1">
          <ApprovalSummary action={approval.planned_action} />
        </div>
      )}

      <div className="mt-2 flex items-center justify-between gap-2">
        <label className="flex cursor-pointer items-center gap-1.5 text-[10px] text-current opacity-90">
          <input
            type="checkbox"
            checked={remember}
            onChange={(e) => setRemember(e.target.checked)}
            className="h-3 w-3 accent-emerald-500"
          />
          Approve and remember for this run
        </label>
        <div className="flex gap-2">
          <button
            onClick={() => onApprove(remember)}
            className="rounded bg-emerald-600/70 px-2 py-1 text-[10px] font-medium text-white hover:bg-emerald-500/70"
          >
            Approve
          </button>
          <button
            onClick={onReject}
            className="rounded bg-gray-700 px-2 py-1 text-[10px] font-medium text-gray-200 hover:bg-gray-600"
          >
            Reject
          </button>
        </div>
      </div>

      {approval.planned_action && (
        <details className="mt-2">
          <summary className="cursor-pointer text-[10px] uppercase text-gray-400 hover:text-gray-200">
            Show raw
          </summary>
          <pre className="mt-1 max-h-32 overflow-auto rounded bg-gray-950/40 p-2 text-[10px] text-gray-400">
            {payloadPreview(approval.planned_action.payload)}
          </pre>
        </details>
      )}
    </div>
  );
}

function CountdownPill({
  deadlineMs,
  status,
}: {
  deadlineMs: number | null | undefined;
  status: string;
}) {
  const [now, setNow] = useState(() => Date.now());
  useEffect(() => {
    if (!deadlineMs || status !== "pending") return;
    const id = window.setInterval(() => setNow(() => Date.now()), 1000);
    return () => window.clearInterval(id);
  }, [deadlineMs, status]);

  if (status === "approval_timeout") {
    return <span className="text-[10px] uppercase text-red-300">Timed out</span>;
  }
  if (!deadlineMs || status !== "pending") return null;

  const remainingMs = Math.max(0, deadlineMs - now);
  const totalSec = Math.ceil(remainingMs / 1000);
  const mm = Math.floor(totalSec / 60);
  const ss = totalSec % 60;
  const text = `${mm}:${ss.toString().padStart(2, "0")} left`;
  let tone = "text-gray-400";
  if (remainingMs <= 10_000) tone = "text-red-300";
  else if (remainingMs <= 30_000) tone = "text-amber-300";

  return <span className={`text-[10px] uppercase ${tone}`}>{text}</span>;
}

function ApprovalSummary({ action }: { action: AgentPlannedAction }) {
  // Human-friendly summary view keyed by `kind`. The raw JSON remains available
  // behind `Show raw` in the caller for audit trail integrity. Each kind shows
  // the 1–3 fields a reviewer cares about most; unknown kinds fall back to a
  // single-line label so we degrade gracefully when new kinds are added.
  const payload = (action.payload ?? {}) as Record<string, unknown>;
  const field = (key: string) =>
    typeof payload[key] === "string" || typeof payload[key] === "number"
      ? String(payload[key])
      : payload[key] !== undefined
        ? JSON.stringify(payload[key])
        : null;

  switch (action.kind) {
    case "terminal_command":
      return (
        <div className="space-y-0.5 text-[11px]">
          <p className="text-current">
            <span className="opacity-70">cmd: </span>
            <code className="rounded bg-gray-950/40 px-1">{field("program") ?? action.label}</code>
          </p>
          {field("cwd") && (
            <p className="text-current opacity-80">
              <span className="opacity-70">cwd: </span>
              {field("cwd")}
            </p>
          )}
        </div>
      );
    case "file_write": {
      const content = field("content") ?? "";
      const byteLen = typeof content === "string" ? content.length : 0;
      return (
        <div className="space-y-0.5 text-[11px]">
          <p>
            <span className="opacity-70">path: </span>
            <code className="rounded bg-gray-950/40 px-1">{field("path") ?? action.label}</code>
          </p>
          <p className="opacity-80">{byteLen} bytes</p>
        </div>
      );
    }
    case "create_note_draft":
      return (
        <div className="space-y-0.5 text-[11px]">
          <p>
            <span className="opacity-70">title: </span>
            {field("title") ?? action.label}
          </p>
          {field("body") && (
            <p className="opacity-80 line-clamp-2">{field("body")}</p>
          )}
        </div>
      );
    case "update_setting_draft":
      return (
        <div className="space-y-0.5 text-[11px]">
          <p>
            <span className="opacity-70">{field("key") ?? "setting"}: </span>
            <code className="rounded bg-gray-950/40 px-1">{field("value")}</code>
          </p>
        </div>
      );
    case "open_panel":
      return (
        <p className="text-[11px]">
          <span className="opacity-70">panel: </span>
          {field("panel") ?? action.label}
        </p>
      );
    case "run_builtin_command":
      return (
        <p className="text-[11px]">
          <span className="opacity-70">cmd: </span>
          <code className="rounded bg-gray-950/40 px-1">{field("command") ?? action.label}</code>
        </p>
      );
    case "system_control":
      return (
        <p className="text-[11px]">
          <span className="opacity-70">{field("control") ?? "system"}: </span>
          {field("value") ?? action.label}
        </p>
      );
    case "model_lifecycle":
      return (
        <p className="text-[11px]">
          <span className="opacity-70">{field("op") ?? "model"}: </span>
          {field("model") ?? action.label}
        </p>
      );
    default:
      return <p className="text-[11px] opacity-80">{action.label}</p>;
  }
}

function ErrorCard({
  classified,
  onRetry,
  onOpenSetting,
}: {
  classified: ClassifiedAiError;
  onRetry?: () => void;
  onOpenSetting?: () => void;
}) {
  return (
    <div className="space-y-1">
      <p className="text-xs font-semibold text-red-200">{classified.title}</p>
      <p className="text-[11px] text-red-200/80">{classified.hint}</p>
      {classified.cta && (
        <div className="pt-1">
          {classified.cta.action === "setting" && (
            <button
              onClick={onOpenSetting}
              className="rounded bg-red-500/20 px-2 py-0.5 text-[10px] uppercase text-red-100 hover:bg-red-500/30"
            >
              {classified.cta.label}
            </button>
          )}
          {classified.cta.action === "retry" && (
            <button
              onClick={onRetry}
              className="rounded bg-red-500/20 px-2 py-0.5 text-[10px] uppercase text-red-100 hover:bg-red-500/30"
            >
              {classified.cta.label}
            </button>
          )}
          {classified.cta.action.startsWith("external:") && (
            <a
              href={classified.cta.action.slice("external:".length)}
              target="_blank"
              rel="noreferrer"
              className="rounded bg-red-500/20 px-2 py-0.5 text-[10px] uppercase text-red-100 hover:bg-red-500/30"
            >
              {classified.cta.label}
            </a>
          )}
        </div>
      )}
    </div>
  );
}

function ElapsedTimer({ startedAt }: { startedAt: number }) {
  const [now, setNow] = useState(() => startedAt);
  useEffect(() => {
    const id = window.setInterval(() => setNow(() => Date.now()), 500);
    return () => window.clearInterval(id);
  }, []);
  const elapsedMs = Math.max(0, now - startedAt);
  const elapsedSec = (elapsedMs / 1000).toFixed(1);
  const isLong = elapsedMs >= 30_000;
  return (
    <span className="text-[10px] text-gray-500">
      Thinking… {elapsedSec}s
      {isLong && <span className="ml-1 text-amber-400/80">Long running — Cancel?</span>}
    </span>
  );
}

function payloadPreview(value: unknown) {
  try {
    return JSON.stringify(value, null, 2);
  } catch {
    return String(value);
  }
}

function commandResultLabel(result: AgentRun["command_result"]) {
  if (!result) return null;
  if (result.ui_type.type === "Inline") return "Inline response";
  if (result.ui_type.type === "Panel") return `Open panel: ${result.ui_type.value}`;
  return `Terminal launch: ${result.ui_type.value.program}`;
}

function runStatusLabel(run: AgentRun) {
  if (run.status === "completed" && run.approvals.length === 0 && !run.command_result) {
    return "answered";
  }
  return run.status;
}

function SourceList({ sources }: { sources: GroundingSource[] }) {
  if (sources.length === 0) {
    return <p className="text-[11px] text-gray-500">No included grounding sources.</p>;
  }
  return (
    <div className="space-y-1">
      {sources.slice(0, 4).map((source) => (
        <div key={source.source_id} className="rounded bg-gray-950/30 px-2 py-1">
          <div className="flex items-center justify-between gap-2">
            <span className="truncate text-[11px] font-medium text-gray-200">{source.title}</span>
            <span className="shrink-0 text-[10px] uppercase text-gray-500">
              {source.source_type}
            </span>
          </div>
          <p className="mt-0.5 line-clamp-2 text-[11px] text-gray-500">{source.snippet}</p>
        </div>
      ))}
    </div>
  );
}

function FilteredSourceList({ sources }: { sources: AgentFilteredSource[] }) {
  if (sources.length === 0) {
    return <p className="text-[11px] text-gray-500">No filtered private or secret sources.</p>;
  }
  return (
    <div className="space-y-1">
      {sources.slice(0, 4).map((source) => (
        <div key={source.source_id} className="rounded bg-gray-950/30 px-2 py-1">
          <div className="flex items-center justify-between gap-2">
            <span className="truncate text-[11px] font-medium text-gray-200">{source.title}</span>
            <span className="shrink-0 text-[10px] uppercase text-rose-300/80">{source.reason}</span>
          </div>
          <p className="mt-0.5 text-[11px] text-gray-500">
            {source.source_type} · {source.visibility}
          </p>
        </div>
      ))}
    </div>
  );
}

function stepStatusBadge(status: string) {
  switch (status) {
    case "executing":
      return "animate-pulse bg-blue-500/30 text-blue-200";
    case "waiting_approval":
      return "animate-pulse bg-amber-500/30 text-amber-200";
    case "approved_executed":
    case "executed":
      return "bg-emerald-500/20 text-emerald-300";
    case "approval_rejected":
    case "approval_timeout":
      return "bg-red-500/20 text-red-300";
    case "final":
      return "bg-emerald-600/30 text-emerald-200 font-semibold";
    default:
      return "bg-gray-700/30 text-gray-400";
  }
}

function ReactStepTimeline({ steps }: { steps: ReactStep[] }) {
  if (steps.length === 0) return null;
  const obsCount = steps.filter((s) => s.observation_preview).length;
  return (
    <div className="mt-3 space-y-1 rounded bg-gray-900/40 px-2 py-2">
      <div className="mb-1 flex items-center gap-2">
        <span className="text-[10px] uppercase text-gray-500">ReAct steps</span>
        {obsCount > 0 && (
          <span className="rounded bg-gray-800 px-1.5 py-0.5 text-[10px] text-gray-400">
            {obsCount} obs
          </span>
        )}
      </div>
      {steps.map((s, i) => (
        <div key={i} className="flex items-center gap-2 text-[11px]">
          <span className="w-4 shrink-0 text-right text-[10px] text-gray-600">{s.step + 1}</span>
          <span
            className={`shrink-0 rounded px-1.5 py-0.5 text-[10px] uppercase ${stepStatusBadge(s.status)}`}
          >
            {s.status.replace(/_/g, " ")}
          </span>
          <span className="min-w-0 flex-1 truncate text-gray-400">
            {s.tool_name ?? "(thinking)"}
          </span>
          {s.observation_preview && (
            <span className="ml-1 shrink-0 max-w-[38%] truncate text-right text-[10px] text-gray-600">
              {s.observation_preview}
            </span>
          )}
        </div>
      ))}
    </div>
  );
}

function SetupCard({
  status,
  onDismiss,
  onRecheck,
}: {
  status: AiSetupStatus;
  onDismiss: () => void;
  onRecheck: () => void;
}) {
  const isOllama = status.provider === "ollama";
  const needsInstall = isOllama && !status.ollama_reachable;
  const needsPull = isOllama && status.ollama_reachable && !status.model_available;
  const recommended = status.recommended_model || "qwen2.5:7b";

  return (
    <div className="mx-auto mt-4 max-w-md rounded-lg border border-amber-500/30 bg-amber-500/5 p-4 text-sm">
      <p className="mb-3 font-semibold text-amber-300">AI Setup Required</p>
      {status.reason && (
        <p className="mb-3 text-xs text-amber-200/70">{status.reason}</p>
      )}

      <ol className="space-y-3 text-gray-300">
        {isOllama && (
          <li className={`flex gap-2 ${!needsInstall ? "opacity-40" : ""}`}>
            <span className="mt-0.5 flex h-5 w-5 shrink-0 items-center justify-center rounded-full border border-amber-500/50 text-[10px] font-bold text-amber-300">
              1
            </span>
            <div>
              <p className="text-xs font-medium">Install Ollama</p>
              <a
                href="https://ollama.com/download"
                target="_blank"
                rel="noreferrer"
                className="text-[11px] text-blue-400 underline hover:text-blue-300"
              >
                ollama.com/download
              </a>
            </div>
          </li>
        )}

        {isOllama && (
          <li className={`flex gap-2 ${!needsPull && !needsInstall ? "opacity-40" : ""}`}>
            <span className="mt-0.5 flex h-5 w-5 shrink-0 items-center justify-center rounded-full border border-amber-500/50 text-[10px] font-bold text-amber-300">
              2
            </span>
            <div>
              <p className="text-xs font-medium">Pull a model</p>
              <code className="mt-0.5 block rounded bg-gray-800 px-2 py-1 text-[11px] text-green-300">
                ollama pull {recommended}
              </code>
              <p className="mt-1 text-[10px] text-gray-500">
                Recommended for your system. Smaller options: qwen2.5:1.5b, llama3.2:1b
              </p>
            </div>
          </li>
        )}

        <li className="flex gap-2">
          <span className="mt-0.5 flex h-5 w-5 shrink-0 items-center justify-center rounded-full border border-gray-600 text-[10px] font-bold text-gray-400">
            {isOllama ? "3" : "1"}
          </span>
          <div>
            <p className="text-xs font-medium text-gray-400">
              Or switch to OpenAI / Claude
            </p>
            <p className="text-[11px] text-gray-500">
              Open <span className="text-gray-300">/setting</span> and set{" "}
              <span className="text-gray-300">ai.provider</span> and your API key.
            </p>
          </div>
        </li>
      </ol>

      <div className="mt-4 flex gap-2">
        <button
          onClick={onRecheck}
          className="rounded bg-amber-600/40 px-3 py-1 text-xs text-amber-200 hover:bg-amber-600/60"
        >
          Re-check
        </button>
        <button
          onClick={onDismiss}
          className="rounded bg-gray-700/60 px-3 py-1 text-xs text-gray-400 hover:text-gray-200"
        >
          Use anyway
        </button>
      </div>
    </div>
  );
}

export function AiPanel({ onClose, onRunCommandResult }: PanelProps) {
  const t = useI18n();
  const { messages, loading, send, cancel: cancelChat, clearHistory, checkSetup } = useAi();
  const agent = useAgent();
  const [input, setInput] = useState("");
  const [mode, setMode] = useState<"chat" | "agent">("chat");
  const [setupStatus, setSetupStatus] = useState<AiSetupStatus | null>(null);
  const [setupDismissed, setSetupDismissed] = useState(false);
  const bottomRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLTextAreaElement>(null);
  const deliveredResultRef = useRef<string | null>(null);
  const chatHistory = useLocalHistory("keynova.chatHistory", 50);
  const agentHistory = useLocalHistory("keynova.agentHistory", 50);
  const [showAuditByDefault, setShowAuditByDefault] = useState(false);
  useTextareaAutosize(inputRef, input, { min: 1, max: 8 });

  useEffect(() => {
    if (!window.__TAURI_INTERNALS__) return;
    void invoke<string>("cmd_dispatch", {
      route: IPC.SETTING_GET,
      payload: { key: "agent.show_audit_by_default" },
    })
      .then((value) => {
        setShowAuditByDefault(value?.trim().toLowerCase() === "true");
      })
      .catch(() => {});
  }, []);

  useEffect(() => {
    bottomRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [messages, agent.runs]);

  useEffect(() => {
    inputRef.current?.focus();
  }, []);

  useEffect(() => {
    void checkSetup().then((s) => {
      if (s?.needs_setup) setSetupStatus(s);
    });
  }, [checkSetup]);

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
      return;
    }
    if ((e.key === "ArrowUp" || e.key === "ArrowDown") && input === "") {
      // History recall only when input is empty so multi-line edits aren't hijacked.
      e.preventDefault();
      const history = mode === "agent" ? agentHistory : chatHistory;
      const recalled = history.recall(e.key === "ArrowUp" ? -1 : 1);
      if (recalled !== null) setInput(recalled);
    }
  }

  async function handleRecheck() {
    const s = await checkSetup();
    if (s?.needs_setup) {
      setSetupStatus(s);
      setSetupDismissed(false);
    } else {
      setSetupStatus(null);
    }
  }

  async function handleSend() {
    const prompt = input.trim();
    if (!prompt || loading || agent.loading) return;
    setInput("");
    if (mode === "agent") {
      agentHistory.append(prompt);
      await agent.start(prompt);
    } else {
      chatHistory.append(prompt);
      await send(prompt);
    }
  }

  function lastUserPromptForCurrentMode(): string | null {
    if (mode === "chat") {
      for (let i = messages.length - 1; i >= 0; i--) {
        if (messages[i].role === "user") return messages[i].content;
      }
      return null;
    }
    return agent.runs[agent.runs.length - 1]?.prompt ?? null;
  }

  async function handleRegenerate() {
    const prompt = lastUserPromptForCurrentMode();
    if (!prompt) return;
    if (mode === "agent") {
      await agent.start(prompt);
    } else {
      await send(prompt);
    }
  }

  async function handleCopy(text: string) {
    try {
      await navigator.clipboard.writeText(text);
    } catch {
      // Clipboard permission denied — silent no-op; ErrorCard pattern is overkill here.
    }
  }

  async function handleClear() {
    if (mode === "agent") {
      await agent.clearRuns();
      return;
    }
    await clearHistory();
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
                  mode === item ? "bg-blue-600/60 text-white" : "text-gray-500 hover:text-gray-300"
                }`}
              >
                {item}
              </button>
            ))}
          </div>
        </div>
        <button
          onClick={() => void handleClear()}
          className="text-xs text-gray-600 hover:text-gray-400 transition-colors"
        >
          {t.ai.clear}
        </button>
      </div>

      <div
        className="flex-1 overflow-y-auto px-4 py-2 space-y-3"
        style={{ minHeight: 560, maxHeight: 560 }}
      >
        {setupStatus?.needs_setup && !setupDismissed && (
          <SetupCard
            status={setupStatus}
            onDismiss={() => setSetupDismissed(true)}
            onRecheck={() => void handleRecheck()}
          />
        )}

        {mode === "chat" && messages.length === 0 && !setupStatus?.needs_setup && (
          <p className="text-xs text-gray-600 text-center mt-4">{t.ai.placeholder}</p>
        )}
        {mode === "chat" &&
          messages.map((msg, i) => (
            <div
              key={i}
              className={`group flex ${msg.role === "user" ? "justify-end" : "justify-start"}`}
            >
              <div
                className={`relative max-w-[85%] rounded-lg px-3 py-2 text-sm ${
                  msg.role === "user"
                    ? "bg-blue-600/60 text-white"
                    : msg.error
                      ? "bg-red-900/40 text-red-300"
                      : "bg-gray-700/60 text-gray-200"
                }`}
              >
                {msg.role === "assistant" && !msg.pending && !msg.error && msg.content && (
                  <div className="absolute -top-2 right-2 hidden gap-1 group-hover:flex">
                    <button
                      onClick={() => void handleCopy(msg.content)}
                      className="rounded bg-gray-900/80 px-1.5 py-0.5 text-[10px] uppercase text-gray-300 hover:bg-gray-800 hover:text-white"
                    >
                      Copy
                    </button>
                    <button
                      onClick={() => void handleRegenerate()}
                      className="rounded bg-gray-900/80 px-1.5 py-0.5 text-[10px] uppercase text-gray-300 hover:bg-gray-800 hover:text-white"
                    >
                      Regenerate
                    </button>
                  </div>
                )}
                {msg.pending ? (
                  <div className="space-y-1">
                    {msg.content ? (
                      <Markdown content={msg.content} />
                    ) : (
                      <span className="animate-pulse text-gray-400">{t.ai.sending}</span>
                    )}
                    <div className="flex items-center justify-between gap-2">
                      {msg.startedAt ? <ElapsedTimer startedAt={msg.startedAt} /> : <span />}
                      <button
                        onClick={() => void cancelChat()}
                        className="text-[10px] text-gray-500 hover:text-gray-300"
                      >
                        Cancel
                      </button>
                    </div>
                  </div>
                ) : msg.error ? (
                  <ErrorCard classified={classifyAiError(msg.error)} />
                ) : msg.role === "assistant" ? (
                  <Markdown content={msg.content} />
                ) : (
                  <pre className="whitespace-pre-wrap font-sans">{msg.content}</pre>
                )}
              </div>
            </div>
          ))}

        {mode === "agent" && agent.runs.length === 0 && (
          <p className="text-xs text-gray-600 text-center mt-4">
            Agent mode is a guarded planner: it searches local context, redacts private sources, and
            asks before touching panels, files, terminal, system, or models.
          </p>
        )}

        {mode === "agent" && agent.archivedCount > 0 && (
          <div className="rounded border border-gray-700/40 bg-gray-900/30 px-3 py-1 text-[10px] text-gray-500">
            Archived {agent.archivedCount} run{agent.archivedCount === 1 ? "" : "s"} this session
            (oldest evicted from in-memory cap; available via `agent_archive` table).
          </div>
        )}

        {mode === "agent" &&
          agent.runs.map((run) => {
            const pendingApprovals = run.approvals.filter(
              (approval) => approval.status === "pending",
            );
            const runReactSteps = agent.reactSteps[run.id] ?? [];
            const audit = run.prompt_audit;
            const toolCalls = run.steps.flatMap((step) =>
              step.tool_calls.map((tool) => ({ ...tool, stepTitle: step.title })),
            );
            const deliveredAction = commandResultLabel(run.command_result);
            return (
              <div
                key={run.id}
                className="rounded border border-gray-700/60 bg-gray-800/60 px-3 py-2 text-sm text-gray-200"
              >
                <div className="mb-2 flex items-center justify-between">
                  <span className="text-xs font-semibold uppercase tracking-wide text-blue-300">
                    {runStatusLabel(run)}
                  </span>
                  {(run.status === "waiting_approval" ||
                    run.status === "running" ||
                    run.status === "planning") && (
                    <button
                      onClick={() => void agent.cancel(run.id)}
                      className="text-[10px] text-gray-500 hover:text-gray-300"
                    >
                      Cancel
                    </button>
                  )}
                </div>
                <p className="mb-2 text-xs text-gray-400">{run.prompt}</p>

                {audit && (
                  <div className="mb-2 grid grid-cols-4 gap-1 text-center text-[10px] uppercase text-gray-500">
                    <div className="rounded bg-gray-950/30 px-1 py-1">
                      <div className="text-gray-300">{audit.included_sources.length}</div>
                      Included
                    </div>
                    <div className="rounded bg-gray-950/30 px-1 py-1">
                      <div className="text-gray-300">{audit.filtered_sources.length}</div>
                      Filtered
                    </div>
                    <div className="rounded bg-gray-950/30 px-1 py-1">
                      <div className="text-gray-300">{audit.redacted_secret_count}</div>
                      Secrets
                    </div>
                    <div className="rounded bg-gray-950/30 px-1 py-1">
                      <div className="text-gray-300">
                        {audit.prompt_chars}/{audit.budget_chars}
                      </div>
                      Budget
                    </div>
                  </div>
                )}

                <ul className="list-disc space-y-1 pl-4 text-xs text-gray-300">
                  {run.plan.map((step) => (
                    <li key={step}>{step}</li>
                  ))}
                </ul>

                <ReactStepTimeline steps={runReactSteps} />

                {pendingApprovals.length > 0 && (
                  <div className="mt-3 space-y-2">
                    <p className="text-[10px] uppercase text-amber-400/80">
                      Pending approval ({pendingApprovals.length})
                    </p>
                    {pendingApprovals.map((approval) => (
                      <ApprovalCard
                        key={approval.id}
                        approval={approval}
                        onApprove={(remember) => void agent.approve(run.id, approval.id, remember)}
                        onReject={() => void agent.reject(run.id, approval.id)}
                      />
                    ))}
                  </div>
                )}

                {deliveredAction && (
                  <div className="mt-3 rounded border border-emerald-500/20 bg-emerald-500/10 px-2 py-1 text-xs text-emerald-200">
                    Delivered action: {deliveredAction}
                  </div>
                )}

                {toolCalls.length > 0 && (
                  <details className="mt-3 rounded bg-gray-900/40 px-2 py-1">
                    <summary className="cursor-pointer text-[10px] uppercase text-gray-400 hover:text-gray-200">
                      Tool calls ({toolCalls.length})
                    </summary>
                    <div className="mt-2 space-y-1">
                      {toolCalls.map((tool) => (
                        <div
                          key={tool.id}
                          className="flex items-center justify-between gap-2 text-[11px] text-gray-400"
                        >
                          <span className="truncate">
                            {tool.tool_name} · {tool.stepTitle}
                          </span>
                          <span className="shrink-0">
                            {tool.status}
                            {tool.duration_ms != null ? ` · ${tool.duration_ms}ms` : ""}
                          </span>
                        </div>
                      ))}
                    </div>
                  </details>
                )}

                {(run.sources.length > 0 || audit) && (
                  <details
                    className="mt-3 rounded bg-gray-900/40 px-2 py-1"
                    open={showAuditByDefault}
                  >
                    <summary className="cursor-pointer text-[10px] uppercase text-gray-400 hover:text-gray-200">
                      Context audit
                    </summary>
                    <div className="mt-2 grid gap-2 md:grid-cols-2">
                      <div>
                        <p className="mb-1 text-[10px] uppercase text-gray-500">Included sources</p>
                        <SourceList sources={audit?.included_sources ?? run.sources} />
                      </div>
                      <div>
                        <p className="mb-1 text-[10px] uppercase text-gray-500">Filtered sources</p>
                        <FilteredSourceList sources={audit?.filtered_sources ?? []} />
                      </div>
                    </div>
                    {audit?.truncated && (
                      <p className="mt-2 text-[11px] text-amber-300/80">
                        Context was truncated to stay inside the prompt budget.
                      </p>
                    )}
                  </details>
                )}

                {run.memory_refs.length > 0 && (
                  <details className="mt-3 rounded bg-gray-900/40 px-2 py-1">
                    <summary className="cursor-pointer text-[10px] uppercase text-gray-400 hover:text-gray-200">
                      Memory refs ({run.memory_refs.length})
                    </summary>
                    <div className="mt-2 space-y-1">
                      {run.memory_refs.slice(0, 4).map((memory) => (
                        <div key={memory.id} className="rounded bg-gray-950/30 px-2 py-1">
                          <div className="text-[10px] uppercase text-gray-500">
                            {memory.scope} · {memory.visibility}
                          </div>
                          <p className="text-[11px] text-gray-400">{memory.summary}</p>
                        </div>
                      ))}
                    </div>
                  </details>
                )}

                {run.status === "completed" && run.output && (
                  <div className="group/finalanswer relative mt-3 rounded border border-emerald-500/25 bg-emerald-500/5 px-3 py-2">
                    <div className="mb-1 flex items-center justify-between">
                      <p className="text-[10px] uppercase tracking-wide text-emerald-400/70">
                        Final Answer
                      </p>
                      <div className="hidden gap-1 group-hover/finalanswer:flex">
                        <button
                          onClick={() => void handleCopy(run.output ?? "")}
                          className="rounded bg-gray-900/80 px-1.5 py-0.5 text-[10px] uppercase text-gray-300 hover:bg-gray-800 hover:text-white"
                        >
                          Copy
                        </button>
                        <button
                          onClick={() => void handleRegenerate()}
                          className="rounded bg-gray-900/80 px-1.5 py-0.5 text-[10px] uppercase text-gray-300 hover:bg-gray-800 hover:text-white"
                        >
                          Regenerate
                        </button>
                      </div>
                    </div>
                    <Markdown content={run.output} className="text-xs text-gray-200" />
                  </div>
                )}
                {run.status !== "completed" && run.output && (
                  <pre className="mt-2 whitespace-pre-wrap text-xs text-gray-400">{run.output}</pre>
                )}

                {run.error && (
                  <pre className="mt-2 whitespace-pre-wrap rounded bg-red-950/30 p-2 text-xs text-red-300">
                    {run.error}
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
