import { useCallback, useEffect, useState } from "react";

import type { DispatchFn } from "../../../context/IPCContext";

interface PinToggleResult {
  pinned?: boolean;
  commands?: unknown;
}

interface WorkspaceStateSlice {
  pinned_commands?: unknown;
}

function asStrings(value: unknown): string[] {
  return Array.isArray(value) ? value.filter((item): item is string => typeof item === "string") : [];
}

export interface UseWorkspacePins {
  pins: string[];
  toggle: (command: string) => Promise<void>;
  refresh: () => Promise<void>;
}

/**
 * PROFILE.2 (ADR-0055): the current workspace slot's pinned commands. Reads the
 * list from `workspace.get_current` and toggles via `workspace.pin`; the result
 * is merged atop the `profile` list in the palette. All failures are swallowed —
 * pinning is an enhancement, never a blocker.
 */
export function useWorkspacePins({ dispatch }: { dispatch: DispatchFn }): UseWorkspacePins {
  const [pins, setPins] = useState<string[]>([]);

  const refresh = useCallback(async () => {
    try {
      const state = await dispatch<WorkspaceStateSlice>("workspace.get_current");
      setPins(asStrings(state?.pinned_commands));
    } catch {
      /* ignore — leave the last known pins */
    }
  }, [dispatch]);

  // Load the current slot's pins once on mount. Inlined (rather than calling
  // `refresh`) so the setState lands inside the async resolution, not the effect
  // body — mirrors `useWorkspace`'s `workspace.get_current` load.
  useEffect(() => {
    let active = true;
    dispatch<WorkspaceStateSlice>("workspace.get_current")
      .then((state) => {
        if (active) setPins(asStrings(state?.pinned_commands));
      })
      .catch(() => {});
    return () => {
      active = false;
    };
  }, [dispatch]);

  const toggle = useCallback(
    async (command: string) => {
      try {
        const result = await dispatch<PinToggleResult>("workspace.pin", { command });
        setPins(asStrings(result?.commands));
      } catch {
        /* ignore */
      }
    },
    [dispatch],
  );

  return { pins, toggle, refresh };
}
