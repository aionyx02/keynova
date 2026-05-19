use std::sync::Arc;
use std::time::{Duration, Instant};

use tauri::{Emitter, Manager};

use crate::app::state::AppState;
use crate::core::IpcError;
pub(crate) fn show_launcher(app: &tauri::AppHandle) -> Result<(), IpcError> {
    let window = app
        .get_webview_window("main")
        .ok_or_else(|| IpcError::new("window_not_found", "main window not found"))?;
    let _ = window.unminimize();
    window
        .show()
        .map_err(|e| IpcError::tauri_api("window.show", e.to_string()))?;
    window
        .set_focus()
        .map_err(|e| IpcError::tauri_api("window.set_focus", e.to_string()))
}

// ─── Entry point ─────────────────────────────────────────────────────────────

pub(crate) fn setup_main_window(app: &tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let window = app
        .get_webview_window("main")
        .ok_or("main window not found")?;

    if let Ok(Some(monitor)) = window.current_monitor() {
        let screen = monitor.size();
        let scale = monitor.scale_factor();
        let phys_w = (640.0 * scale) as i32;
        let x = ((screen.width as i32 - phys_w) / 2).max(0);
        let y = (screen.height as f64 * 0.25) as i32;
        let _ = window.set_position(tauri::PhysicalPosition::new(x, y));
    }

    let window_focused = window.clone();
    let window_blur = window.clone();
    let blur_guard = app.state::<AppState>().launcher_focus_guard.clone();
    window.on_window_event(move |event| match event {
        tauri::WindowEvent::Focused(true) => {
            let _ = window_focused.emit("window-focused", ());
        }
        tauri::WindowEvent::Focused(false) => {
            // Bug-fix 2026-05-19 (round 2) — 真根因不是 IME composition：使用者
            // 回報英文打字、滑鼠不動、單純坐著都會觸發。WebView2 transparent
            // window 在 Windows 11 任何 keystroke / accessibility subprocess
            // 切換 / popup 都可能 emit 短暫 Focused(false) blip。先前的 400ms
            // grace + 只 hook onComposition* 的 frontend guard 覆蓋不到英文
            // typing path。
            //
            // 修法：grace 拉到 1500ms（覆蓋幾乎所有觀察到的 blip），配合 frontend
            // 在 input onFocus + 每個 keydown（200ms throttle）主動把 launcher_focus_guard
            // 更新成 2s TTL — 任何使用者互動都會把 guard 推到未來，sleep 完才
            // 檢查 guard 時保證 guard 還有效 → 不 hide。真要 dismiss 用 Esc /
            // Ctrl+K toggle / 等 1.5s 點別處兩種路徑。
            let window_blur = window_blur.clone();
            let blur_guard = Arc::clone(&blur_guard);
            tauri::async_runtime::spawn(async move {
                tokio::time::sleep(Duration::from_millis(1500)).await;
                let should_keep_open = blur_guard
                    .lock()
                    .map(|mut guard| {
                        let now = Instant::now();
                        match guard.as_ref() {
                            Some(until) if now <= *until => true,
                            Some(_) => {
                                *guard = None;
                                false
                            }
                            None => false,
                        }
                    })
                    .unwrap_or(false);
                if should_keep_open {
                    let _ = window_blur.show();
                    let _ = window_blur.set_focus();
                    return;
                }
                if window_blur.is_focused().unwrap_or(false) {
                    return;
                }
                let _ = window_blur.hide();
            });
        }
        _ => {}
    });

    Ok(())
}
