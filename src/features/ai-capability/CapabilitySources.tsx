import type { ReactElement } from "react";

import { useI18n } from "../../i18n/useI18n";
import type { CapabilitySource } from "./types";

export function CapabilitySources({
  sources,
}: {
  sources: CapabilitySource[];
}): ReactElement | null {
  const c = useI18n().capability;
  if (sources.length === 0) {
    return null;
  }

  return (
    <div
      className="text-[11px] leading-5 text-[color:var(--kn-text-faint)]"
      data-testid="capability-sources"
    >
      <span className="font-semibold uppercase tracking-[0.12em]">{c.basedOn}: </span>
      {sources.map((source, index) => (
        <span key={source.source_id} title={source.uri ?? source.source_type}>
          {index > 0 ? " · " : ""}
          {source.title}
        </span>
      ))}
    </div>
  );
}
