#!/usr/bin/env node

import fs from "node:fs";
import path from "node:path";
import { ROOT, parseFrontmatter, repoPath } from "./docs-utils.mjs";

const ACTIVE_TASKS = "docs/tasks/active.md";
const REFACTOR_PLAN = "docs/tasks/refactor-ai-capability.md";
const STATE_TARGET = "docs/state/tasks.json";
const SUMMARY_TARGET = "docs/state/tasks-summary.json";
const SUGGESTIONS_SOURCE = "docs/state/workbench-suggestions.json";
const WORKBENCH_TARGET = "docs/workbench/tasks.html";
const ADR_TEMPLATE_TARGET = "docs/workbench/adr-preview-template.html";

function readMarkdown(relPath) {
  const fullPath = path.join(ROOT, relPath);
  const raw = fs.readFileSync(fullPath, "utf8");
  return parseFrontmatter(raw).body;
}

function writeIfChanged(relPath, content) {
  const fullPath = path.join(ROOT, relPath);
  fs.mkdirSync(path.dirname(fullPath), { recursive: true });
  const normalized = content.replace(/\r\n/g, "\n");
  if (fs.existsSync(fullPath)) {
    const current = fs.readFileSync(fullPath, "utf8").replace(/\r\n/g, "\n");
    if (current === normalized) {
      return false;
    }
  }
  fs.writeFileSync(fullPath, normalized, "utf8");
  return true;
}

function stripTrailingPeriod(text) {
  return text.trim().replace(/\.$/, "");
}

function parseActiveTasks(body) {
  const tasks = new Map();
  let priority = "unscoped";
  for (const line of body.split("\n")) {
    const priorityMatch = line.match(/^###\s+(.+?)\s*$/);
    if (priorityMatch) {
      priority = priorityMatch[1].trim();
      continue;
    }

    const taskMatch = line.match(/^- \[(x|X| )\]\s+`([^`]+)`\s+(.+)$/);
    if (!taskMatch) {
      continue;
    }

    tasks.set(taskMatch[2], {
      id: taskMatch[2],
      status: taskMatch[1].trim().toLowerCase() === "x" ? "done" : "open",
      priority,
      summary: stripTrailingPeriod(taskMatch[3]),
    });
  }
  return tasks;
}

function headingPositions(body) {
  return [...body.matchAll(/^###\s+(REF\.\d+)\s+-\s+(.+)$/gm)].map((match) => ({
    id: match[1],
    title: match[2].trim(),
    index: match.index,
  }));
}

function parseBullets(block, label) {
  const lines = block.split("\n");
  const output = [];
  let collecting = false;

  for (const line of lines) {
    if (line.trim() === `${label}:`) {
      collecting = true;
      continue;
    }

    if (!collecting) {
      continue;
    }

    if (line.trim() === "") {
      if (output.length > 0) {
        break;
      }
      continue;
    }

    const bullet = line.match(/^\s*-\s+(.+)$/);
    if (bullet) {
      output.push(bullet[1].trim());
      continue;
    }

    if (/^[A-Z][A-Za-z -]+:/.test(line.trim())) {
      break;
    }
  }

  return output;
}

function parseScalar(block, label) {
  const match = block.match(new RegExp(`^${label}:[ \\t]*(.+)$`, "m"));
  return match ? match[1].trim() : "";
}

function parsePlanTasks(body) {
  const headings = headingPositions(body);
  return headings.map((heading, index) => {
    const next = headings[index + 1];
    const block = body.slice(heading.index, next ? next.index : body.length);
    return {
      id: heading.id,
      title: heading.title,
      priority: parseScalar(block, "Priority") || "P0",
      parallelism: parseBullets(block, "Parallelism"),
      scope: parseBullets(block, "Scope"),
      non_goals: parseBullets(block, "Non-goals"),
      done_criteria: parseBullets(block, "Done"),
    };
  });
}

function taskNumber(taskId) {
  const match = taskId.match(/^REF\.(\d+)$/);
  return match ? Number(match[1]) : Number.MAX_SAFE_INTEGER;
}

function dependenciesFor(task, orderedTasks) {
  const previous = orderedTasks[orderedTasks.findIndex((item) => item.id === task.id) - 1];
  const overrides = new Map([
    ["REF.0", []],
    ["REF.5", ["REF.1"]],
    ["REF.6", ["REF.4", "REF.5"]],
  ]);

  if (overrides.has(task.id)) {
    return overrides.get(task.id);
  }
  return previous ? [previous.id] : [];
}

function impactForTask(taskId) {
  const impacts = {
    "REF.0": {
      user: "工作方向被鎖定，避免 AI/chat-first 舊路線繼續擴張。",
      code: "只影響 ADR 與任務文件，不改 runtime code。",
      risk: "低；主要風險是決策文字和實際任務不同步。",
    },
    "REF.1": {
      user: "所有結果列會逐步走向同一種 action/preview 模型。",
      code: "建立 Rust 與 TypeScript 的 UnifiedResult 契約，舊型別先保留相容。",
      risk: "中；schema 若過早定死，後續 REF.6 會卡住。",
    },
    "REF.2": {
      user: "畫面和鍵盤行為應維持原樣，但後續 UI 改版會更容易。",
      code: "CommandPalette 拆成 feature hooks/components，降低熱路徑檔案大小。",
      risk: "中；最容易回歸的是 launcher focus/IME 和刪除確認流程。",
    },
    "REF.3": {
      user: "使用者可見行為應不變，但 agent 相關功能的維護邊界會更清楚。",
      code: "agent handler 拆分 lifecycle、context、tool dispatch、dev runner。",
      risk: "中高；錯切邊界可能破壞既有 legacy agent fallback。",
    },
    "REF.4": {
      user: "AI 會變成結果列旁的 explain/summarize/fix_error 動作，而不是獨立聊天面板。",
      code: "新增 stateless capability layer、IPC handler、前端 inline hooks/surfaces。",
      risk: "高；需要守住 approval、audit、latency 和 provider 相容性。",
    },
    "REF.5": {
      user: "Keynova 會開始記住近期工作流並提供非 AI 的下一步建議。",
      code: "新增 workflow_history schema 與 workflow_memory core。",
      risk: "中高；資料遷移和隱私邊界要非常保守。",
    },
    "REF.6": {
      user: "搜尋框會更像純 dispatcher，結果列 action chip 成為主要互動方式。",
      code: "CommandPalette 改吃 UnifiedResult，移除 AiPanel/TerminalPanel 熱路徑掛載。",
      risk: "高；這是最接近產品手感的切換點。",
    },
    "REF.7": {
      user: "新 AI 互動模型成為預設，legacy agent 只作為 fallback。",
      code: "加 performance gates，ai.legacy_agent 預設 false，觀察一個 release cycle。",
      risk: "中；如果量化指標不足，不能急著刪 legacy。",
    },
    "REF.8": {
      user: "舊 chat-first AI 面板與 ReAct runtime 退出主產品路線。",
      code: "刪除或大幅縮小 AiPanel、agent_runtime、legacy handler code。",
      risk: "高；必須確認 audit history 和 fallback 需求已處理。",
    },
  };

  return impacts[taskId] || {
    user: "待 AI 依任務內容補上使用者可見影響。",
    code: "待 AI 依任務內容補上程式結構影響。",
    risk: "待評估。",
  };
}

function defaultSuggestions() {
  return {
    schema_version: 1,
    source: "ai_editable",
    updated_by: "AI",
    guidance:
      "AI 有新的流程、排版、ADR 或風險建議時，先加到這裡；docs:workbench-sync 會自動渲染到 HTML，使用者勾選後再回寫 Markdown。",
    questions: [
      {
        id: "ref2_layout_direction",
        title: "REF.2 期間要偏向哪種 Command Palette 排版？",
        help: "這題主要決定 UI mock 的採用方向；正式實作仍以 REF.2 不改行為為前提。",
        type: "single",
        default: "compact-list",
        options: [
          {
            id: "compact-list",
            label: "緊湊清單",
            impact: "最保守，較不容易打斷 REF.2 拆分與 Bug A/B regression。",
          },
          {
            id: "split-preview",
            label: "左右預覽",
            impact: "較接近 REF.6 的目標，但建議先停留在 mock，不急著進 runtime。",
          },
          {
            id: "action-table",
            label: "動作表格",
            impact: "資訊量最高，適合後續 UnifiedResult 穩定後再評估。",
          },
        ],
      },
      {
        id: "ref2_split_priority",
        title: "REF.2 拆分時要優先保護哪些路徑？",
        help: "可以多選；AI 會依勾選結果安排檢查順序與測試重點。",
        type: "multiple",
        default: ["focus_ime", "delete_verify"],
        options: [
          {
            id: "focus_ime",
            label: "Launcher focus / IME",
            impact: "保護 Bug A 類回歸，避免視窗焦點、輸入法、Escape 行為被拆分影響。",
          },
          {
            id: "delete_verify",
            label: "Delete verification",
            impact: "保護 Bug B 類回歸，避免最近刪除、確認流程或狀態重置被拆壞。",
          },
          {
            id: "keyboard_nav",
            label: "鍵盤導覽",
            impact: "確保上下選取、Enter、Escape、secondary menu 不因 hook 拆分漂移。",
          },
          {
            id: "visual_baseline",
            label: "視覺 baseline",
            impact: "要求拆分後畫面密度與狀態呈現維持目前使用感。",
          },
        ],
      },
      {
        id: "ai_suggestion_policy",
        title: "AI 建議要以什麼方式進入正式流程？",
        help: "這題決定 AI 後續看到 proposal 時的預設處理方式。",
        type: "multiple",
        default: ["preview_first", "markdown_after_confirm"],
        options: [
          {
            id: "preview_first",
            label: "先更新 HTML 預覽",
            impact: "AI 先把想法放進 workbench，讓你勾選確認後再動正式文件。",
          },
          {
            id: "markdown_after_confirm",
            label: "確認後才更新 Markdown",
            impact: "保持 Markdown 權威，避免 AI 建議直接污染 active/current。",
          },
          {
            id: "session_log_trace",
            label: "保留 session log 軌跡",
            impact: "重要決策脈絡進 session log，不塞進 current.md 或 active.md。",
          },
        ],
      },
    ],
    suggestion_cards: [
      {
        id: "ref2-first-pass",
        title: "建議先用保守 UI mock 配合 REF.2",
        applies_to: ["REF.2"],
        body: "REF.2 的核心價值是降低 CommandPalette 複雜度，不是改 UI。建議先用緊湊清單做 baseline，再把左右預覽與動作表格留作 REF.6 評估。",
        recommended_options: ["compact-list", "focus_ime", "delete_verify", "preview_first"],
      },
      {
        id: "proposal-loop",
        title: "建議採用 proposal loop",
        applies_to: ["workflow"],
        body: "AI 的新想法先進 workbench-suggestions.json，HTML 自動更新；你勾選後複製 proposal 回對話，AI 再驗證並更新最小 Markdown。",
        recommended_options: ["preview_first", "markdown_after_confirm", "session_log_trace"],
      },
    ],
  };
}

function loadSuggestions() {
  const fullPath = path.join(ROOT, SUGGESTIONS_SOURCE);
  if (!fs.existsSync(fullPath)) {
    writeIfChanged(SUGGESTIONS_SOURCE, `${JSON.stringify(defaultSuggestions(), null, 2)}\n`);
  }

  try {
    return JSON.parse(fs.readFileSync(fullPath, "utf8"));
  } catch (error) {
    console.warn(`[docs-workbench-sync] Unable to parse ${SUGGESTIONS_SOURCE}; using defaults.`);
    console.warn(String(error));
    return defaultSuggestions();
  }
}

function buildState() {
  const active = parseActiveTasks(readMarkdown(ACTIVE_TASKS));
  const suggestions = loadSuggestions();
  const planTasks = parsePlanTasks(readMarkdown(REFACTOR_PLAN)).sort(
    (a, b) => taskNumber(a.id) - taskNumber(b.id),
  );

  const tasks = planTasks.map((task) => {
    const activeTask = active.get(task.id);
    return {
      id: task.id,
      title: task.title,
      priority: activeTask?.priority || task.priority,
      status: activeTask?.status || "planned",
      summary: activeTask?.summary || task.title,
      depends_on: dependenciesFor(task, planTasks),
      parallel_with: task.id === "REF.5" ? ["REF.4"] : [],
      source: REFACTOR_PLAN,
      scope: task.scope,
      non_goals: task.non_goals,
      done_criteria: task.done_criteria,
      notes: task.parallelism,
      impact: impactForTask(task.id),
    };
  });

  const current = tasks.find((task) => task.status !== "done");
  const doneCount = tasks.filter((task) => task.status === "done").length;

  return {
    schema_version: 1,
    generated_by: "scripts/docs-workbench-sync.mjs",
    authority: {
      markdown_sources: [ACTIVE_TASKS, REFACTOR_PLAN],
      suggestion_source: SUGGESTIONS_SOURCE,
      generated_outputs: [STATE_TARGET, SUMMARY_TARGET, WORKBENCH_TARGET],
      conflict_rule: "markdown_wins",
      lifecycle: "shadow_state",
    },
    workflow: {
      name: "P0 Refactor: AI Capability Layer And Unified Search",
      zh_name: "P0 重構：AI 能力層與統一搜尋主線",
      current_task: current?.id || null,
      progress: {
        done: doneCount,
        total: tasks.length,
        percent: tasks.length === 0 ? 0 : Math.round((doneCount / tasks.length) * 100),
      },
      rules: [
        "Markdown remains the authority.",
        "Workbench files are generated convenience views.",
        "REF.4 and REF.5 may overlap after schema boundaries are clear.",
      ],
      expected_outcomes: [
        {
          title: "產品體感",
          body: "Keynova 會更像鍵盤優先的工作流 launcher；搜尋結果列負責呈現可執行動作，AI 變成 inline capability，而不是獨立聊天產品。",
        },
        {
          title: "UI 結構",
          body: "Command Palette 熱路徑會變薄，結果列、動作 chips、preview 與確認流程會逐步分層，讓後續 UI 模擬可以先在 workbench 討論。",
        },
        {
          title: "架構邊界",
          body: "UnifiedResult 會成為搜尋、builtin command、file action、AI capability 的共同契約；legacy agent/chat 先保留 fallback，最後在 REF.8 移除。",
        },
        {
          title: "風險控制",
          body: "新流程不應擴大 shell、檔案或網路邊界；高風險操作仍由 backend risk tag 與 UI confirmation 管住。",
        },
      ],
      ui_layout_options: [
        {
          id: "compact-list",
          label: "緊湊清單",
          description: "保留目前 launcher 的高速掃描感，結果列最密，preview 只在需要時展開。",
          best_for: "REF.2 拆分期間，最不容易造成視覺回歸。",
          tradeoff: "可解釋性較弱，AI action 的狀態需要靠 chip 或狀態列補足。",
        },
        {
          id: "split-preview",
          label: "左右預覽",
          description: "左側結果清單，右側顯示 preview、AI 摘要或確認內容。",
          best_for: "REF.6 之後，UnifiedResult 與 PreviewPayload 穩定時。",
          tradeoff: "需要仔細控制寬度與鍵盤焦點，避免 launcher 變成重型面板。",
        },
        {
          id: "action-table",
          label: "動作表格",
          description: "每列直接顯示主要 action chips、風險提示與來源 metadata。",
          best_for: "需要比較多個結果和多個可執行動作的 workflow。",
          tradeoff: "資訊密度最高，但若視覺層級沒有壓好會顯得吵。",
        },
      ],
    },
    suggestions,
    tasks,
  };
}

function buildSummary(state) {
  const questions = state.suggestions.questions.map((question) => ({
    id: question.id,
    title: question.title,
    type: question.type,
    default: question.default,
    options: question.options.map((option) => ({
      id: option.id,
      label: option.label,
      impact: option.impact,
    })),
  }));

  return {
    schema_version: state.schema_version,
    generated_by: state.generated_by,
    authority: {
      markdown_sources: state.authority.markdown_sources,
      suggestion_source: state.authority.suggestion_source,
      conflict_rule: state.authority.conflict_rule,
      lifecycle: state.authority.lifecycle,
      full_state: STATE_TARGET,
      workbench: WORKBENCH_TARGET,
    },
    workflow: {
      name: state.workflow.name,
      zh_name: state.workflow.zh_name,
      current_task: state.workflow.current_task,
      progress: state.workflow.progress,
      rules: state.workflow.rules,
      expected_outcomes: state.workflow.expected_outcomes.map((outcome) => ({
        title: outcome.title,
        body: outcome.body,
      })),
      ui_layout_options: state.workflow.ui_layout_options.map((option) => ({
        id: option.id,
        label: option.label,
        best_for: option.best_for,
        tradeoff: option.tradeoff,
      })),
    },
    suggestion_questions: questions,
    suggestion_cards: state.suggestions.suggestion_cards.map((card) => ({
      id: card.id,
      title: card.title,
      applies_to: card.applies_to,
      recommended_options: card.recommended_options,
    })),
    tasks: state.tasks.map((task) => ({
      id: task.id,
      title: task.title,
      priority: task.priority,
      status: task.status,
      summary: task.summary,
      depends_on: task.depends_on,
      parallel_with: task.parallel_with,
      impact: task.impact,
    })),
  };
}

function jsonForScript(value) {
  return JSON.stringify(value, null, 2)
    .replace(/&/g, "\\u0026")
    .replace(/</g, "\\u003c")
    .replace(/>/g, "\\u003e")
    .replace(/\u2028/g, "\\u2028")
    .replace(/\u2029/g, "\\u2029");
}

function renderTasksHtml(state) {
  const data = jsonForScript(state);
  return `<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>Keynova Task Workbench</title>
  <style>
    :root {
      color-scheme: light;
      --bg: #f7f8fa;
      --panel: #ffffff;
      --text: #172033;
      --muted: #5f6b7a;
      --line: #d9dee7;
      --accent: #1264a3;
      --done: #1f7a4d;
      --open: #9a5b00;
      --blocked: #a23b3b;
    }
    * { box-sizing: border-box; }
    body {
      margin: 0;
      background: var(--bg);
      color: var(--text);
      font: 14px/1.5 system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
    }
    header {
      border-bottom: 1px solid var(--line);
      background: var(--panel);
      padding: 20px clamp(18px, 4vw, 44px);
    }
    h1 {
      margin: 0 0 8px;
      font-size: 24px;
      letter-spacing: 0;
    }
    main {
      display: grid;
      grid-template-columns: minmax(0, 1fr) 360px;
      gap: 18px;
      padding: 18px clamp(18px, 4vw, 44px) 40px;
    }
    .summary {
      display: flex;
      flex-wrap: wrap;
      gap: 10px;
      color: var(--muted);
    }
    .pill {
      border: 1px solid var(--line);
      border-radius: 999px;
      padding: 4px 10px;
      background: #fbfcfe;
    }
    .toolbar {
      display: flex;
      flex-wrap: wrap;
      align-items: center;
      gap: 8px;
      margin-bottom: 12px;
    }
    button, select {
      border: 1px solid var(--line);
      border-radius: 6px;
      background: var(--panel);
      color: var(--text);
      min-height: 34px;
      padding: 6px 10px;
      font: inherit;
    }
    button:hover, select:hover { border-color: var(--accent); }
    .flow {
      display: grid;
      gap: 10px;
    }
    .task {
      border: 1px solid var(--line);
      border-left: 4px solid var(--accent);
      border-radius: 8px;
      background: var(--panel);
      padding: 12px;
      cursor: grab;
    }
    .task[data-status="done"] { border-left-color: var(--done); }
    .task[data-state="blocked"] { border-left-color: var(--blocked); }
    .task.dragging { opacity: 0.5; }
    .task-head {
      display: flex;
      justify-content: space-between;
      gap: 10px;
      align-items: flex-start;
    }
    .task-actions {
      display: flex;
      flex-wrap: wrap;
      gap: 6px;
      justify-content: flex-end;
      align-items: center;
    }
    .task-actions button {
      min-height: 28px;
      padding: 3px 8px;
      font-size: 12px;
    }
    .drag-handle {
      border: 1px solid var(--line);
      border-radius: 6px;
      background: #eef2f6;
      color: var(--muted);
      cursor: grab;
      padding: 4px 8px;
      user-select: none;
      font-size: 12px;
    }
    .drag-handle:active { cursor: grabbing; }
    .task-title {
      margin: 0;
      font-size: 15px;
      letter-spacing: 0;
    }
    .status {
      flex: none;
      color: var(--muted);
      font-size: 12px;
      text-transform: uppercase;
    }
    .meta {
      margin: 8px 0 0;
      color: var(--muted);
      font-size: 13px;
    }
    details {
      margin-top: 10px;
      border-top: 1px solid var(--line);
      padding-top: 8px;
    }
    summary { color: var(--accent); cursor: pointer; }
    ul { margin: 8px 0 0 18px; padding: 0; }
    aside {
      border: 1px solid var(--line);
      border-radius: 8px;
      background: var(--panel);
      padding: 14px;
      height: max-content;
      position: sticky;
      top: 14px;
    }
    aside h2 {
      margin: 0 0 10px;
      font-size: 16px;
      letter-spacing: 0;
    }
    textarea {
      width: 100%;
      min-height: 220px;
      resize: vertical;
      border: 1px solid var(--line);
      border-radius: 6px;
      padding: 10px;
      font: 12px/1.5 ui-monospace, SFMono-Regular, Consolas, monospace;
      color: var(--text);
      background: #fbfcfe;
    }
    .note {
      color: var(--muted);
      font-size: 13px;
      margin: 0 0 10px;
    }
    @media (max-width: 900px) {
      main { grid-template-columns: 1fr; }
      aside { position: static; }
    }
  </style>
</head>
<body>
  <header>
    <h1>Keynova Task Workbench</h1>
    <div class="summary">
      <span class="pill" id="current"></span>
      <span class="pill" id="progress"></span>
      <span class="pill">Authority: markdown</span>
      <span class="pill">Generated by docs:workbench-sync</span>
    </div>
  </header>
  <main>
    <section>
      <div class="toolbar">
        <select id="filter" aria-label="Filter tasks">
          <option value="all">All tasks</option>
          <option value="open">Open tasks</option>
          <option value="done">Done tasks</option>
          <option value="blocked">Blocked by dependencies</option>
        </select>
        <button type="button" id="reset">Reset order</button>
        <button type="button" id="copy">Copy proposed order</button>
      </div>
      <div class="flow" id="flow" aria-live="polite"></div>
    </section>
    <aside>
      <h2>Proposed Order</h2>
      <p class="note">Drag task cards to explore sequencing. Markdown remains the source of truth until you apply the order manually.</p>
      <textarea id="order" spellcheck="false"></textarea>
    </aside>
  </main>
  <script type="application/json" id="tasks-data">${data}</script>
  <script>
    const state = JSON.parse(document.getElementById("tasks-data").textContent);
    const flow = document.getElementById("flow");
    const order = document.getElementById("order");
    const filter = document.getElementById("filter");
    const current = document.getElementById("current");
    const progress = document.getElementById("progress");
    let orderedTasks = [...state.tasks];

    current.textContent = "Current: " + (state.workflow.current_task || "complete");
    progress.textContent = "Progress: " + state.workflow.progress.done + "/" + state.workflow.progress.total + " (" + state.workflow.progress.percent + "%)";

    function completedIds() {
      return new Set(state.tasks.filter((task) => task.status === "done").map((task) => task.id));
    }

    function orderFromIds(ids) {
      const byId = new Map(state.tasks.map((task) => [task.id, task]));
      const ordered = ids.map((id) => byId.get(id)).filter(Boolean);
      const seen = new Set(ordered.map((task) => task.id));
      return ordered.concat(state.tasks.filter((task) => !seen.has(task.id)));
    }

    function moveTask(taskId, direction) {
      const index = orderedTasks.findIndex((task) => task.id === taskId);
      const nextIndex = direction === "up" ? index - 1 : index + 1;
      if (index < 0 || nextIndex < 0 || nextIndex >= orderedTasks.length) return;
      const next = [...orderedTasks];
      [next[index], next[nextIndex]] = [next[nextIndex], next[index]];
      orderedTasks = next;
      selectedTaskId = taskId;
      renderTasks();
      renderImpact();
    }

    function currentDecisionPayload() {
      const selectedTask = state.tasks.find((task) => task.id === selectedTaskId);
      const baselineOrder = state.tasks.map((task) => task.id);
      const proposedOrder = orderedTasks.map((task) => task.id);
      const orderDiff = proposedOrder
        .map((id, to) => ({ id, from: baselineOrder.indexOf(id), to }))
        .filter((item) => item.from !== item.to);
      return {
        authority: "proposal_only",
        source: "docs/workbench/tasks.html",
        decision_status: confirmedDecision ? "confirmed" : "unconfirmed_do_not_execute",
        apply_instruction: confirmedDecision
          ? "使用者已按下確認目前決策。請依 Markdown 權威來源檢查這份 proposal；若合理，再更新最小必要的 Markdown 文件或開始使用者要求的實作。"
          : "使用者尚未按下確認目前決策。不要實作 runtime 變更，也不要改權威 Markdown；只能更新建議池或預覽，並等待使用者確認後貼回 proposal。",
        selected_task: selectedTaskId,
        selected_task_impact: selectedTask ? selectedTask.impact : null,
        proposed_order: proposedOrder,
        order_changed: orderDiff.length > 0,
        order_diff: orderDiff,
        confirmed_answers: selectedAnswers,
        suggestion_source: state.authority.suggestion_source,
        ui_layout_choice: selectedLayout,
        ui_toggles: {
          preview: toggles.preview.checked,
          action_chips: toggles.chips.checked,
          status_row: toggles.status.checked,
          dense_rows: toggles.dense.checked,
        },
      };
    }

    function renderDecisionState() {
      if (!confirmedDecision) {
        decisionState.textContent = "尚未確認本頁決策。";
        return;
      }
      decisionState.textContent = [
        "已確認：" + confirmedDecision.confirmed_at,
        "任務：" + confirmedDecision.selected_task,
        "排版：" + confirmedDecision.ui_layout_choice,
        "排序：" + confirmedDecision.proposed_order.join(" > "),
      ].join("\\n");
    }

    function saveDecision() {
      confirmedDecision = {
        ...currentDecisionPayload(),
        confirmed_at: new Date().toISOString(),
      };
      localStorage.setItem(STORAGE_KEY, JSON.stringify(confirmedDecision));
      renderDecisionState();
      updateProposal();
    }

    function loadDecision() {
      const raw = localStorage.getItem(STORAGE_KEY);
      if (!raw) return;
      try {
        const saved = JSON.parse(raw);
        confirmedDecision = saved;
        if (Array.isArray(saved.proposed_order)) {
          orderedTasks = orderFromIds(saved.proposed_order);
        }
        if (saved.selected_task) {
          selectedTaskId = saved.selected_task;
        }
        if (saved.ui_layout_choice) {
          selectedLayout = saved.ui_layout_choice;
        }
        if (saved.confirmed_answers) {
          Object.assign(selectedAnswers, saved.confirmed_answers);
        }
        if (saved.ui_toggles) {
          toggles.preview.checked = saved.ui_toggles.preview !== false;
          toggles.chips.checked = saved.ui_toggles.action_chips !== false;
          toggles.status.checked = saved.ui_toggles.status_row !== false;
          toggles.dense.checked = Boolean(saved.ui_toggles.dense_rows);
        }
      } catch {
        localStorage.removeItem(STORAGE_KEY);
      }
    }

    function dependencyState(task) {
      const done = completedIds();
      const missing = task.depends_on.filter((id) => !done.has(id));
      return missing.length > 0 && task.status !== "done" ? "blocked" : "ready";
    }

    function escapeText(value) {
      return String(value)
        .replace(/&/g, "&amp;")
        .replace(/</g, "&lt;")
        .replace(/>/g, "&gt;")
        .replace(/"/g, "&quot;")
        .replace(/'/g, "&#039;");
    }

    function inputValueSelector(value) {
      return 'input[value="' + String(value).replace(/\\\\/g, "\\\\\\\\").replace(/"/g, '\\\\"') + '"]';
    }

    function renderList(items, label) {
      if (!items.length) return "";
      return "<strong>" + escapeText(label) + "</strong><ul>" + items.map((item) => "<li>" + escapeText(item) + "</li>").join("") + "</ul>";
    }

    function render() {
      const filterValue = filter.value;
      flow.innerHTML = "";
      for (const task of orderedTasks) {
        const depState = dependencyState(task);
        if (filterValue === "open" && task.status === "done") continue;
        if (filterValue === "done" && task.status !== "done") continue;
        if (filterValue === "blocked" && depState !== "blocked") continue;

        const card = document.createElement("article");
        card.className = "task";
        card.draggable = true;
        card.dataset.id = task.id;
        card.dataset.status = task.status;
        card.dataset.state = depState;
        const title = escapeText(task.id + " - " + task.title);
        const summary = escapeText(task.summary);
        const status = escapeText(task.status + (depState === "blocked" ? " / blocked" : ""));
        const depends = escapeText(task.depends_on.join(", ") || "none");
        const parallel = task.parallel_with.length ? " | Parallel with: " + escapeText(task.parallel_with.join(", ")) : "";
        card.innerHTML = [
          '<div class="task-head">',
          '<h3 class="task-title">' + title + "</h3>",
          '<span class="status">' + status + "</span>",
          "</div>",
          '<p class="meta">' + summary + "</p>",
          '<p class="meta">Depends on: ' + depends + parallel + "</p>",
          "<details><summary>Details</summary>",
          renderList(task.scope, "Scope"),
          renderList(task.non_goals, "Non-goals"),
          renderList(task.done_criteria, "Done"),
          "</details>",
        ].join("");
        flow.appendChild(card);
      }
      updateOrder();
    }

    function updateOrder() {
      order.value = JSON.stringify({
        authority: "proposal_only",
        source: "docs/workbench/tasks.html",
        order: orderedTasks.map((task) => task.id),
      }, null, 2);
    }

    flow.addEventListener("dragstart", (event) => {
      const card = event.target.closest(".task");
      if (!card) return;
      card.classList.add("dragging");
      event.dataTransfer.setData("text/plain", card.dataset.id);
    });

    flow.addEventListener("dragend", (event) => {
      const card = event.target.closest(".task");
      if (card) card.classList.remove("dragging");
    });

    flow.addEventListener("dragover", (event) => {
      event.preventDefault();
      const dragging = flow.querySelector(".dragging");
      const target = event.target.closest(".task");
      if (!dragging || !target || dragging === target) return;
      const rect = target.getBoundingClientRect();
      const after = event.clientY > rect.top + rect.height / 2;
      flow.insertBefore(dragging, after ? target.nextSibling : target);
    });

    flow.addEventListener("drop", () => {
      const ids = [...flow.querySelectorAll(".task")].map((item) => item.dataset.id);
      const hidden = orderedTasks.filter((task) => !ids.includes(task.id));
      orderedTasks = ids.map((id) => orderedTasks.find((task) => task.id === id)).concat(hidden);
      render();
    });

    filter.addEventListener("change", render);
    document.getElementById("reset").addEventListener("click", () => {
      orderedTasks = [...state.tasks];
      filter.value = "all";
      render();
    });
    document.getElementById("copy").addEventListener("click", async () => {
      await navigator.clipboard.writeText(order.value);
    });

    render();
  </script>
</body>
</html>
`;
}

function renderDecisionWorkbenchHtml(state) {
  const data = jsonForScript(state);
  return `<!doctype html>
<html lang="zh-Hant">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>Keynova 決策工作台</title>
  <style>
    :root {
      color-scheme: light;
      --bg: #f6f7f9;
      --panel: #ffffff;
      --text: #172033;
      --muted: #647084;
      --line: #d9dee7;
      --accent: #1264a3;
      --accent-soft: #e8f2fb;
      --done: #1f7a4d;
      --open: #9a5b00;
      --blocked: #a23b3b;
      --shadow: 0 10px 28px rgba(23, 32, 51, 0.08);
    }
    * { box-sizing: border-box; }
    body {
      margin: 0;
      background: var(--bg);
      color: var(--text);
      font: 14px/1.55 system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
    }
    header {
      background: var(--panel);
      border-bottom: 1px solid var(--line);
      padding: 22px clamp(18px, 4vw, 46px);
    }
    h1, h2, h3 { letter-spacing: 0; }
    h1 { margin: 0 0 8px; font-size: 26px; }
    h2 { margin: 0 0 12px; font-size: 18px; }
    h3 { margin: 0; font-size: 15px; }
    p { margin: 0; }
    main {
      display: grid;
      grid-template-columns: minmax(0, 1fr) 380px;
      gap: 18px;
      padding: 18px clamp(18px, 4vw, 46px) 40px;
    }
    .stack { display: grid; gap: 16px; }
    .panel {
      background: var(--panel);
      border: 1px solid var(--line);
      border-radius: 8px;
      padding: 16px;
      box-shadow: var(--shadow);
    }
    .summary {
      display: flex;
      flex-wrap: wrap;
      gap: 8px;
      color: var(--muted);
    }
    .pill {
      border: 1px solid var(--line);
      border-radius: 999px;
      padding: 4px 10px;
      background: #fbfcfe;
      white-space: nowrap;
    }
    .outcome-grid {
      display: grid;
      grid-template-columns: repeat(4, minmax(0, 1fr));
      gap: 10px;
    }
    .outcome {
      border: 1px solid var(--line);
      border-radius: 8px;
      padding: 12px;
      background: #fbfcfe;
      min-height: 132px;
    }
    .outcome strong { display: block; margin-bottom: 6px; }
    .outcome span { color: var(--muted); }
    .layout-grid {
      display: grid;
      grid-template-columns: 310px minmax(0, 1fr);
      gap: 14px;
      align-items: start;
    }
    .choice-list { display: grid; gap: 8px; }
    .choice {
      display: grid;
      gap: 4px;
      border: 1px solid var(--line);
      border-radius: 8px;
      padding: 10px;
      background: #fbfcfe;
      cursor: pointer;
    }
    .choice:has(input:checked) {
      border-color: var(--accent);
      background: var(--accent-soft);
    }
    .choice input { margin-right: 6px; }
    .choice small { color: var(--muted); }
    .toggles {
      display: grid;
      grid-template-columns: repeat(2, minmax(0, 1fr));
      gap: 8px;
      margin-top: 10px;
    }
    .toggles label {
      border: 1px solid var(--line);
      border-radius: 8px;
      padding: 8px;
      background: #fbfcfe;
    }
    .suggestion-grid {
      display: grid;
      grid-template-columns: repeat(2, minmax(0, 1fr));
      gap: 10px;
    }
    .suggestion-card,
    .question-card {
      border: 1px solid var(--line);
      border-radius: 8px;
      background: #fbfcfe;
      padding: 12px;
    }
    .suggestion-card strong,
    .question-card strong {
      display: block;
      margin-bottom: 6px;
    }
    .question-grid {
      display: grid;
      gap: 10px;
    }
    .option-list {
      display: grid;
      gap: 7px;
      margin-top: 8px;
    }
    .option-row {
      display: grid;
      grid-template-columns: auto minmax(0, 1fr);
      gap: 8px;
      border: 1px solid var(--line);
      border-radius: 8px;
      padding: 8px;
      background: var(--panel);
      cursor: pointer;
    }
    .option-row:has(input:checked) {
      border-color: var(--accent);
      background: var(--accent-soft);
    }
    .option-row small {
      display: block;
      color: var(--muted);
      margin-top: 2px;
    }
    .mock {
      border: 1px solid #202a3a;
      border-radius: 10px;
      background: #111827;
      color: #e5e7eb;
      overflow: hidden;
    }
    .mock-bar {
      border-bottom: 1px solid #2b3546;
      padding: 10px 12px;
      color: #9ca3af;
    }
    .mock-body {
      display: grid;
      grid-template-columns: var(--mock-grid, 1fr 280px);
      min-height: 260px;
    }
    .mock-results {
      padding: 10px;
      display: grid;
      gap: 8px;
      align-content: start;
    }
    .mock-row {
      border: 1px solid #374151;
      border-radius: 8px;
      padding: 9px;
      background: #1f2937;
    }
    .mock-row.active { border-color: #60a5fa; }
    .mock-title { display: flex; justify-content: space-between; gap: 8px; }
    .chips { display: flex; flex-wrap: wrap; gap: 5px; margin-top: 7px; }
    .chip {
      border: 1px solid #4b5563;
      border-radius: 999px;
      padding: 2px 7px;
      color: #bfdbfe;
      font-size: 12px;
    }
    .mock-preview {
      border-left: 1px solid #2b3546;
      padding: 12px;
      background: #0f172a;
      color: #cbd5e1;
    }
    .status-row {
      border-top: 1px solid #2b3546;
      padding: 8px 12px;
      color: #9ca3af;
      font-size: 12px;
    }
    .toolbar {
      display: flex;
      flex-wrap: wrap;
      gap: 8px;
      margin-bottom: 10px;
    }
    button, select {
      min-height: 34px;
      border: 1px solid var(--line);
      border-radius: 6px;
      background: var(--panel);
      color: var(--text);
      padding: 6px 10px;
      font: inherit;
    }
    button:hover, select:hover { border-color: var(--accent); }
    .flow { display: grid; gap: 10px; }
    .task {
      border: 1px solid var(--line);
      border-left: 4px solid var(--accent);
      border-radius: 8px;
      background: var(--panel);
      padding: 12px;
      cursor: grab;
    }
    .task[data-status="done"] { border-left-color: var(--done); }
    .task[data-state="blocked"] { border-left-color: var(--blocked); }
    .task.dragging { opacity: 0.5; }
    .task-head {
      display: flex;
      justify-content: space-between;
      gap: 10px;
      align-items: flex-start;
    }
    .meta { margin-top: 7px; color: var(--muted); font-size: 13px; }
    .tag {
      flex: none;
      color: var(--muted);
      font-size: 12px;
      text-transform: uppercase;
    }
    details {
      margin-top: 10px;
      border-top: 1px solid var(--line);
      padding-top: 8px;
    }
    summary { color: var(--accent); cursor: pointer; }
    ul { margin: 8px 0 0 18px; padding: 0; }
    aside {
      position: sticky;
      top: 14px;
      align-self: start;
      display: grid;
      gap: 12px;
    }
    .decision-actions {
      display: grid;
      grid-template-columns: repeat(2, minmax(0, 1fr));
      gap: 8px;
      margin-top: 10px;
    }
    .decision-state {
      border: 1px solid var(--line);
      border-radius: 8px;
      background: #fbfcfe;
      padding: 10px;
      color: var(--muted);
      font-size: 13px;
      white-space: pre-wrap;
    }
    textarea {
      width: 100%;
      min-height: 300px;
      resize: vertical;
      border: 1px solid var(--line);
      border-radius: 8px;
      padding: 10px;
      font: 12px/1.5 ui-monospace, SFMono-Regular, Consolas, monospace;
      color: var(--text);
      background: #fbfcfe;
    }
    .note { color: var(--muted); font-size: 13px; }
    @media (max-width: 1100px) {
      main { grid-template-columns: 1fr; }
      aside { position: static; }
      .outcome-grid { grid-template-columns: repeat(2, minmax(0, 1fr)); }
    }
    @media (max-width: 760px) {
      .layout-grid { grid-template-columns: 1fr; }
      .outcome-grid, .toggles, .suggestion-grid { grid-template-columns: 1fr; }
      .mock-body { grid-template-columns: 1fr; }
      .mock-preview { border-left: 0; border-top: 1px solid #2b3546; }
    }
  </style>
</head>
<body>
  <header>
    <h1>Keynova 決策工作台</h1>
    <div class="summary">
      <span class="pill" id="current"></span>
      <span class="pill" id="progress"></span>
      <span class="pill">權威來源：Markdown</span>
      <span class="pill">此頁是 proposal，不會直接改任務</span>
    </div>
  </header>
  <main>
    <div class="stack">
      <section class="panel">
        <h2>完成後的預期成果</h2>
        <div class="outcome-grid" id="outcomes"></div>
      </section>
      <section class="panel">
        <h2>UI 排版方案模擬</h2>
        <div class="layout-grid">
          <div>
            <div class="choice-list" id="layoutChoices"></div>
            <div class="toggles">
              <label><input type="checkbox" id="togglePreview" checked> 顯示預覽區</label>
              <label><input type="checkbox" id="toggleChips" checked> 顯示 action chips</label>
              <label><input type="checkbox" id="toggleStatus" checked> 顯示狀態列</label>
              <label><input type="checkbox" id="toggleDense"> 壓縮列高</label>
            </div>
          </div>
          <div class="mock" aria-label="Command Palette mock">
            <div class="mock-bar">⌘ Keynova / 搜尋、執行、AI inline action</div>
            <div class="mock-body" id="mockBody">
              <div class="mock-results" id="mockResults"></div>
              <div class="mock-preview" id="mockPreview"></div>
            </div>
            <div class="status-row" id="mockStatus">Search first chunk P50 目標：80ms 以內</div>
          </div>
        </div>
      </section>
      <section class="panel">
        <h2>AI 建議與手動確認</h2>
        <p class="note">AI 後續有新建議會先放進建議池；你可以在這裡用勾選確認，多題答案會一起進右側 proposal。</p>
        <div class="suggestion-grid" id="suggestionCards"></div>
        <div class="question-grid" id="questionCards"></div>
      </section>
      <section class="panel">
        <h2>任務積木排序</h2>
        <div class="toolbar">
          <select id="filter" aria-label="篩選任務">
            <option value="all">全部任務</option>
            <option value="open">尚未完成</option>
            <option value="done">已完成</option>
            <option value="blocked">依賴未完成</option>
          </select>
          <button type="button" id="reset">重設排序</button>
          <button type="button" id="copy">複製 proposal</button>
        </div>
        <div class="flow" id="flow" aria-live="polite"></div>
      </section>
    </div>
    <aside>
      <section class="panel">
        <h2>目前選擇的影響</h2>
        <div id="impact" class="note"></div>
        <div class="decision-actions">
          <button type="button" id="confirmDecision">確認目前決策</button>
          <button type="button" id="clearDecision">清除暫存</button>
        </div>
        <div class="decision-state" id="decisionState">尚未確認本頁決策。</div>
      </section>
      <section class="panel">
        <h2>給 AI 的 proposal</h2>
        <p class="note">把這段貼回對話，AI 會依 Markdown 權威來源更新任務或產生更正式的 ADR/UI mock。</p>
        <textarea id="proposal" spellcheck="false"></textarea>
      </section>
    </aside>
  </main>
  <script type="application/json" id="workbench-data">${data}</script>
  <script>
    const state = JSON.parse(document.getElementById("workbench-data").textContent);
    const flow = document.getElementById("flow");
    const proposal = document.getElementById("proposal");
    const filter = document.getElementById("filter");
    const impact = document.getElementById("impact");
    const layoutChoices = document.getElementById("layoutChoices");
    const suggestionCards = document.getElementById("suggestionCards");
    const questionCards = document.getElementById("questionCards");
    const mockBody = document.getElementById("mockBody");
    const mockResults = document.getElementById("mockResults");
    const mockPreview = document.getElementById("mockPreview");
    const mockStatus = document.getElementById("mockStatus");
    const decisionState = document.getElementById("decisionState");
    const toggles = {
      preview: document.getElementById("togglePreview"),
      chips: document.getElementById("toggleChips"),
      status: document.getElementById("toggleStatus"),
      dense: document.getElementById("toggleDense"),
    };
    let orderedTasks = [...state.tasks];
    let selectedTaskId = state.workflow.current_task || state.tasks[0]?.id;
    let selectedLayout = "compact-list";
    const selectedAnswers = {};
    const STORAGE_KEY = "keynova-workbench-decision-v1";
    let confirmedDecision = null;

    document.getElementById("current").textContent = "目前任務：" + (state.workflow.current_task || "已完成");
    document.getElementById("progress").textContent = "進度：" + state.workflow.progress.done + "/" + state.workflow.progress.total + " (" + state.workflow.progress.percent + "%)";

    function escapeText(value) {
      return String(value)
        .replace(/&/g, "&amp;")
        .replace(/</g, "&lt;")
        .replace(/>/g, "&gt;")
        .replace(/"/g, "&quot;")
        .replace(/'/g, "&#039;");
    }

    function inputValueSelector(value) {
      return 'input[value="' + String(value).replace(/\\\\/g, "\\\\\\\\").replace(/"/g, '\\\\"') + '"]';
    }

    function completedIds() {
      return new Set(state.tasks.filter((task) => task.status === "done").map((task) => task.id));
    }

    function orderFromIds(ids) {
      const byId = new Map(state.tasks.map((task) => [task.id, task]));
      const ordered = ids.map((id) => byId.get(id)).filter(Boolean);
      const seen = new Set(ordered.map((task) => task.id));
      return ordered.concat(state.tasks.filter((task) => !seen.has(task.id)));
    }

    function moveTask(taskId, direction) {
      const index = orderedTasks.findIndex((task) => task.id === taskId);
      const nextIndex = direction === "up" ? index - 1 : index + 1;
      if (index < 0 || nextIndex < 0 || nextIndex >= orderedTasks.length) return;
      const next = [...orderedTasks];
      [next[index], next[nextIndex]] = [next[nextIndex], next[index]];
      orderedTasks = next;
      selectedTaskId = taskId;
      renderTasks();
      renderImpact();
    }

    function currentDecisionPayload() {
      const selectedTask = state.tasks.find((task) => task.id === selectedTaskId);
      const baselineOrder = state.tasks.map((task) => task.id);
      const proposedOrder = orderedTasks.map((task) => task.id);
      const orderDiff = proposedOrder
        .map((id, to) => ({ id, from: baselineOrder.indexOf(id), to }))
        .filter((item) => item.from !== item.to);
      return {
        authority: "proposal_only",
        source: "docs/workbench/tasks.html",
        decision_status: confirmedDecision ? "confirmed" : "unconfirmed_do_not_execute",
        apply_instruction: confirmedDecision
          ? "使用者已按下確認目前決策。請依 Markdown 權威來源檢查這份 proposal；若合理，再更新最小必要的 Markdown 文件或開始使用者要求的實作。"
          : "使用者尚未按下確認目前決策。不要實作 runtime 變更，也不要改權威 Markdown；只能更新建議池或預覽，並等待使用者確認後貼回 proposal。",
        selected_task: selectedTaskId,
        selected_task_impact: selectedTask ? selectedTask.impact : null,
        proposed_order: proposedOrder,
        order_changed: orderDiff.length > 0,
        order_diff: orderDiff,
        confirmed_answers: selectedAnswers,
        suggestion_source: state.authority.suggestion_source,
        ui_layout_choice: selectedLayout,
        ui_toggles: {
          preview: toggles.preview.checked,
          action_chips: toggles.chips.checked,
          status_row: toggles.status.checked,
          dense_rows: toggles.dense.checked,
        },
      };
    }

    function renderDecisionState() {
      if (!confirmedDecision) {
        decisionState.textContent = "尚未確認本頁決策。";
        return;
      }
      decisionState.textContent = [
        "已確認：" + confirmedDecision.confirmed_at,
        "任務：" + confirmedDecision.selected_task,
        "排版：" + confirmedDecision.ui_layout_choice,
        "排序：" + confirmedDecision.proposed_order.join(" > "),
      ].join("\\n");
    }

    function saveDecision() {
      confirmedDecision = {
        ...currentDecisionPayload(),
        confirmed_at: new Date().toISOString(),
      };
      localStorage.setItem(STORAGE_KEY, JSON.stringify(confirmedDecision));
      renderDecisionState();
      updateProposal();
    }

    function loadDecision() {
      const raw = localStorage.getItem(STORAGE_KEY);
      if (!raw) return;
      try {
        const saved = JSON.parse(raw);
        confirmedDecision = saved;
        if (Array.isArray(saved.proposed_order)) {
          orderedTasks = orderFromIds(saved.proposed_order);
        }
        if (saved.selected_task) {
          selectedTaskId = saved.selected_task;
        }
        if (saved.ui_layout_choice) {
          selectedLayout = saved.ui_layout_choice;
        }
        if (saved.confirmed_answers) {
          Object.assign(selectedAnswers, saved.confirmed_answers);
        }
        if (saved.ui_toggles) {
          toggles.preview.checked = saved.ui_toggles.preview !== false;
          toggles.chips.checked = saved.ui_toggles.action_chips !== false;
          toggles.status.checked = saved.ui_toggles.status_row !== false;
          toggles.dense.checked = Boolean(saved.ui_toggles.dense_rows);
        }
      } catch {
        localStorage.removeItem(STORAGE_KEY);
      }
    }

    function dependencyState(task) {
      const done = completedIds();
      const missing = task.depends_on.filter((id) => !done.has(id));
      return missing.length > 0 && task.status !== "done" ? "blocked" : "ready";
    }

    function renderOutcomes() {
      document.getElementById("outcomes").innerHTML = state.workflow.expected_outcomes.map((item) =>
        '<article class="outcome"><strong>' + escapeText(item.title) + '</strong><span>' + escapeText(item.body) + '</span></article>'
      ).join("");
    }

    function renderLayoutChoices() {
      layoutChoices.innerHTML = state.workflow.ui_layout_options.map((option, index) =>
        '<label class="choice"><span><input type="radio" name="layout" value="' + escapeText(option.id) + '"' + (option.id === selectedLayout ? " checked" : "") + '> ' + escapeText(option.label) + '</span><small>' + escapeText(option.description) + '</small><small>適合：' + escapeText(option.best_for) + '</small><small>取捨：' + escapeText(option.tradeoff) + '</small></label>'
      ).join("");
      layoutChoices.addEventListener("change", (event) => {
        if (event.target.name === "layout") {
          selectedLayout = event.target.value;
          selectedAnswers.ref2_layout_direction = selectedLayout;
          const questionInput = questionCards.querySelector(inputValueSelector(selectedLayout));
          if (questionInput && questionInput.type === "radio") questionInput.checked = true;
          renderMock();
          updateProposal();
        }
      });
    }

    function defaultAnswerFor(question) {
      if (question.default !== undefined) return question.default;
      return question.type === "multiple" ? [] : question.options[0]?.id || "";
    }

    function ensureAnswers() {
      for (const question of state.suggestions.questions || []) {
        if (selectedAnswers[question.id] === undefined) {
          selectedAnswers[question.id] = defaultAnswerFor(question);
        }
      }
    }

    function renderSuggestionCards() {
      const cards = state.suggestions.suggestion_cards || [];
      suggestionCards.innerHTML = cards.map((card) =>
        '<article class="suggestion-card"><strong>' + escapeText(card.title) + '</strong><p class="note">' + escapeText(card.body) + '</p><p class="meta">適用：' + escapeText((card.applies_to || []).join(", ") || "通用") + '</p><p class="meta">推薦選項：' + escapeText((card.recommended_options || []).join(", ") || "無") + '</p></article>'
      ).join("");
    }

    function renderQuestionCards() {
      ensureAnswers();
      questionCards.innerHTML = (state.suggestions.questions || []).map((question) => {
        const name = "q_" + question.id;
        const answers = selectedAnswers[question.id];
        const options = question.options.map((option) => {
          const checked = question.type === "multiple"
            ? Array.isArray(answers) && answers.includes(option.id)
            : answers === option.id;
          const inputType = question.type === "multiple" ? "checkbox" : "radio";
          return '<label class="option-row"><input type="' + inputType + '" name="' + escapeText(name) + '" value="' + escapeText(option.id) + '"' + (checked ? " checked" : "") + '><span><strong>' + escapeText(option.label) + '</strong><small>' + escapeText(option.impact) + '</small></span></label>';
        }).join("");
        return '<article class="question-card" data-question="' + escapeText(question.id) + '"><strong>' + escapeText(question.title) + '</strong><p class="note">' + escapeText(question.help || "") + '</p><div class="option-list">' + options + '</div></article>';
      }).join("");

      questionCards.addEventListener("change", (event) => {
        const card = event.target.closest(".question-card");
        if (!card) return;
        const question = (state.suggestions.questions || []).find((item) => item.id === card.dataset.question);
        if (!question) return;
        if (question.type === "multiple") {
          selectedAnswers[question.id] = [...card.querySelectorAll("input:checked")].map((input) => input.value);
        } else {
          selectedAnswers[question.id] = event.target.value;
          if (question.id === "ref2_layout_direction") {
            selectedLayout = event.target.value;
            const layoutInput = layoutChoices.querySelector(inputValueSelector(selectedLayout));
            if (layoutInput) layoutInput.checked = true;
            renderMock();
          }
        }
        updateProposal();
      });
    }

    function renderMock() {
      const showPreview = toggles.preview.checked && selectedLayout !== "compact-list";
      const showChips = toggles.chips.checked;
      const dense = toggles.dense.checked;
      mockBody.style.setProperty("--mock-grid", showPreview ? "minmax(0, 1fr) 280px" : "1fr");
      mockPreview.style.display = showPreview ? "block" : "none";
      mockStatus.style.display = toggles.status.checked ? "block" : "none";
      const rows = [
        { title: "src/components/CommandPalette.tsx", meta: "目前 REF.2 拆分目標", chips: ["開啟", "預覽", "拆分建議"] },
        { title: "Explain selected result", meta: "REF.4 inline capability", chips: ["explain", "summarize"] },
        { title: "Fix terminal error", meta: "需要 risk tag 判斷", chips: ["fix_error", "需確認"] },
      ];
      mockResults.innerHTML = rows.map((row, index) =>
        '<div class="mock-row' + (index === 0 ? " active" : "") + '" style="padding:' + (dense ? "6px" : "9px") + '"><div class="mock-title"><strong>' + escapeText(row.title) + '</strong><span>' + (index + 1) + '</span></div><div class="meta">' + escapeText(row.meta) + '</div>' + (showChips ? '<div class="chips">' + row.chips.map((chip) => '<span class="chip">' + escapeText(chip) + '</span>').join("") + '</div>' : '') + '</div>'
      ).join("");
      const layout = state.workflow.ui_layout_options.find((item) => item.id === selectedLayout);
      mockPreview.innerHTML = '<h3>' + escapeText(layout.label) + '</h3><p class="meta">' + escapeText(layout.description) + '</p><p class="meta">完成後影響：結果列 action / preview / confirmation 的分工會更清楚，但正式採用仍需回寫 Markdown 任務或 ADR。</p>';
    }

    function renderList(items, label) {
      if (!items.length) return "";
      return "<strong>" + escapeText(label) + "</strong><ul>" + items.map((item) => "<li>" + escapeText(item) + "</li>").join("") + "</ul>";
    }

    function renderTasks() {
      const filterValue = filter.value;
      flow.innerHTML = "";
      for (const task of orderedTasks) {
        const depState = dependencyState(task);
        if (filterValue === "open" && task.status === "done") continue;
        if (filterValue === "done" && task.status !== "done") continue;
        if (filterValue === "blocked" && depState !== "blocked") continue;
        const card = document.createElement("article");
        card.className = "task";
        card.draggable = true;
        card.dataset.id = task.id;
        card.dataset.status = task.status;
        card.dataset.state = depState;
        card.draggable = true;
        card.innerHTML = [
          '<div class="task-head">',
          '<h3>' + escapeText(task.id + " - " + task.title) + "</h3>",
          '<div class="task-actions">',
          '<button type="button" data-move="up" aria-label="上移 ' + escapeText(task.id) + '">上移</button>',
          '<button type="button" data-move="down" aria-label="下移 ' + escapeText(task.id) + '">下移</button>',
          '<span class="drag-handle" title="拖曳排序">拖曳</span>',
          '<span class="tag">' + escapeText(task.status + (depState === "blocked" ? " / blocked" : "")) + "</span>",
          "</div>",
          "</div>",
          '<p class="meta">' + escapeText(task.summary) + "</p>",
          '<p class="meta">依賴：' + escapeText(task.depends_on.join(", ") || "無") + (task.parallel_with.length ? "｜可並行：" + escapeText(task.parallel_with.join(", ")) : "") + "</p>",
          "<details><summary>完成後影響與細節</summary>",
          renderList([task.impact.user, task.impact.code, task.impact.risk], "影響"),
          renderList(task.scope, "Scope"),
          renderList(task.non_goals, "Non-goals"),
          renderList(task.done_criteria, "Done"),
          "</details>",
        ].join("");
        card.addEventListener("click", (event) => {
          if (event.target.closest("button[data-move]")) return;
          selectedTaskId = task.id;
          renderImpact();
          updateProposal();
        });
        card.addEventListener("click", (event) => {
          const button = event.target.closest("button[data-move]");
          if (!button) return;
          event.stopPropagation();
          moveTask(task.id, button.dataset.move);
        });
        flow.appendChild(card);
      }
      updateProposal();
    }

    function renderImpact() {
      const task = state.tasks.find((item) => item.id === selectedTaskId) || state.tasks[0];
      impact.innerHTML = [
        '<strong>' + escapeText(task.id + " - " + task.title) + "</strong>",
        '<p>使用者影響：' + escapeText(task.impact.user) + "</p>",
        '<p>程式影響：' + escapeText(task.impact.code) + "</p>",
        '<p>主要風險：' + escapeText(task.impact.risk) + "</p>",
      ].join("");
    }

    function updateProposal() {
      proposal.value = JSON.stringify({
        ...currentDecisionPayload(),
        confirmed_decision: confirmedDecision,
      }, null, 2);
    }

    flow.addEventListener("dragstart", (event) => {
      const card = event.target.closest(".task");
      if (!card) return;
      card.classList.add("dragging");
      event.dataTransfer.effectAllowed = "move";
      event.dataTransfer.setData("text/plain", card.dataset.id);
    });
    flow.addEventListener("dragend", (event) => {
      const card = event.target.closest(".task");
      if (card) card.classList.remove("dragging");
    });
    flow.addEventListener("dragover", (event) => {
      event.preventDefault();
      const dragging = flow.querySelector(".dragging");
      const target = event.target.closest(".task");
      if (!dragging || !target || dragging === target) return;
      const rect = target.getBoundingClientRect();
      flow.insertBefore(dragging, event.clientY > rect.top + rect.height / 2 ? target.nextSibling : target);
    });
    flow.addEventListener("drop", () => {
      const visibleIds = [...flow.querySelectorAll(".task")].map((item) => item.dataset.id);
      const hidden = orderedTasks.filter((task) => !visibleIds.includes(task.id));
      orderedTasks = visibleIds.map((id) => orderedTasks.find((task) => task.id === id)).concat(hidden);
      renderTasks();
      renderImpact();
    });
    filter.addEventListener("change", renderTasks);
    document.getElementById("reset").addEventListener("click", () => {
      orderedTasks = [...state.tasks];
      filter.value = "all";
      renderTasks();
    });
    document.getElementById("copy").addEventListener("click", async () => {
      await navigator.clipboard.writeText(proposal.value);
    });
    document.getElementById("confirmDecision").addEventListener("click", saveDecision);
    document.getElementById("clearDecision").addEventListener("click", () => {
      localStorage.removeItem(STORAGE_KEY);
      confirmedDecision = null;
      renderDecisionState();
      updateProposal();
    });
    Object.values(toggles).forEach((toggle) => toggle.addEventListener("change", () => {
      renderMock();
      updateProposal();
    }));

    loadDecision();
    renderOutcomes();
    renderLayoutChoices();
    renderSuggestionCards();
    renderQuestionCards();
    renderMock();
    renderTasks();
    renderImpact();
    renderDecisionState();
  </script>
</body>
</html>
`;
}

function renderAdrTemplateHtml() {
  return `<!doctype html>
<html lang="zh-Hant">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>ADR 決策預覽模板</title>
  <style>
    :root {
      --bg: #f7f8fa;
      --panel: #ffffff;
      --text: #172033;
      --muted: #5f6b7a;
      --line: #d9dee7;
      --accent: #1264a3;
    }
    * { box-sizing: border-box; }
    body {
      margin: 0;
      background: var(--bg);
      color: var(--text);
      font: 14px/1.5 system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
    }
    header, main {
      padding: 22px clamp(18px, 4vw, 48px);
    }
    header {
      border-bottom: 1px solid var(--line);
      background: var(--panel);
    }
    h1 { margin: 0; font-size: 24px; letter-spacing: 0; }
    h2 { margin: 0 0 10px; font-size: 17px; letter-spacing: 0; }
    main {
      display: grid;
      gap: 14px;
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }
    section {
      border: 1px solid var(--line);
      border-radius: 8px;
      background: var(--panel);
      padding: 14px;
    }
    .wide { grid-column: 1 / -1; }
    ul { margin: 8px 0 0 18px; padding: 0; }
    p { margin: 0; color: var(--muted); }
    code {
      background: #eef2f6;
      border-radius: 4px;
      padding: 1px 4px;
    }
    @media (max-width: 800px) {
      main { grid-template-columns: 1fr; }
    }
  </style>
</head>
<body>
  <header>
    <h1>ADR 決策預覽模板</h1>
    <p>這是 proposed ADR 的視覺輔助頁。正式決策、狀態與實作授權仍以 Markdown ADR 為準。</p>
  </header>
  <main>
    <section>
      <h2>預期成果</h2>
      <ul>
        <li>接受此 ADR 後，使用者體感會長什麼樣子。</li>
        <li>實作完成後，模組、資料契約或權限邊界應該變成什麼樣子。</li>
        <li>哪些東西明確不應該改。</li>
      </ul>
    </section>
    <section>
      <h2>決策選項</h2>
      <ul>
        <li>方案 A：維持現狀或最小改動。</li>
        <li>方案 B：建議採用路線。</li>
        <li>方案 C：被排除但需要記錄的替代方案。</li>
      </ul>
    </section>
    <section>
      <h2>風險與回滾</h2>
      <ul>
        <li>最高風險的行為、資料契約或安全邊界。</li>
        <li>回滾開關、migration 反向策略或相容視窗。</li>
        <li>merge 前需要人工確認的項目。</li>
      </ul>
    </section>
    <section>
      <h2>驗證關卡</h2>
      <ul>
        <li><code>npm run docs:refresh</code></li>
        <li>對應的 unit / integration checks。</li>
        <li>使用者可見行為的手動流程檢查。</li>
      </ul>
    </section>
    <section class="wide">
      <h2>預期檔案形狀</h2>
      <p>正式改 runtime code 前，先列出預期新增、修改、搬移、刪除的檔案與原因。</p>
    </section>
  </main>
</body>
</html>
`;
}

const state = buildState();
const summary = buildSummary(state);
const stateChanged = writeIfChanged(STATE_TARGET, `${JSON.stringify(state, null, 2)}\n`);
const summaryChanged = writeIfChanged(SUMMARY_TARGET, `${JSON.stringify(summary, null, 2)}\n`);
const workbenchChanged = writeIfChanged(WORKBENCH_TARGET, renderDecisionWorkbenchHtml(state));
const adrTemplateChanged = writeIfChanged(ADR_TEMPLATE_TARGET, renderAdrTemplateHtml());

const changed = [
  stateChanged ? STATE_TARGET : null,
  summaryChanged ? SUMMARY_TARGET : null,
  workbenchChanged ? WORKBENCH_TARGET : null,
  adrTemplateChanged ? ADR_TEMPLATE_TARGET : null,
].filter(Boolean);

if (changed.length === 0) {
  console.log("[docs-workbench-sync] Workbench outputs already up to date.");
} else {
  console.log("[docs-workbench-sync] Updated:");
  for (const file of changed) {
    console.log(`  - ${repoPath(path.join(ROOT, file))}`);
  }
}
