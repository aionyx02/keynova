//! Windows application (Start Menu shortcut) scanning and launch.
//!
//! Extracted from `platform/windows.rs` (REF.9.C) as a pure structural move;
//! behavior unchanged.

use crate::models::app::AppInfo;
use std::path::Path;

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
            let path_str = path.to_string_lossy();
            if path_str.contains('\u{FFFD}') {
                eprintln!("[keynova] skipping non-UTF-8 app path: {:?}", path);
                continue;
            }
            out.push(AppInfo {
                name,
                path: path_str.into_owned(),
                icon_data: None,
                launch_count: 0,
            });
        }
    }
}

pub fn launch_app(path: &str) -> Result<(), String> {
    // Use the opener plugin (ShellExecute semantics) instead of `cmd /C start`
    // so paths containing shell metacharacters (e.g. `&`) cannot be reinterpreted
    // by cmd.exe — avoids command injection / launch mangling. (Security wave A #3)
    tauri_plugin_opener::open_path(Path::new(path), None::<&str>)
        .map_err(|e| format!("launch failed: {e}"))
}
