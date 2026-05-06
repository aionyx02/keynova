#![cfg(target_os = "windows")]

use crate::models::app::AppInfo;
use std::path::Path;
use std::sync::{Arc, Mutex, OnceLock};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, INPUT_MOUSE, KEYBDINPUT, KEYEVENTF_KEYUP,
    MOUSEEVENTF_ABSOLUTE, MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP, MOUSEEVENTF_MIDDLEDOWN,
    MOUSEEVENTF_MIDDLEUP, MOUSEEVENTF_MOVE, MOUSEEVENTF_RIGHTDOWN, MOUSEEVENTF_RIGHTUP, MOUSEINPUT,
    VK_RETURN, VK_SPACE,
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

// ─── Basic File Scanner（無需 Everything）───────────────────────────────────

/// 已知噪音目錄：掃描時略過，避免 AppData / node_modules / build 產物拖慢速度。
const SKIP_DIRS: &[&str] = &[
    "AppData",
    "node_modules",
    ".git",
    ".cargo",
    ".rustup",
    ".npm",
    "target",
    "dist",
    ".idea",
    ".vscode",
    "System Volume Information",
    "$Recycle.Bin",
    "$WinREAgent",
    "Windows",
    "Program Files",
    "Program Files (x86)",
    "ProgramData",
    "Recovery",
    "boot",
];

// ─── In-memory file index cache ─────────────────────────────────────────────

type FileEntry = (String, String, bool); // (name, path, is_folder)

static FILE_CACHE: OnceLock<Arc<Mutex<Vec<FileEntry>>>> = OnceLock::new();

fn file_cache() -> Arc<Mutex<Vec<FileEntry>>> {
    Arc::clone(FILE_CACHE.get_or_init(|| Arc::new(Mutex::new(Vec::new()))))
}

/// 背景啟動後呼叫一次，將所有搜尋路徑的檔案條目寫入記憶體快取。
pub fn build_file_index() {
    let mut entries: Vec<FileEntry> = Vec::new();
    for (dir, depth) in user_search_dirs() {
        collect_all(&dir, &mut entries, depth);
    }
    let cache = file_cache();
    let Ok(mut guard) = cache.lock() else { return };
    *guard = entries;
}

/// 從記憶體快取中搜尋符合 query 的條目，不觸發磁碟 I/O。
pub fn scan_files_from_cache(query: &str, max: usize) -> Vec<FileEntry> {
    let q = query.to_lowercase();
    let cache = file_cache();
    let Ok(guard) = cache.lock() else {
        return Vec::new();
    };
    guard
        .iter()
        .filter(|(name, _, _)| name.to_lowercase().contains(&q))
        .take(max)
        .cloned()
        .collect()
}

/// 掃描目錄樹，不做 query 過濾，只收集全部條目（供 build_file_index 使用）。
fn collect_all(dir: &Path, out: &mut Vec<FileEntry>, depth: usize) {
    if depth == 0 {
        return;
    }
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        if name.starts_with('.') {
            continue;
        }
        let is_dir = path.is_dir();
        if is_dir && SKIP_DIRS.iter().any(|s| name.eq_ignore_ascii_case(s)) {
            continue;
        }
        out.push((
            name.to_string(),
            path.to_string_lossy().into_owned(),
            is_dir,
        ));
        if is_dir {
            collect_all(&path, out, depth - 1);
        }
    }
}

/// 回傳 (目錄路徑, 最大掃描深度) 清單。
fn user_search_dirs() -> Vec<(std::path::PathBuf, usize)> {
    let mut dirs = Vec::new();
    if let Ok(home) = std::env::var("USERPROFILE") {
        let home = std::path::PathBuf::from(home);
        for sub in &[
            "Desktop",
            "Downloads",
            "Documents",
            "Pictures",
            "Music",
            "Videos",
        ] {
            let p = home.join(sub);
            if p.exists() {
                dirs.push((p, 3));
            }
        }
        dirs.push((home, 2));
    }
    dirs.extend(extra_drive_dirs());
    dirs.extend(wsl_home_dirs());
    dirs
}

/// 枚舉 D–Z 槽，各以深度 3 掃描（跳過 SKIP_DIRS 定義的系統目錄）。
fn extra_drive_dirs() -> Vec<(std::path::PathBuf, usize)> {
    let mut dirs = Vec::new();
    for letter in b'D'..=b'Z' {
        let path = std::path::PathBuf::from(format!("{}:\\", letter as char));
        if path.exists() {
            dirs.push((path, 3));
        }
    }
    dirs
}

/// 透過 wsl.exe --list 取得已安裝的發行版名稱，
/// 再直接構建 \\wsl.localhost\<Distro>\home 路徑（WSL2 執行中時可存取）。
fn wsl_home_dirs() -> Vec<(std::path::PathBuf, usize)> {
    let mut dirs = Vec::new();
    let distros = wsl_distro_names();

    // 先嘗試 wsl.localhost（Windows 11+），再嘗試舊版 wsl$
    for prefix in &[r"\\wsl.localhost", r"\\wsl$"] {
        for distro in &distros {
            let home = std::path::PathBuf::from(format!(r"{}\{}\home", prefix, distro));
            if home.exists() {
                dirs.push((home, 3));
            }
        }
        if !dirs.is_empty() {
            break;
        }
    }
    dirs
}

/// 呼叫 `wsl.exe --list --quiet` 取得發行版清單（輸出為 UTF-16 LE）。
/// 失敗時回退至常見發行版名稱清單。
fn wsl_distro_names() -> Vec<String> {
    if let Ok(out) = std::process::Command::new("wsl.exe")
        .args(["--list", "--quiet"])
        .output()
    {
        if out.status.success() && !out.stdout.is_empty() {
            // wsl.exe 輸出 UTF-16 LE，含 BOM
            let words: Vec<u16> = out
                .stdout
                .chunks_exact(2)
                .map(|c| u16::from_le_bytes([c[0], c[1]]))
                .collect();
            let text = String::from_utf16_lossy(&words);
            let names: Vec<String> = text
                .lines()
                .map(|l| l.trim().trim_matches('\0').to_string())
                .filter(|l| !l.is_empty() && l != "\u{feff}")
                .collect();
            if !names.is_empty() {
                return names;
            }
        }
    }
    // 無 WSL 或解析失敗時的備用清單
    vec![
        "Ubuntu".into(),
        "Ubuntu-22.04".into(),
        "Ubuntu-24.04".into(),
        "Ubuntu-20.04".into(),
        "Debian".into(),
        "kali-linux".into(),
    ]
}

// ─── Everything IPC ──────────────────────────────────────────────────────────

type FnSetSearchW = unsafe extern "system" fn(*const u16);
type FnSetMax = unsafe extern "system" fn(u32);
type FnQueryW = unsafe extern "system" fn(i32) -> i32;
type FnGetNum = unsafe extern "system" fn() -> u32;
type FnGetFullPathW = unsafe extern "system" fn(u32, *mut u16, u32) -> u32;
type FnGetFileNameW = unsafe extern "system" fn(u32) -> *const u16;
type FnIsFolderResult = unsafe extern "system" fn(u32) -> i32;

struct EvFns {
    set_search_w: FnSetSearchW,
    set_max: FnSetMax,
    query_w: FnQueryW,
    get_num: FnGetNum,
    get_full_path_w: FnGetFullPathW,
    get_file_name_w: FnGetFileNameW,
    is_folder_result: FnIsFolderResult,
}

// fn ptrs are always Send+Sync
unsafe impl Send for EvFns {}
unsafe impl Sync for EvFns {}

static EV_FNS: std::sync::OnceLock<Option<EvFns>> = std::sync::OnceLock::new();
static EV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn init_everything() -> Option<EvFns> {
    use windows::core::PCWSTR;
    use windows::Win32::System::LibraryLoader::LoadLibraryW;

    let dll_paths: &[&str] = &[
        "Everything64.dll",
        r"C:\Program Files\Everything\Everything64.dll",
        r"C:\Program Files (x86)\Everything\Everything32.dll",
    ];

    for &path in dll_paths {
        let wide: Vec<u16> = path.encode_utf16().chain(Some(0)).collect();
        let Ok(hlib) = (unsafe { LoadLibraryW(PCWSTR(wide.as_ptr())) }) else {
            continue;
        };
        if let Some(fns) = load_ev_fns(hlib) {
            return Some(fns);
        }
    }
    None
}

fn load_ev_fns(hlib: windows::Win32::Foundation::HMODULE) -> Option<EvFns> {
    use windows::core::PCSTR;
    use windows::Win32::System::LibraryLoader::GetProcAddress;

    unsafe fn gp<F: Copy>(hlib: windows::Win32::Foundation::HMODULE, name: &[u8]) -> Option<F> {
        let raw = GetProcAddress(hlib, PCSTR(name.as_ptr()))?;
        Some(std::mem::transmute_copy(&raw))
    }

    unsafe {
        Some(EvFns {
            set_search_w: gp(hlib, b"Everything_SetSearchW\0")?,
            set_max: gp(hlib, b"Everything_SetMax\0")?,
            query_w: gp(hlib, b"Everything_QueryW\0")?,
            get_num: gp(hlib, b"Everything_GetNumResults\0")?,
            get_full_path_w: gp(hlib, b"Everything_GetResultFullPathNameW\0")?,
            get_file_name_w: gp(hlib, b"Everything_GetResultFileNameW\0")?,
            is_folder_result: gp(hlib, b"Everything_IsFolderResult\0")?,
        })
    }
}

/// 偵測 Everything DLL 是否可載入（不啟動服務不行查詢）。
pub fn check_everything() -> bool {
    EV_FNS.get_or_init(init_everything).is_some()
}

/// 透過 Everything DLL IPC 查詢，回傳 (name, full_path, is_folder) 清單。
/// Everything 服務需正在執行，否則回傳空清單。
pub fn everything_search(query: &str, max_results: u32) -> Vec<(String, String, bool)> {
    let Some(fns) = EV_FNS.get_or_init(init_everything) else {
        return Vec::new();
    };

    // Everything DLL 內部使用全域狀態，必須序列化呼叫
    let _guard = EV_LOCK.lock().unwrap_or_else(|e| e.into_inner());

    unsafe {
        let q_wide: Vec<u16> = query.encode_utf16().chain(Some(0)).collect();
        (fns.set_search_w)(q_wide.as_ptr());
        (fns.set_max)(max_results);

        if (fns.query_w)(1) == 0 {
            return Vec::new();
        }

        let count = (fns.get_num)().min(max_results);
        let mut results = Vec::with_capacity(count as usize);

        for i in 0..count {
            let mut buf = vec![0u16; 4096];
            let len = (fns.get_full_path_w)(i, buf.as_mut_ptr(), buf.len() as u32);
            if len == 0 {
                continue;
            }
            let full_path = String::from_utf16_lossy(&buf[..len as usize]).to_string();

            let name_ptr = (fns.get_file_name_w)(i);
            if name_ptr.is_null() {
                continue;
            }
            let name = read_wstr(name_ptr);
            let is_folder = (fns.is_folder_result)(i) != 0;
            results.push((name, full_path, is_folder));
        }

        results
    }
}

unsafe fn read_wstr(ptr: *const u16) -> String {
    let mut len = 0;
    while *ptr.add(len) != 0 {
        len += 1;
    }
    String::from_utf16_lossy(std::slice::from_raw_parts(ptr, len)).to_string()
}
