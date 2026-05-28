import { useEffect, useRef, useState } from "react";

import { IPC } from "../ipc/routes";
import type { SearchIconAsset, SearchMetadata, SearchResult } from "../types/search";
import { useIPC } from "./useIPC";

const ICON_PREFETCH_LIMIT = 24;

export function useSearchMetadata(results: SearchResult[], selected: number) {
  const { dispatch } = useIPC();
  const [metadataByPath, setMetadataByPath] = useState<Record<string, SearchMetadata>>({});
  const [iconsByKey, setIconsByKey] = useState<Record<string, SearchIconAsset>>({});
  const pendingIconKeysRef = useRef<Set<string>>(new Set());

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
  }, [dispatch, metadataByPath, results, selected]);

  useEffect(() => {
    if (!window.__TAURI_INTERNALS__) return;

    const targets = results.slice(0, ICON_PREFETCH_LIMIT).filter(
      (
        result,
      ): result is SearchResult & {
        icon_key: string;
      } =>
        typeof result.icon_key === "string" &&
        result.icon_key.length > 0 &&
        !iconsByKey[result.icon_key] &&
        !pendingIconKeysRef.current.has(result.icon_key),
    );

    if (targets.length === 0) return;

    let cancelled = false;

    for (const result of targets) {
      const iconKey = result.icon_key;
      pendingIconKeysRef.current.add(iconKey);

      dispatch<SearchIconAsset>(IPC.SEARCH_ICON, {
        icon_key: iconKey,
        kind: result.kind,
        path: result.path,
      })
        .then((asset) => {
          if (cancelled) return;
          setIconsByKey((prev) => {
            if (prev[asset.icon_key]) return prev;
            return { ...prev, [asset.icon_key]: asset };
          });
        })
        .catch(() => {})
        .finally(() => {
          pendingIconKeysRef.current.delete(iconKey);
        });
    }

    return () => {
      cancelled = true;
    };
  }, [dispatch, iconsByKey, results]);

  return { metadataByPath, iconsByKey };
}
