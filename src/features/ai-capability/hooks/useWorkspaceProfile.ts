import { useCallback } from "react";

import type { DispatchFn } from "../../../context/IPCContext";
import {
  parseSuggestNextOutput,
  type SuggestedNextAction,
  type SuggestNextPayload,
} from "../types";
import { useCapability, type UseCapability, type UseCapabilityRunOptions } from "./useCapability";

export interface UseWorkspaceProfile extends Omit<UseCapability, "data" | "run"> {
  data: SuggestedNextAction[];
  run: (payload?: SuggestNextPayload, opts?: UseCapabilityRunOptions) => Promise<void>;
}

/**
 * PROFILE.1 (ADR-0054): the current project's signature commands. Same
 * `SuggestedNextAction[]` shape as `suggest_next`, so it reuses
 * `parseSuggestNextOutput` and the shared `CapabilityListCard` + replay path.
 */
export function useWorkspaceProfile({ dispatch }: { dispatch: DispatchFn }): UseWorkspaceProfile {
  const inner = useCapability({ dispatch, id: "workspace_profile" });
  const innerRun = inner.run;

  const run = useCallback(
    (payload: SuggestNextPayload = {}, opts?: UseCapabilityRunOptions) => {
      const normalized: SuggestNextPayload = { ctx: payload.ctx ?? {} };
      return innerRun(normalized, opts);
    },
    [innerRun],
  );

  return {
    ...inner,
    data: parseSuggestNextOutput(inner.data),
    run,
  };
}
