import { convertFileSrc } from "@tauri-apps/api/core";
import { useState } from "react";

import type { FilePreviewResult, SearchResult } from "../../types/search";

interface Props {
  result: SearchResult | null;
  preview: FilePreviewResult | undefined;
  loading: boolean;
}

function formatBytes(n: number): string {
  if (n < 1024) return `${n} B`;
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB`;
  if (n < 1024 * 1024 * 1024) return `${(n / 1024 / 1024).toFixed(1)} MB`;
  return `${(n / 1024 / 1024 / 1024).toFixed(1)} GB`;
}

function formatMtime(ms?: number): string {
  if (!ms) return "Unknown";
  return new Date(ms).toLocaleString();
}

export function PreviewPane({ result, preview, loading }: Props) {
  const [failedImagePath, setFailedImagePath] = useState<string | null>(null);

  if (!result) {
    return (
      <div className="flex h-full items-center justify-center px-6 text-sm text-[color:var(--kn-text-muted)]">
        Select a result to preview
      </div>
    );
  }

  if (loading && !preview) {
    return (
      <div className="space-y-3 px-4 py-4">
        <div className="h-3 w-3/4 animate-pulse rounded-full bg-white/[0.07]" />
        <div className="h-3 w-1/2 animate-pulse rounded-full bg-white/[0.07]" />
        <div className="h-3 w-2/3 animate-pulse rounded-full bg-white/[0.07]" />
        <div className="h-3 w-1/3 animate-pulse rounded-full bg-white/[0.07]" />
      </div>
    );
  }

  if (!preview) {
    return (
      <div className="flex h-full flex-col justify-center px-5 text-sm text-[color:var(--kn-text-muted)]">
        <div className="mb-1 text-[color:var(--kn-text-soft)]">No preview available</div>
        <div className="truncate text-[11px] uppercase tracking-[0.14em] text-[color:var(--kn-text-faint)]">
          {result.name}
        </div>
      </div>
    );
  }

  if (preview.kind === "image") {
    const imageSrc = convertFileSrc(preview.path);
    const imageLoadFailed = failedImagePath === preview.path;
    return (
      <div className="flex h-full flex-col p-3">
        <div className="flex flex-1 items-center justify-center overflow-hidden rounded-[8px] border border-[color:var(--kn-border)] bg-white/[0.03] shadow-[inset_0_1px_0_rgba(255,255,255,0.04)]">
          {imageLoadFailed ? (
            <div className="px-5 text-center text-sm text-[color:var(--kn-text-muted)]">
              Image preview failed to load
            </div>
          ) : (
            <img
              src={imageSrc}
              alt=""
              className="max-h-full max-w-full object-contain"
              draggable={false}
              onError={() => setFailedImagePath(preview.path)}
            />
          )}
        </div>
        <div className="mt-2 truncate text-[11px] text-[color:var(--kn-text-muted)]">
          {formatBytes(preview.size_bytes)} / {formatMtime(preview.modified_ms)}
        </div>
      </div>
    );
  }

  if (preview.kind === "binary") {
    return (
      <div className="flex h-full flex-col justify-center px-5 text-sm text-[color:var(--kn-text-soft)]">
        <div className="mb-2 text-[10px] uppercase tracking-[0.2em] text-[color:var(--kn-text-faint)]">
          Binary
        </div>
        <div className="mb-2 truncate font-medium text-[color:var(--kn-text)]">{result.name}</div>
        <div className="text-[11px] text-[color:var(--kn-text-muted)]">
          size: {formatBytes(preview.size_bytes)}
        </div>
        <div className="text-[11px] text-[color:var(--kn-text-muted)]">
          modified: {formatMtime(preview.modified_ms)}
        </div>
      </div>
    );
  }

  return (
    <div className="flex h-full flex-col">
      <div className="flex items-center justify-between border-b border-[color:var(--kn-border)] bg-white/[0.02] px-3 py-2 text-[10px] uppercase tracking-[0.16em] text-[color:var(--kn-text-faint)]">
        <span>Preview</span>
        {preview.truncated && (
          <span className="rounded-full border border-amber-400/20 bg-amber-400/10 px-2 py-0.5 text-[9px] text-amber-200">
            truncated
          </span>
        )}
      </div>
      <pre className="kn-scroll flex-1 overflow-y-auto whitespace-pre-wrap break-words bg-transparent px-3 py-3 font-mono text-[11px] leading-6 text-[color:var(--kn-text-soft)]">
        {preview.content ?? ""}
      </pre>
      <div className="border-t border-[color:var(--kn-border)] px-3 py-2 text-[11px] text-[color:var(--kn-text-muted)]">
        {formatBytes(preview.size_bytes)}
        {preview.line_count !== undefined && ` / ${preview.line_count} lines shown`}
      </div>
    </div>
  );
}
