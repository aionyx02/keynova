import React from "react";

import { fmt } from "../../i18n/format";
import { resolveI18n } from "../../i18n/useI18n";

interface Props {
  children: React.ReactNode;
}

interface State {
  error: Error | null;
  info: React.ErrorInfo | null;
}

/**
 * Top-level error boundary for the launcher window. Catches synchronous
 * renderer errors that would otherwise blank the WebView. Intentionally
 * does **not** touch IPC, the Zustand store, or any provider — if the app
 * is already crashing, the fallback must stand on its own.
 *
 * Bug-fix 2026-05-18: launcher was crashing intermittently on Ctrl+K open
 * with no recovery path. This catches the renderer error so the user sees
 * an actionable card instead of a dead window, and logs full stack to the
 * console so the next reproduction leaves evidence.
 */
export class ErrorBoundary extends React.Component<Props, State> {
  state: State = { error: null, info: null };

  static getDerivedStateFromError(error: Error): Partial<State> {
    return { error };
  }

  componentDidCatch(error: Error, info: React.ErrorInfo) {
    console.error("[keynova:render-error]", error, info.componentStack);
    this.setState({ info });
  }

  private reload = () => {
    window.location.reload();
  };

  private copyError = async () => {
    const { error, info } = this.state;
    if (!error) return;
    const t = resolveI18n().errorBoundary;
    const payload = [
      `Error: ${error.name}: ${error.message}`,
      error.stack ?? t.noStack,
      "",
      t.componentStack,
      info?.componentStack ?? t.noComponentStack,
      "",
      fmt(t.userAgent, { value: navigator.userAgent }),
      fmt(t.time, { value: new Date().toISOString() }),
    ].join("\n");
    try {
      await navigator.clipboard.writeText(payload);
    } catch {
      /* clipboard may not be available; user can still read on-screen */
    }
  };

  render() {
    const { error } = this.state;
    if (!error) return this.props.children;
    const t = resolveI18n().errorBoundary;

    return (
      <div className="fixed inset-0 z-50 flex items-center justify-center bg-gray-950/95 p-6">
        <div className="w-[520px] max-w-full rounded-xl border border-red-500/40 bg-gray-900 shadow-2xl">
          <div className="border-b border-red-500/30 px-5 py-3">
            <div className="text-base font-semibold text-red-300">{t.title}</div>
            <div className="mt-1 text-xs text-gray-500">{t.subtitle}</div>
          </div>
          <div className="px-5 py-3 text-xs">
            <div className="mb-1 text-[10px] uppercase tracking-wider text-gray-500">
              {error.name || t.fallbackErrorName}
            </div>
            <div className="font-mono text-gray-200 whitespace-pre-wrap break-words">
              {error.message || t.noMessage}
            </div>
          </div>
          <div className="flex items-center justify-end gap-2 border-t border-gray-700/50 px-4 py-2">
            <button
              type="button"
              onClick={this.copyError}
              className="rounded px-3 py-1 text-xs text-gray-300 hover:bg-gray-800"
            >
              {t.copyError}
            </button>
            <button
              type="button"
              onClick={this.reload}
              className="rounded bg-sky-600 px-3 py-1 text-xs font-medium text-white hover:bg-sky-500"
            >
              {t.reload}
            </button>
          </div>
        </div>
      </div>
    );
  }
}
