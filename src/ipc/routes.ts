export const IPC = {
  // search
  SEARCH_QUERY: "search.query",
  SEARCH_CANCEL: "search.cancel",
  SEARCH_RECORD_SELECTION: "search.record_selection",
  SEARCH_BACKEND: "search.backend",
  SEARCH_REBUILD_INDEX: "search.rebuild_index",
  SEARCH_ICON: "search.icon",
  SEARCH_METADATA: "search.metadata",

  // setting
  SETTING_GET: "setting.get",
  SETTING_SET: "setting.set",
  SETTING_LIST_ALL: "setting.list_all",
  SETTING_SCHEMA: "setting.schema",

  // terminal
  TERMINAL_OPEN: "terminal.open",
  TERMINAL_CLOSE: "terminal.close",
  TERMINAL_SEND: "terminal.send",
  TERMINAL_RESIZE: "terminal.resize",

  // action
  ACTION_RUN: "action.run",
  ACTION_LIST_SECONDARY: "action.list_secondary",

  // launcher
  LAUNCHER_LAUNCH: "launcher.launch",
  LAUNCHER_LIST: "launcher.list",

  // builtin commands
  CMD_RUN: "cmd.run",
  CMD_LIST: "cmd.list",
  CMD_SUGGEST_ARGS: "cmd.suggest_args",

  // hotkey
  HOTKEY_REGISTER: "hotkey.register",
  HOTKEY_UNREGISTER: "hotkey.unregister",
  HOTKEY_LIST: "hotkey.list",
  HOTKEY_CHECK_CONFLICT: "hotkey.check_conflict",

  // feature gate
  FEATURE_ACTIVATE: "feature.activate",

  // learning material review
  LEARNING_MATERIAL_SCAN: "learning_material.scan",
  LEARNING_MATERIAL_EXPORT_NOTE: "learning_material.export_note",
  LEARNING_MATERIAL_EXPORT_MARKDOWN: "learning_material.export_markdown",

  // ai (Phase 7a)
  AI_CHAT: "ai.chat",
  AI_CANCEL: "ai.cancel",
  AI_GET_HISTORY: "ai.get_history",
  AI_CLEAR_HISTORY: "ai.clear_history",
  AI_CHECK_SETUP: "ai.check_setup",
  AI_UNLOAD: "ai.unload",

  // agent (Phase 7a)
  AGENT_START: "agent.start",
  AGENT_CANCEL: "agent.cancel",
  AGENT_APPROVE: "agent.approve",
  AGENT_REJECT: "agent.reject",
  AGENT_CLEAR_RUNS: "agent.clear_runs",

  // file ops (Phase 8b — LAUNCH.1; populated incrementally across Slice 2-4)
  FILE_OPEN_WITH: "file.open_with",
  FILE_REVEAL: "file.reveal",
  FILE_RENAME: "file.rename",
  FILE_MOVE: "file.move",
  FILE_DELETE: "file.delete",
  FILE_HASH: "file.hash",
  FILE_PREVIEW: "file.preview",
  FILE_OPEN_AS_TEXT: "file.open_as_text",
} as const;

export type IpcRoute = (typeof IPC)[keyof typeof IPC];