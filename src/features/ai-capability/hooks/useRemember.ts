import { useCallback } from "react";

import type { DispatchFn } from "../../../context/IPCContext";
import { parseRememberOutput, type RememberOutput, type RememberPayload } from "../types";
import { useCapability, type UseCapability, type UseCapabilityRunOptions } from "./useCapability";

export interface UseRemember extends Omit<UseCapability, "data" | "run"> {
  data: RememberOutput | null;
  run: (payload: RememberPayload | string, opts?: UseCapabilityRunOptions) => Promise<void>;
}

export function useRemember({ dispatch }: { dispatch: DispatchFn }): UseRemember {
  const inner = useCapability({ dispatch, id: "remember" });
  const innerRun = inner.run;

  const run = useCallback(
    (payload: RememberPayload | string, opts?: UseCapabilityRunOptions) => {
      const normalized: RememberPayload =
        typeof payload === "string" ? { text: payload } : payload;
      return innerRun(normalized, opts);
    },
    [innerRun],
  );

  return {
    ...inner,
    data: parseRememberOutput(inner.data),
    run,
  };
}
