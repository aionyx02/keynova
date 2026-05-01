#![cfg(target_os = "windows")]

use crate::models::app::AppInfo;
use crate::models::hotkey::HotkeyConfig;
use std::path::Path;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    SendInput, INPUT, INPUT_0, INPUT_MOUSE, INPUT_KEYBOARD,
    MOUSEEVENTF_MOVE, MOUSEEVENTF_ABSOLUTE, MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP,
    MOUSEEVENTF_RIGHTDOWN, MOUSEEVENTF_RIGHTUP, MOUSEEVENTF_MIDDLEDOWN, MOUSEEVENTF_MIDDLEUP,
    MOUSEINPUT, KEYBDINPUT, KEYEVENTF_KEYUP, VK_RETURN, VK_SPACE,
};

// ─── App Scanner ────────────────────────────────────────────────────────────

/// 掃描 Windows Start Menu 與 %LOCALAPPDATA%\Programs 下的捷徑。
pub fn scan_applications() -> Vec<AppInfo> {
    let mut apps = Vec::new();
    let search_dirs = start_menu_dirs();
    for dir in search_dirs {
        collect_lnk_files(&dir, &mut apps);
    }
    apps
}

fn start_menu_dirs() -> Vec<std::path::PathBuf> {
    let mut dirs = Vec::new();
    if let Ok(appdata) = std::env::var("APPDATA") {
        dirs.push(
            Path::new(&appdata)
                .join("Microsoft")
                .join("Windows")
                .join("Start Menu")
                .join("Programs"),
        );
    }
    if let Ok(local) = std::env::var("LOCALAPPDATA") {
        dirs.push(Path::new(&local).join("Programs"));
    }
    // 系統級 Start Menu
    dirs.push(Path::new(r"C:\ProgramData\Microsoft\Windows\Start Menu\Programs").to_path_buf());
    dirs
}

fn collect_lnk_files(dir: &Path, out: &mut Vec<AppInfo>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_lnk_files(&path, out);
        } else if path.extension().and_then(|e| e.to_str()) == Some("lnk") {
            let name = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_string();
            if name.is_empty() {
                continue;
            }
            out.push(AppInfo {
                name,
                path: path.to_string_lossy().into_owned(),
                icon_data: None,
                launch_count: 0,
            });
        }
    }
}

/// 以 ShellExecute 語義開啟捷徑或可執行檔。
pub fn launch_app(path: &str) -> Result<(), String> {
    std::process::Command::new("cmd")
        .args(["/C", "start", "", path])
        .spawn()
        .map_err(|e| e.to_string())?;
    Ok(())
}

// ─── Hotkey ─────────────────────────────────────────────────────────────────

/// 向 Windows 註冊全局快捷鍵（RegisterHotKey）。
/// 目前為骨架實作，完整 message loop 整合於後續 PR 補齊。
pub fn register_hotkey(_config: &HotkeyConfig) -> Result<(), String> {
    // TODO: 呼叫 RegisterHotKey WinAPI + 啟動 message loop thread
    Ok(())
}

pub fn unregister_hotkey(_id: &str) -> Result<(), String> {
    // TODO: 呼叫 UnregisterHotKey WinAPI
    Ok(())
}

// ─── Mouse / Keyboard Input ─────────────────────────────────────────────────

pub fn move_cursor_relative(dx: i32, dy: i32) -> Result<(), String> {
    send_mouse_input(MOUSEEVENTF_MOVE, dx, dy, 0)
}

pub fn move_cursor_absolute(x: i32, y: i32) -> Result<(), String> {
    // MOUSEEVENTF_ABSOLUTE 座標範圍 0–65535
    let norm_x = (x * 65535) / screen_width();
    let norm_y = (y * 65535) / screen_height();
    send_mouse_input(MOUSEEVENTF_MOVE | MOUSEEVENTF_ABSOLUTE, norm_x, norm_y, 0)
}

pub fn simulate_click(button: &str, count: u32) -> Result<(), String> {
    let (down_flag, up_flag) = match button {
        "right" => (MOUSEEVENTF_RIGHTDOWN, MOUSEEVENTF_RIGHTUP),
        "middle" => (MOUSEEVENTF_MIDDLEDOWN, MOUSEEVENTF_MIDDLEUP),
        _ => (MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP),
    };
    for _ in 0..count {
        send_mouse_input(down_flag, 0, 0, 0)?;
        send_mouse_input(up_flag, 0, 0, 0)?;
    }
    Ok(())
}

pub fn simulate_key(key: &str, _modifiers: &[String]) -> Result<(), String> {
    let vk = match key {
        "Enter" | "Return" => VK_RETURN.0 as u16,
        "Space" => VK_SPACE.0 as u16,
        _ => return Err(format!("unsupported key: '{key}'")),
    };
    send_key_input(vk, false)?;
    send_key_input(vk, true)?;
    Ok(())
}

pub fn type_text(text: &str) -> Result<(), String> {
    for ch in text.chars() {
        send_unicode_char(ch as u16, false)?;
        send_unicode_char(ch as u16, true)?;
    }
    Ok(())
}

// ─── Internal helpers ────────────────────────────────────────────────────────

fn send_mouse_input(
    flags: windows::Win32::UI::Input::KeyboardAndMouse::MOUSE_EVENT_FLAGS,
    dx: i32,
    dy: i32,
    data: u32,
) -> Result<(), String> {
    let input = INPUT {
        r#type: INPUT_MOUSE,
        Anonymous: INPUT_0 {
            mi: MOUSEINPUT {
                dx,
                dy,
                mouseData: data,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    };
    unsafe {
        let sent = SendInput(&[input], std::mem::size_of::<INPUT>() as i32);
        if sent == 0 {
            return Err("SendInput failed".into());
        }
    }
    Ok(())
}

fn send_key_input(vk: u16, key_up: bool) -> Result<(), String> {
    let flags = if key_up {
        KEYEVENTF_KEYUP
    } else {
        windows::Win32::UI::Input::KeyboardAndMouse::KEYBD_EVENT_FLAGS(0)
    };
    let input = INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: windows::Win32::UI::Input::KeyboardAndMouse::VIRTUAL_KEY(vk),
                wScan: 0,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    };
    unsafe {
        let sent = SendInput(&[input], std::mem::size_of::<INPUT>() as i32);
        if sent == 0 {
            return Err("SendInput (key) failed".into());
        }
    }
    Ok(())
}

fn send_unicode_char(ch: u16, key_up: bool) -> Result<(), String> {
    use windows::Win32::UI::Input::KeyboardAndMouse::KEYEVENTF_UNICODE;
    let flags = if key_up {
        KEYEVENTF_UNICODE | KEYEVENTF_KEYUP
    } else {
        KEYEVENTF_UNICODE
    };
    let input = INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: windows::Win32::UI::Input::KeyboardAndMouse::VIRTUAL_KEY(0),
                wScan: ch,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    };
    unsafe {
        let sent = SendInput(&[input], std::mem::size_of::<INPUT>() as i32);
        if sent == 0 {
            return Err("SendInput (unicode) failed".into());
        }
    }
    Ok(())
}

fn screen_width() -> i32 {
    use windows::Win32::UI::WindowsAndMessaging::{GetSystemMetrics, SM_CXSCREEN};
    unsafe { GetSystemMetrics(SM_CXSCREEN) }
}

fn screen_height() -> i32 {
    use windows::Win32::UI::WindowsAndMessaging::{GetSystemMetrics, SM_CYSCREEN};
    unsafe { GetSystemMetrics(SM_CYSCREEN) }
}
