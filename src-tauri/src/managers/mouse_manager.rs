/// 管理鍵盤模擬滑鼠游標移動與點擊操作。
pub struct MouseManager;

impl MouseManager {
    pub fn new() -> Self {
        Self
    }

    /// 相對移動游標（WASD / 方向鍵使用）。
    pub fn move_cursor_relative(&self, dx: i32, dy: i32) -> Result<(), String> {
        #[cfg(target_os = "windows")]
        {
            crate::platform::windows::move_cursor_relative(dx, dy)
        }
        #[cfg(not(target_os = "windows"))]
        {
            let _ = (dx, dy);
            Err("mouse control not yet implemented on this platform".into())
        }
    }

    /// 移動游標至絕對座標。
    pub fn move_cursor_absolute(&self, x: i32, y: i32) -> Result<(), String> {
        #[cfg(target_os = "windows")]
        {
            crate::platform::windows::move_cursor_absolute(x, y)
        }
        #[cfg(not(target_os = "windows"))]
        {
            let _ = (x, y);
            Err("mouse control not yet implemented on this platform".into())
        }
    }

    /// 模擬滑鼠點擊（button: "left" | "right" | "middle"）。
    pub fn simulate_click(&self, button: &str, count: u32) -> Result<(), String> {
        #[cfg(target_os = "windows")]
        {
            crate::platform::windows::simulate_click(button, count)
        }
        #[cfg(not(target_os = "windows"))]
        {
            let _ = (button, count);
            Err("mouse control not yet implemented on this platform".into())
        }
    }

    /// 模擬鍵盤按鍵。
    pub fn simulate_key(&self, key: &str, modifiers: &[String]) -> Result<(), String> {
        #[cfg(target_os = "windows")]
        {
            crate::platform::windows::simulate_key(key, modifiers)
        }
        #[cfg(not(target_os = "windows"))]
        {
            let _ = (key, modifiers);
            Err("keyboard simulation not yet implemented on this platform".into())
        }
    }

    /// 模擬文字輸入。
    pub fn type_text(&self, text: &str) -> Result<(), String> {
        #[cfg(target_os = "windows")]
        {
            crate::platform::windows::type_text(text)
        }
        #[cfg(not(target_os = "windows"))]
        {
            let _ = text;
            Err("text input not yet implemented on this platform".into())
        }
    }
}
