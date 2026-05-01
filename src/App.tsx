import { isTauri } from "@tauri-apps/api/core";
import { CommandPalette } from "./components/CommandPalette";
import { MouseControlOverlay } from "./components/MouseControlOverlay";

function App() {
  return (
    <>
      {/* Dev mode: show connection status */}
      {import.meta.env.DEV && (
        <div
          className={`fixed top-0 left-0 right-0 z-[200] px-3 py-1 text-center text-xs font-mono ${
            isTauri()
              ? "bg-green-900/80 text-green-300"
              : "bg-red-900/80 text-red-300"
          }`}
        >
          {isTauri()
            ? "✓ Tauri runtime 已連線 — 全局 Ctrl+K 切換視窗 / Esc 關閉"
            : "✗ 非 Tauri 環境 — 請用 npm run tauri dev 啟動"}
        </div>
      )}

      {/* CommandPalette is always mounted; window visibility controls show/hide */}
      <CommandPalette />

      {/* Keyboard/mouse control mode indicator */}
      <MouseControlOverlay />
    </>
  );
}

export default App;
