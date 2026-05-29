import { useCallback } from "react";

import type { DispatchFn } from "../../../context/IPCContext";
import { parseGenCommandOutput, type GenCommandOutput, type GenCommandPayload } from "../types";
import { useCapability, type UseCapability, type UseCapabilityRunOptions } from "./useCapability";

export interface UseGenCommand extends Omit<UseCapability, "data" | "run"> {
  data: GenCommandOutput | null;
  run: (payload: GenCommandPayload | string, opts?: UseCapabilityRunOptions) => Promise<void>;
}

export function useGenCommand({ dispatch }: { dispatch: DispatchFn }): UseGenCommand {
  const inner = useCapability({ dispatch, id: "gen_command" });
  const innerRun = inner.run;

  const run = useCallback(
    (payload: GenCommandPayload | string, opts?: UseCapabilityRunOptions) => {
      const normalized: GenCommandPayload =
        typeof payload === "string"
          ? { intent: payload, ctx: {} }
          : { ...payload, ctx: payload.ctx ?? {} };
      return innerRun(normalized, opts);
    },
    [innerRun],
  );

  return {
    ...inner,
    data: parseGenCommandOutput(inner.data),
    run,
  };
}
