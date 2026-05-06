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
