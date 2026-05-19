import { useCallback, useRef, useState } from "react";
import { useIPCContext } from "../context/IPCContext";
import type { PanelProps } from "../types/panel";

interface MaterialCandidate {
  path: string;
  name: string;
  class: "project" | "note" | "report" | "presentation" | "certificate" | "unknown";
  size_bytes: number;
  modified_secs: number;
}

interface ScanStats {
  scanned_count: number;
  candidate_count: number;
  filtered_count: number;
  denied_count: number;
}

interface ReviewReport {
  roots: string[];
  candidates: MaterialCandidate[];
  stats: ScanStats;
}

const CLASS_LABELS: Record<MaterialCandidate["class"], string> = {
  project: "Project",
  note: "Note",
  report: "Report",
  presentation: "Presentation",
  certificate: "Certificate",
  unknown: "Unknown",
};

const CLASS_ORDER: MaterialCandidate["class"][] = [
  "project", "note", "report", "presentation", "certificate", "unknown",
];

function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

export function LearningMaterialPanel({ onClose }: PanelProps) {
  const { dispatch } = useIPCContext();
  const [roots, setRoots] = useState("");
  const [report, setReport] = useState<ReviewReport | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [exportNote, setExportNote] = useState<string | null>(null);
  const [activeClass, setActiveClass] = useState<MaterialCandidate["class"] | "all">("all");
  const inputRef = useRef<HTMLInputElement>(null);

  const handleScan = useCallback(async () => {
    const rootList = roots
      .split(",")
      .map((r) => r.trim())
      .filter(Boolean);

    setLoading(true);
    setError(null);
    setReport(null);
    setExportNote(null);

    try {
      const result = await dispatch<ReviewReport>("learning_material.scan", {
        roots: rootList,
      });
      setReport(result);
      setActiveClass("all");
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setLoading(false);
    }
  }, [dispatch, roots]);

  const handleExportNote = useCallback(async () => {
    if (!report) return;
    const title = `Learning Review ${new Date().toISOString().slice(0, 10)}`;
    try {
      await dispatch("learning_material.export_note", { title, report });
      setExportNote(title);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  }, [dispatch, report]);

  const displayed =
    report?.candidates.filter(
      (c) => activeClass === "all" || c.class === activeClass
    ) ?? [];

  return (
    <div
      className="bg-gray-900/95 backdrop-blur-md rounded-b-xl shadow-2xl flex flex-col"
      style={{ maxHeight: 520 }}
    >
      {/* Header */}
      <div className="flex items-center justify-between px-4 py-2 border-b border-gray-700/50 shrink-0">
        <span className="text-xs font-semibold text-violet-400 uppercase tracking-wide">
          Learning Material Review
        </span>
        <button
          onClick={onClose}
          className="text-gray-600 hover:text-gray-400 text-xs transition-colors"
        >
          ✕
        </button>
      </div>

      {/* Scan input */}
      <div className="px-4 py-2 border-b border-gray-700/30 shrink-0">
        <div className="flex gap-2">
          <input
            ref={inputRef}
            value={roots}
            onChange={(e) => setRoots(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter") void handleScan();
              if (e.key === "Escape") onClose();
            }}
            placeholder="Scan roots (comma-separated paths, blank = workspace root)"
            className="flex-1 bg-gray-800/60 text-gray-200 text-xs rounded px-3 py-1.5 outline-none placeholder-gray-600"
          />
          <button
            onClick={() => void handleScan()}
            disabled={loading}
            className="px-3 py-1.5 text-xs rounded bg-violet-700/80 hover:bg-violet-600/80 text-white disabled:opacity-50 transition-colors shrink-0"
          >
            {loading ? "Scanning…" : "Scan"}
          </button>
        </div>
        {error && (
          <p className="mt-1 text-[10px] text-red-400 leading-tight">{error}</p>
        )}
      </div>

      {/* Stats bar */}
      {report && (
        <div className="px-4 py-1.5 border-b border-gray-700/30 flex gap-4 text-[10px] text-gray-500 shrink-0">
          <span className="text-gray-300">{report.stats.candidate_count} candidates</span>
          <span>{report.stats.scanned_count} scanned</span>
          <span>{report.stats.filtered_count} filtered</span>
          <span>{report.stats.denied_count} denied</span>

          {/* Class filter tabs */}
          <div className="ml-auto flex gap-1">
            <button
              onClick={() => setActiveClass("all")}
              className={`px-2 py-0.5 rounded text-[10px] transition-colors ${
                activeClass === "all"
                  ? "bg-violet-800/70 text-violet-200"
                  : "text-gray-500 hover:text-gray-300"
              }`}
            >
              All
            </button>
            {CLASS_ORDER.filter((cls) =>
              report.candidates.some((c) => c.class === cls)
            ).map((cls) => (
              <button
                key={cls}
                onClick={() => setActiveClass(cls)}
                className={`px-2 py-0.5 rounded text-[10px] transition-colors ${
                  activeClass === cls
                    ? "bg-violet-800/70 text-violet-200"
                    : "text-gray-500 hover:text-gray-300"
                }`}
              >
                {CLASS_LABELS[cls]}
              </button>
            ))}
          </div>
        </div>
      )}

      {/* Results */}
      <div className="flex-1 overflow-y-auto">
        {!report && !loading && (
          <p className="text-[11px] text-gray-600 px-4 py-4 text-center">
            Enter scan roots above and press Scan.
          </p>
        )}
        {loading && (
          <p className="text-[11px] text-gray-500 px-4 py-4 text-center animate-pulse">
            Scanning…
          </p>
        )}
        {report && displayed.length === 0 && (
          <p className="text-[11px] text-gray-600 px-4 py-4 text-center">
            No candidates found.
          </p>
        )}
        {displayed.map((item) => (
          <div
            key={item.path}
            className="flex items-start gap-2 px-4 py-2 border-b border-gray-800/50 hover:bg-gray-800/30 transition-colors"
          >
            <span className="text-[9px] uppercase font-semibold text-violet-500/80 mt-0.5 w-16 shrink-0">
              {CLASS_LABELS[item.class]}
            </span>
            <div className="flex-1 min-w-0">
              <p className="text-[11px] text-gray-200 truncate">{item.name}</p>
              <p className="text-[10px] text-gray-600 truncate">{item.path}</p>
            </div>
            <span className="text-[10px] text-gray-600 shrink-0">
              {formatBytes(item.size_bytes)}
            </span>
          </div>
        ))}
      </div>

      {/* Footer actions */}
      {report && (
        <div className="px-4 py-2 border-t border-gray-700/30 flex items-center gap-3 shrink-0">
          <button
            onClick={() => void handleExportNote()}
            className="text-[11px] px-3 py-1 rounded bg-gray-800/70 hover:bg-gray-700/70 text-gray-300 transition-colors"
          >
            Export as Note
          </button>
          {exportNote && (
            <span className="text-[10px] text-green-400">
              Saved: {exportNote}
            </span>
          )}
        </div>
      )}
    </div>
  );
}