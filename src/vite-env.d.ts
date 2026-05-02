/// <reference types="vite/client" />

interface Window {
  // Tauri v2 runtime bridge — set by the Tauri webview before page load.
  // Undefined when running in a plain browser.
  __TAURI_INTERNALS__?: unknown;
}
