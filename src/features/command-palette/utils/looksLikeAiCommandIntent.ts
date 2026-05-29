const ENGLISH_ACTIONS = new Set([
  "build",
  "checkout",
  "copy",
  "create",
  "delete",
  "deploy",
  "diff",
  "find",
  "format",
  "generate",
  "grep",
  "install",
  "kill",
  "list",
  "lint",
  "make",
  "move",
  "open",
  "pull",
  "push",
  "remove",
  "rename",
  "restart",
  "run",
  "search",
  "show",
  "start",
  "stop",
  "switch",
  "tail",
  "test",
  "update",
]);

const SHELL_TOOLS = new Set([
  "bash",
  "cargo",
  "cmd",
  "docker",
  "git",
  "kubectl",
  "make",
  "node",
  "npm",
  "npx",
  "pip",
  "pnpm",
  "powershell",
  "pwsh",
  "python",
  "rg",
  "ripgrep",
  "sh",
  "uv",
  "yarn",
]);

const ENGLISH_OBJECT_HINTS = [
  "branch",
  "branches",
  "dependency",
  "dependencies",
  "directory",
  "directories",
  "file",
  "files",
  "folder",
  "folders",
  "log",
  "logs",
  "package",
  "packages",
  "panel",
  "project",
  "repo",
  "repository",
  "server",
  "terminal",
  "test",
  "tests",
  "workspace",
];

const CJK_ACTION_HINTS = [
  "打開",
  "開啟",
  "顯示",
  "列出",
  "找出",
  "搜尋",
  "執行",
  "跑",
  "建立",
  "新增",
  "刪除",
  "複製",
  "移動",
  "重新命名",
  "切換",
  "安裝",
  "更新",
  "推送",
  "拉取",
  "測試",
  "建置",
  "格式化",
];

const CJK_OBJECT_HINTS = [
  "檔案",
  "文件",
  "資料夾",
  "目錄",
  "專案",
  "分支",
  "終端",
  "終端機",
  "面板",
  "設定",
  "測試",
  "套件",
  "依賴",
  "日誌",
];

function tokenizeLatinWords(text: string): string[] {
  return text.toLowerCase().match(/[a-z0-9_.:-]+/g) ?? [];
}

function looksLikeEnglishCommandIntent(trimmed: string): boolean {
  const words = tokenizeLatinWords(trimmed);
  if (words.length === 0) return false;
  const [first, second = ""] = words;

  if (SHELL_TOOLS.has(first)) return true;
  if (ENGLISH_ACTIONS.has(first)) return true;
  if (first === "please" && ENGLISH_ACTIONS.has(second)) return true;
  if (first === "how" || first === "what" || first === "why") return false;

  const hasObjectHint = ENGLISH_OBJECT_HINTS.some((hint) => words.includes(hint));
  return hasObjectHint && (ENGLISH_ACTIONS.has(first) || ENGLISH_ACTIONS.has(second));
}

function looksLikeCjkCommandIntent(trimmed: string): boolean {
  if (!/[\u3400-\u9fff]/.test(trimmed)) return false;
  const hasAction = CJK_ACTION_HINTS.some((hint) => trimmed.includes(hint));
  if (!hasAction) return false;
  const hasObjectHint = CJK_OBJECT_HINTS.some((hint) => trimmed.includes(hint));
  if (hasObjectHint) return true;
  return Array.from(SHELL_TOOLS).some((tool) => trimmed.toLowerCase().includes(tool));
}

export function looksLikeAiCommandIntent(query: string): boolean {
  const trimmed = query.trim();
  if (!trimmed) return false;
  if (trimmed.startsWith("/") || trimmed.startsWith(">")) return false;
  return looksLikeEnglishCommandIntent(trimmed) || looksLikeCjkCommandIntent(trimmed);
}
