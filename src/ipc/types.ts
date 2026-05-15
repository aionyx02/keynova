import type { ActionRef } from "../types/search";
import type { TerminalLaunchSpec } from "../types/terminal";

// ── search ────────────────────────────────────────────────────────────────────

export interface SearchQueryRequest {
  query: string;
  limit?: number;
  stream?: boolean;
  request_id?: string;
  first_batch_limit?: number;
}

export interface SearchRecordSelectionRequest {
  source: string;
  path: string;
}

export interface SearchMetadataRequest {
  path: string;
  kind: string;
}

export interface SearchIconRequest {
  icon_key: string;
  kind: string;
  path: string;
}

export interface SearchBackendInfo {
  configured: string;
  active: string;
  everything_available: boolean;
  tantivy_available: boolean;
  file_cache_entries: number;
  tantivy_index_entries: number;
  tantivy_index_dir: string;
  rebuild_supported: boolean;
}

// ── setting ───────────────────────────────────────────────────────────────────

export interface SettingGetRequest {
  key: string;
}

export interface SettingGetResponse {
  value: string | null;
  sensitive: boolean;
}

export interface SettingSetRequest {
  key: string;
  value: string;
}

export interface SettingEntry {
  key: string;
  value: string;
  sensitive: boolean;
}

// ── terminal ──────────────────────────────────────────────────────────────────

export interface TerminalOpenRequest {
  rows: number;
  cols: number;
  launch_spec: TerminalLaunchSpec | null;
}

export interface TerminalOpenResponse {
  id: string;
  initial_output: string;
}

export interface TerminalSendRequest {
  id: string;
  input: string;
}

export interface TerminalResizeRequest {
  id: string;
  rows: number;
  cols: number;
}

export interface TerminalSessionRequest {
  id: string;
}

// ── action ────────────────────────────────────────────────────────────────────

export interface ActionRunRequest {
  action_ref: ActionRef;
}

// ── launcher ──────────────────────────────────────────────────────────────────

export interface LauncherLaunchRequest {
  path: string;
}

// ── builtin commands ──────────────────────────────────────────────────────────

export interface CmdRunRequest {
  name: string;
  args?: string;
}

export interface CmdSuggestArgsRequest {
  name: string;
  partial: string;
}