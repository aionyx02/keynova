export type ResultKind =
  | "app"
  | "file"
  | "folder"
  | "command"
  | "note"
  | "history"
  | "model";

export interface ActionRef {
  id: string;
  session_id?: string;
  generation: number;
}

export interface SearchResult {
  item_ref?: ActionRef;
  title?: string;
  subtitle?: string;
  source?: string;
  icon_key?: string | null;
  primary_action?: ActionRef;
  primary_action_label?: string;
  secondary_action_count?: number;
  kind: ResultKind;
  name: string;
  path: string;
  score: number;
}

export interface SearchChunkDiagnostics {
  elapsed_ms: number;
  timed_out: boolean;
  file_cache_entries: number;
  tantivy_index_entries: number;
  everything_available: boolean;
  pre_balance_count: number;
  returned_count: number;
  fallback_reason: string | null;
}

export interface SearchChunkPayload {
  request_id: string;
  generation: number;
  chunk_index: number;
  items: SearchResult[];
  done: boolean;
  /** True on the final balanced batch — frontend should replace results entirely. */
  replace?: boolean;
  timed_out_providers?: string[];
  diagnostics?: SearchChunkDiagnostics;
}

export interface SearchErrorPayload {
  request_id: string;
  generation: number;
  error: string;
}

export interface SearchMetadata {
  path: string;
  exists: boolean;
  is_dir: boolean;
  size_bytes?: number;
  modified_ms?: number;
  preview?: string;
}

export interface SearchIconAsset {
  icon_key: string;
  mime: string;
  data_url: string;
}
