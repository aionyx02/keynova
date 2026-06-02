//! Mouse / keyboard input simulation, cursor movement, screen metrics, and
//! clipboard reads via Win32 SendInput / DataExchange.
//!
//! Focused Windows input helpers re-exported by the platform facade.

use windows::Win32::UI::Input::KeyboardAndMouse::{
    SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, INPUT_MOUSE, KEYBDINPUT, KEYEVENTF_KEYUP,
    MOUSEEVENTF_ABSOLUTE, MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP, MOUSEEVENTF_MIDDLEDOWN,
    MOUSEEVENTF_MIDDLEUP, MOUSEEVENTF_MOVE, MOUSEEVENTF_RIGHTDOWN, MOUSEEVENTF_RIGHTUP, MOUSEINPUT,
    VK_RETURN, VK_SPACE,
};

const CF_UNICODETEXT_FORMAT: u32 = 13;

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
        "Enter" | "Return" => VK_RETURN.0,
        "Space" => VK_SPACE.0,
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

/// Returns the Windows clipboard sequence number for low-cost change detection.
pub fn clipboard_sequence_number() -> u32 {
    unsafe { windows::Win32::System::DataExchange::GetClipboardSequenceNumber() }
}

/// Reads Unicode text from the Windows clipboard without spawning PowerShell.
pub fn read_clipboard_text() -> Option<String> {
    use windows::Win32::Foundation::{HGLOBAL, HWND};
    use windows::Win32::System::DataExchange::{
        CloseClipboard, GetClipboardData, IsClipboardFormatAvailable, OpenClipboard,
    };
    use windows::Win32::System::Memory::{GlobalLock, GlobalUnlock};

    unsafe {
        if IsClipboardFormatAvailable(CF_UNICODETEXT_FORMAT).is_err() {
            return None;
        }
        if OpenClipboard(HWND(std::ptr::null_mut())).is_err() {
            return None;
        }

        let result = (|| {
            let handle = GetClipboardData(CF_UNICODETEXT_FORMAT).ok()?;
            let global = HGLOBAL(handle.0);
            let ptr = GlobalLock(global) as *const u16;
            if ptr.is_null() {
                return None;
            }
            let mut len = 0usize;
            while *ptr.add(len) != 0 {
                len += 1;
            }
            let text = String::from_utf16_lossy(std::slice::from_raw_parts(ptr, len))
                .trim()
                .to_string();
            let _ = GlobalUnlock(global);
            (!text.is_empty()).then_some(text)
        })();

        let _ = CloseClipboard();
        result
    }
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
