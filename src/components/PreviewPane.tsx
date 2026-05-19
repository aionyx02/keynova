import { convertFileSrc } from "@tauri-apps/api/core";
import type { FilePreviewResult, SearchResult } from "../types/search";

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
  if (!ms) return "—";
  return new Date(ms).toLocaleString();
}

export function PreviewPane({ result, preview, loading }: Props) {
  if (!result) {
    return (
      <div className="flex h-full items-center justify-center px-4 text-xs text-gray-600">
        Select a result to preview
      </div>
    );
  }

  if (loading && !preview) {
    return (
      <div className="space-y-2 px-3 py-3">
        <div className="h-3 w-3/4 animate-pulse rounded bg-gray-800/80" />
        <div className="h-3 w-1/2 animate-pulse rounded bg-gray-800/80" />
        <div className="h-3 w-2/3 animate-pulse rounded bg-gray-800/80" />
        <div className="h-3 w-1/3 animate-pulse rounded bg-gray-800/80" />
      </div>
    );
  }

  if (!preview) {
    return (
      <div className="flex h-full flex-col justify-center px-4 text-xs text-gray-500">
        <div className="mb-1 text-gray-400">No preview available</div>
        <div className="truncate">{result.name}</div>
      </div>
    );
  }

  if (preview.kind === "image") {
    return (
      <div className="flex h-full flex-col p-2">
        <div className="flex flex-1 items-center justify-center overflow-hidden rounded bg-gray-950/60">
          <img
            src={convertFileSrc(result.path)}
            alt=""
            className="max-h-full max-w-full object-contain"
            draggable={false}
          />
        </div>
        <div className="mt-1 truncate px-1 text-[10px] text-gray-500">
          {formatBytes(preview.size_bytes)} · {formatMtime(preview.modified_ms)}
        </div>
      </div>
    );
  }

  if (preview.kind === "binary") {
    return (
      <div className="flex h-full flex-col justify-center px-4 text-xs text-gray-400">
        <div className="mb-1 text-[10px] uppercase tracking-wider text-gray-500">Binary</div>
        <div className="mb-2 truncate text-gray-300">{result.name}</div>
        <div className="text-gray-500">size: {formatBytes(preview.size_bytes)}</div>
        <div className="text-gray-500">modified: {formatMtime(preview.modified_ms)}</div>
      </div>
    );
  }

  // text
  return (
    <div className="flex h-full flex-col">
      <div className="flex items-center justify-between border-b border-gray-800/60 px-2 py-1 text-[10px] uppercase tracking-wider text-gray-500">
        <span>Preview</span>
        {preview.truncated && (
          <span className="rounded bg-amber-500/20 px-1.5 py-0.5 text-[9px] text-amber-200">
            truncated
          </span>
        )}
      </div>
      <pre className="max-h-[336px] flex-1 overflow-y-auto whitespace-pre-wrap break-words bg-gray-950/40 px-2 py-1 font-mono text-[11px] leading-snug text-gray-300">
        {preview.content ?? ""}
      </pre>
      <div className="border-t border-gray-800/60 px-2 py-0.5 text-[10px] text-gray-600">
        {formatBytes(preview.size_bytes)}
        {preview.line_count !== undefined && ` · ${preview.line_count} lines shown`}
      </div>
    </div>
  );
}
