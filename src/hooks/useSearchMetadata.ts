import { useEffect, useState } from "react";
import { useIPC } from "./useIPC";
import { IPC } from "../ipc/routes";
import type { SearchIconAsset, SearchMetadata, SearchResult } from "../types/search";

export function useSearchMetadata(results: SearchResult[], selected: number) {
  const { dispatch } = useIPC();
  const [metadataByPath, setMetadataByPath] = useState<Record<string, SearchMetadata>>({});
  const [iconsByKey, setIconsByKey] = useState<Record<string, SearchIconAsset>>({});

  useEffect(() => {
    if (!window.__TAURI_INTERNALS__) return;
    const result = results[selected];
    if (!result || metadataByPath[result.path]) return;
    if (result.kind !== "file" && result.kind !== "folder" && result.kind !== "app") return;

    let cancelled = false;
    dispatch<SearchMetadata>(IPC.SEARCH_METADATA, { path: result.path, kind: result.kind })
      .then((metadata) => {
        if (cancelled) return;
        setMetadataByPath((prev) => ({ ...prev, [metadata.path]: metadata }));
      })
      .catch(() => {});
    return () => {
      cancelled = true;
    };
  }, [dispatch, selected, results, metadataByPath]);

  useEffect(() => {
    if (!window.__TAURI_INTERNALS__) return;
    const result = results[selected];
    const iconKey = result?.icon_key;
    if (!result || !iconKey || iconsByKey[iconKey]) return;

    let cancelled = false;
    dispatch<SearchIconAsset>(IPC.SEARCH_ICON, {
      icon_key: iconKey,
      kind: result.kind,
      path: result.path,
    })
      .then((asset) => {
        if (cancelled) return;
        setIconsByKey((prev) => ({ ...prev, [asset.icon_key]: asset }));
      })
      .catch(() => {});
    return () => {
      cancelled = true;
    };
  }, [dispatch, selected, results, iconsByKey]);

  return { metadataByPath, iconsByKey };
}