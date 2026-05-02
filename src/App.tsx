import { isTauri } from "@tauri-apps/api/core";
import { CommandPalette } from "./components/CommandPalette";

function App() {
  return (
    <>
      {/* Dev mode: connection status */}
      {import.meta.env.DEV && (
        <div
          className={`fixed top-0 left-0 right-0 z-[200] px-3 py-1 text-center text-xs font-mono ${
            isTauri()
              ? "bg-green-900/80 text-green-300"
              : "bg-red-900/80 text-red-300"
          }`}
        >
          {isTauri()
            ? "✓ Tauri — Ctrl+K 開關 | Ctrl+Alt+M 滑鼠控制 | Ctrl+Alt+W/A/S/D 移動"
            : "✗ 非 Tauri 環境 — 請用 npm run tauri dev 啟動"}
        </div>
      )}

      {/* Only the search box — window visibility = UI visibility */}
      <CommandPalette />
    </>
  );
}

export default App;
