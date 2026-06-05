import type { SearchResult } from "../types/search";
import type { I18nKeys } from "../i18n/zh-TW";

type ActionLabels = I18nKeys["palette"]["actions"];

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

export function buildSecondaryActions(
  result: SearchResult,
  a: ActionLabels,
): SecondaryActionItem[] {
  const items: SecondaryActionItem[] = [];

  if (isFilesystemKind(result)) {
    items.push({ id: "reveal", label: a.revealLabel, hint: a.revealHint, risk: "low" });
    items.push({ id: "copy_path", label: a.copyPathLabel, hint: a.copyPathHint, risk: "low" });
    items.push({ id: "copy_name", label: a.copyNameLabel, hint: a.copyNameHint, risk: "low" });
    items.push({
      id: "show_metadata",
      label: a.showMetadataLabel,
      hint: a.showMetadataHint,
      risk: "low",
    });
    items.push({ id: "open_with", label: a.openWithLabel, hint: a.openWithHint, risk: "low" });

    if (result.kind === "file") {
      items.push({ id: "open_as_text", label: a.openAsTextLabel, hint: a.openAsTextHint, risk: "low" });
    }

    if (result.kind !== "app") {
      items.push({ id: "rename", label: a.renameLabel, hint: a.renameHint, risk: "medium" });
      items.push({ id: "move", label: a.moveLabel, hint: a.moveHint, risk: "medium" });
      items.push({ id: "delete", label: a.deleteLabel, hint: a.deleteHint, risk: "high" });
    }

    if (result.kind === "file") {
      items.push({ id: "hash", label: a.hashLabel, hint: a.hashHint, risk: "low" });
    }
  } else if (result.kind === "note") {
    items.push({ id: "copy_path", label: a.copyNotePathLabel, risk: "low" });
    items.push({ id: "show_metadata", label: a.showMetadataLabel, risk: "low" });
  } else {
    items.push({ id: "show_metadata", label: a.showMetadataLabel, risk: "low" });
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
