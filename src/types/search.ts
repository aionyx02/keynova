export type ResultKind = "app" | "file";

export interface SearchResult {
  kind: ResultKind;
  name: string;
  path: string;
  score: number;
}
