//! Clipboard reads via Win32 DataExchange.
//!
//! Focused Windows clipboard helpers re-exported by the platform facade.
//! (REF.8 removed the mouse/keyboard SendInput simulation helpers along with the
//! mouse-control feature.)

const CF_UNICODETEXT_FORMAT: u32 = 13;

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
