import { useCallback } from "react";

import type { DispatchFn } from "../../../context/IPCContext";
import {
  parseSuggestNextOutput,
  type SuggestedNextAction,
  type SuggestNextPayload,
} from "../types";
import { useCapability, type UseCapability, type UseCapabilityRunOptions } from "./useCapability";

export interface UseSuggestNext extends Omit<UseCapability, "data" | "run"> {
  data: SuggestedNextAction[];
  run: (payload?: SuggestNextPayload, opts?: UseCapabilityRunOptions) => Promise<void>;
}

export function useSuggestNext({ dispatch }: { dispatch: DispatchFn }): UseSuggestNext {
  const inner = useCapability({ dispatch, id: "suggest_next" });
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
