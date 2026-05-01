import { useCallback, useEffect, useState } from "react";
import { isTauri } from "@tauri-apps/api/core";
import { CommandPalette } from "./components/CommandPalette";
import { MouseControlOverlay } from "./components/MouseControlOverlay";

function App() {
  const [paletteOpen, setPaletteOpen] = useState(false);

  const closePalette = useCallback(() => setPaletteOpen(false), []);

  // Win+K → 開啟命令面板（全局鍵盤監聽，後續會由 HotkeyManager 替換）
  useEffect(() => {
    function onKeyDown(e: KeyboardEvent) {
      if ((e.metaKey || e.ctrlKey) && e.key === "k") {
        e.preventDefault();
        setPaletteOpen((v) => !v);
      }
    }
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, []);

  return (
    <div className="min-h-screen bg-gray-950 text-gray-100">
      {/* 開發模式狀態列 */}
      {import.meta.env.DEV && (
        <div
          className={`fixed top-0 left-0 right-0 z-[100] px-3 py-1 text-center text-xs font-mono ${
            isTauri() ? "bg-green-900 text-green-300" : "bg-red-900 text-red-300"
          }`}
        >
          {isTauri()
            ? "✓ Tauri runtime 已連線 — 按 Ctrl+K 開啟命令面板"
            : "✗ 非 Tauri 環境 — 請用 npm run tauri dev 啟動"}
        </div>
      )}

      {/* 主要內容區：目前顯示提示；後續各功能組件掛載於此 */}
      <main className="flex min-h-screen items-center justify-center">
        <div className="text-center">
          <h1 className="text-3xl font-bold tracking-tight">Keynova</h1>
          <p className="mt-2 text-sm text-gray-500">
            按 <kbd className="rounded bg-gray-800 px-1.5 py-0.5 font-mono text-xs">Ctrl+K</kbd> 開啟應用程式啟動器
          </p>
        </div>
      </main>

      {/* 命令面板（全域浮層） */}
      {paletteOpen && <CommandPalette onClose={closePalette} />}

      {/* 鍵盤滑鼠控制模式指示器 */}
      <MouseControlOverlay />
    </div>
  );
}

export default App;
