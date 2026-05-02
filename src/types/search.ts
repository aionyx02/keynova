export type ResultKind = "app" | "file" | "folder";

export interface SearchResult {
  kind: ResultKind;
  name: string;
  path: string;
  score: number;
}
