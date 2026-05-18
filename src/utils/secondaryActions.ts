import type { SearchResult } from "../types/search";

export type SecondaryActionId =
  | "reveal"
  | "copy_path"
  | "copy_name"
  | "show_metadata"
  | "open_with"
  | "open_as_text"
  | "rename"
  | "move"
  | "delete"
  | "hash";

export type ActionRisk = "low" | "medium" | "high";

export interface SecondaryActionItem {
  id: SecondaryActionId;
  label: string;
  hint?: string;
  risk: ActionRisk;
  disabled?: boolean;
  disabledReason?: string;
}

function isFilesystemKind(result: SearchResult): boolean {
  return result.kind === "file" || result.kind === "folder" || result.kind === "app";
}

export function isDestructive(id: SecondaryActionId): boolean {
  return id === "rename" || id === "move" || id === "delete";
}

export function buildSecondaryActions(result: SearchResult): SecondaryActionItem[] {
  const items: SecondaryActionItem[] = [];

  if (isFilesystemKind(result)) {
    items.push({ id: "reveal", label: "Reveal in Explorer", hint: "Open containing folder", risk: "low" });
    items.push({ id: "copy_path", label: "Copy path", hint: "Copy absolute path", risk: "low" });
    items.push({ id: "copy_name", label: "Copy name", hint: "Copy filename", risk: "low" });
    items.push({ id: "show_metadata", label: "Show metadata", hint: "Size, modified, preview", risk: "low" });
    items.push({ id: "open_with", label: "Open with…", hint: "OS default opener", risk: "low" });

    if (result.kind === "file") {
      items.push({ id: "open_as_text", label: "Open as text", hint: "Force text editor", risk: "low" });
    }

    if (result.kind !== "app") {
      items.push({ id: "rename", label: "Rename…", hint: "Enter new name", risk: "medium" });
      items.push({ id: "move", label: "Move to…", hint: "Enter target folder", risk: "medium" });
      items.push({ id: "delete", label: "Delete", hint: "Send to recycle bin", risk: "high" });
    }

    if (result.kind === "file") {
      items.push({ id: "hash", label: "Compute SHA-256", hint: "Stream hash", risk: "low" });
    }
  } else if (result.kind === "note") {
    items.push({ id: "copy_path", label: "Copy note path", risk: "low" });
    items.push({ id: "show_metadata", label: "Show metadata", risk: "low" });
  } else {
    items.push({ id: "show_metadata", label: "Show metadata", risk: "low" });
  }

  return items;
}

export function basenameFromPath(path: string): string {
  const idx = Math.max(path.lastIndexOf("/"), path.lastIndexOf("\\"));
  return idx >= 0 ? path.slice(idx + 1) : path;
}

export function parentDirFromPath(path: string): string {
  const idx = Math.max(path.lastIndexOf("/"), path.lastIndexOf("\\"));
  return idx >= 0 ? path.slice(0, idx) : "";
}
