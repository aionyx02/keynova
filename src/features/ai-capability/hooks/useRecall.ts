import { useCallback } from "react";

import type { DispatchFn } from "../../../context/IPCContext";
import { parseRecallOutput, type RecallPayload, type RecalledMemory } from "../types";
import { useCapability, type UseCapability, type UseCapabilityRunOptions } from "./useCapability";

export interface UseRecall extends Omit<UseCapability, "data" | "run"> {
  data: RecalledMemory[];
  run: (payload: RecallPayload | string, opts?: UseCapabilityRunOptions) => Promise<void>;
}

export function useRecall({ dispatch }: { dispatch: DispatchFn }): UseRecall {
  const inner = useCapability({ dispatch, id: "recall" });
  const innerRun = inner.run;

  const run = useCallback(
    (payload: RecallPayload | string, opts?: UseCapabilityRunOptions) => {
      const normalized: RecallPayload =
        typeof payload === "string" ? { query: payload } : payload;
      return innerRun(normalized, opts);
    },
    [innerRun],
  );

  return {
    ...inner,
    data: parseRecallOutput(inner.data),
    run,
  };
}
