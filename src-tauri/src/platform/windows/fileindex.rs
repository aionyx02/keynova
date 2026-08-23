//! Basic in-memory file index: directory tree scanning (no Everything needed),
//! user/Dropbox/OneDrive/WSL search-root discovery, and cached query lookup.
//!
//! Focused Windows file-index helpers re-exported by the platform facade.

use std::path::Path;
use std::sync::{Arc, Mutex, OnceLock};

use crate::core::SilentCommandExt;

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
pub fn build_file_index() -> usize {
    let mut entries: Vec<FileEntry> = Vec::new();
    // Shared visited set across all root scans prevents re-traversing directories
    // that were already covered by a higher-priority (deeper) root entry.
    let mut visited = std::collections::HashSet::<std::path::PathBuf>::new();
    for (dir, depth) in user_search_dirs() {
        if visited.insert(dir.clone()) {
            collect_all(&dir, &mut entries, depth, &mut visited);
        }
    }
    let len = entries.len();
    let cache = file_cache();
    let Ok(mut guard) = cache.lock() else {
        return 0;
    };
    *guard = entries;
    len
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

pub fn file_index_snapshot() -> Vec<FileEntry> {
    let cache = file_cache();
    cache.lock().map(|guard| guard.clone()).unwrap_or_default()
}

pub fn file_index_len() -> usize {
    let cache = file_cache();
    cache.lock().map(|guard| guard.len()).unwrap_or(0)
}

/// 掃描目錄樹，不做 query 過濾，只收集全部條目（供 build_file_index 使用）。
/// `visited` 跨所有根目錄共享，防止已被其他根目錄掃描過的子目錄被重複遞迴，
/// 從而避免快取中出現重複條目。
fn collect_all(
    dir: &Path,
    out: &mut Vec<FileEntry>,
    depth: usize,
    visited: &mut std::collections::HashSet<std::path::PathBuf>,
) {
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
        let path_str = path.to_string_lossy();
        if path_str.contains('\u{FFFD}') {
            eprintln!("[keynova] skipping non-UTF-8 file path: {:?}", path);
            continue;
        }
        out.push((name.to_string(), path_str.into_owned(), is_dir));
        // visited.insert returns true only if the path is newly inserted,
        // preventing re-traversal of directories already covered by another root.
        if is_dir && visited.insert(path.clone()) {
            collect_all(&path, out, depth - 1, visited);
        }
    }
}

/// 回傳 (目錄路徑, 最大掃描深度) 清單。
///
/// 不使用硬編碼目錄名稱（如 "Dropbox"、"OneDrive - Personal"）——這些名稱因語系、
/// 使用者設定而異。改用：
/// - OS 標準子目錄（Desktop/Downloads/Documents 等，這些在 Windows 上名稱固定）
/// - OneDrive env var（Microsoft sync client 設定，不受資料夾名稱影響）
/// - Dropbox info.json（Dropbox 自己的規範格式，包含實際路徑）
/// - 全磁碟掃描覆蓋其餘位置（global_drive_dirs）
fn user_search_dirs() -> Vec<(std::path::PathBuf, usize)> {
    let mut seen = std::collections::HashSet::new();
    let mut dirs: Vec<(std::path::PathBuf, usize)> = Vec::new();

    let mut add = |path: std::path::PathBuf, depth: usize| {
        if path.exists() && seen.insert(path.clone()) {
            dirs.push((path, depth));
        }
    };

    if let Ok(home_str) = std::env::var("USERPROFILE") {
        let home = std::path::PathBuf::from(&home_str);

        // OS-standardised user dirs: names are fixed by Windows shell APIs.
        for sub in &[
            "Desktop",
            "Downloads",
            "Documents",
            "Pictures",
            "Music",
            "Videos",
        ] {
            add(home.join(sub), 6);
        }

        // OneDrive: read the path that the sync client writes to env vars.
        // Works regardless of the folder's display name or relocated root.
        for key in &["OneDrive", "OneDriveConsumer", "OneDriveCommercial"] {
            if let Ok(value) = std::env::var(key) {
                add(std::path::PathBuf::from(value), 6);
            }
        }

        // Dropbox: read the canonical path from its own config file.
        // info.json is the authoritative source; the folder name is irrelevant.
        for p in dropbox_paths() {
            add(std::path::PathBuf::from(p), 6);
        }

        // Home at same depth as a catch-all for non-standard subdirectories
        // (e.g. ~/Projects, ~/code) not listed above.
        add(home, 6);
    }

    dirs.extend(global_drive_dirs());
    dirs.extend(wsl_home_dirs());
    dirs
}

/// Reads every `"path"` entry from a Dropbox `info.json` file.
/// The file format is: `{ "personal": { "path": "C:\\Users\\foo\\Dropbox" }, ... }`
fn dropbox_paths() -> Vec<String> {
    let mut paths = Vec::new();
    for env_key in &["LOCALAPPDATA", "APPDATA"] {
        if let Ok(base) = std::env::var(env_key) {
            let info = std::path::PathBuf::from(base)
                .join("Dropbox")
                .join("info.json");
            if let Ok(content) = std::fs::read_to_string(info) {
                paths.extend(extract_json_path_values(&content));
                if !paths.is_empty() {
                    break;
                }
            }
        }
    }
    paths
}

/// Minimal extraction of every `"path": "<value>"` entry from a JSON string.
/// Avoids pulling in serde_json for a single known-format file.
fn extract_json_path_values(json: &str) -> Vec<String> {
    let mut paths = Vec::new();
    let mut s = json;
    while let Some(pos) = s.find("\"path\"") {
        s = &s[pos + 6..];
        let after = s.trim_start();
        if !after.starts_with(':') {
            continue;
        }
        let after = after[1..].trim_start();
        if !after.starts_with('"') {
            continue;
        }
        let content = &after[1..];
        let mut end = 0;
        let bytes = content.as_bytes();
        while end < bytes.len() {
            if bytes[end] == b'\\' {
                end += 2; // skip escaped char
            } else if bytes[end] == b'"' {
                break;
            } else {
                end += 1;
            }
        }
        if end <= content.len() {
            let path = content[..end].replace("\\\\", "\\");
            if !path.is_empty() {
                paths.push(path);
            }
        }
    }
    paths
}

/// Enumerates available C-Z drives with bounded depth while skipping system directories.
fn global_drive_dirs() -> Vec<(std::path::PathBuf, usize)> {
    let mut dirs = Vec::new();
    for letter in b'C'..=b'Z' {
        let path = std::path::PathBuf::from(format!("{}:\\", letter as char));
        if path.exists() {
            dirs.push((path, 4));
        }
    }
    dirs
}

/// 透過 wsl.exe --list 取得已安裝的發行版名稱，
/// 枚舉 \\wsl.localhost\<Distro>\home 下的每個使用者目錄並各自加入索引根。
///
/// 直接加入 /home 根時，使用者的 ~/projects/code/file.py 需要 depth 3 才能到達，
/// 但 /home 本身消耗一層 → 只能掃到 ~/code/file.py（2 層）。
/// 改成枚舉 /home/<user>，每個使用者 home 獨立獲得 depth 4，才能覆蓋常見的
/// ~/projects/<repo>/<file> 層級。
fn wsl_home_dirs() -> Vec<(std::path::PathBuf, usize)> {
    let mut dirs = Vec::new();
    let distros = wsl_distro_names();

    for prefix in &[r"\\wsl.localhost", r"\\wsl$"] {
        for distro in &distros {
            let home_root = std::path::PathBuf::from(format!(r"{}\{}\home", prefix, distro));
            if !home_root.exists() {
                continue;
            }
            // Enumerate individual user home dirs so each gets a fresh depth budget.
            let mut found_any = false;
            if let Ok(entries) = std::fs::read_dir(&home_root) {
                for entry in entries.flatten() {
                    let user_home = entry.path();
                    if user_home.is_dir() {
                        dirs.push((user_home, 4));
                        found_any = true;
                    }
                }
            }
            if !found_any {
                // Can't enumerate — fall back to /home with reduced depth.
                dirs.push((home_root, 3));
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
        .no_window()
        .args(["--list", "--quiet"])
        .output()
    {
        if out.status.success() && !out.stdout.is_empty() {
            let names = parse_wsl_distro_names(&out.stdout);
            if !names.is_empty() {
                return names;
            }
            // wsl.exe 輸出 UTF-16 LE，含 BOM
            let words: Vec<u16> = out
                .stdout
                .as_chunks::<2>()
                .0
                .iter()
                .map(|c| u16::from_le_bytes(*c))
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

fn parse_wsl_distro_names(stdout: &[u8]) -> Vec<String> {
    if stdout.is_empty() {
        return Vec::new();
    }

    let text = if looks_like_utf16_le(stdout) {
        let words = stdout
            .as_chunks::<2>()
            .0
            .iter()
            .map(|c| u16::from_le_bytes(*c))
            .collect::<Vec<_>>();
        String::from_utf16_lossy(&words)
    } else {
        String::from_utf8_lossy(stdout).into_owned()
    };

    text.lines()
        .map(|line| {
            line.replace('\0', "")
                .trim()
                .trim_start_matches('\u{feff}')
                .trim()
                .to_string()
        })
        .filter(|line| !line.is_empty())
        .collect()
}

fn looks_like_utf16_le(bytes: &[u8]) -> bool {
    if bytes.len() < 4 {
        return false;
    }
    let pairs = bytes.as_chunks::<2>().0;
    if pairs.is_empty() {
        return false;
    }
    let nul_second_bytes = pairs.iter().filter(|pair| pair[1] == 0).count();
    nul_second_bytes * 2 >= pairs.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_utf8_wsl_distro_names() {
        let names = parse_wsl_distro_names(b"Ubuntu\r\nDebian\r\n");
        assert_eq!(names, vec!["Ubuntu", "Debian"]);
    }

    #[test]
    fn parses_utf16_wsl_distro_names() {
        let bytes = "Ubuntu\r\nDebian\r\n"
            .encode_utf16()
            .flat_map(u16::to_le_bytes)
            .collect::<Vec<_>>();
        let names = parse_wsl_distro_names(&bytes);
        assert_eq!(names, vec!["Ubuntu", "Debian"]);
    }

    #[test]
    fn extracts_dropbox_personal_path() {
        let json = r#"{"personal":{"path":"C:\\Users\\shawn\\Dropbox","host":123}}"#;
        let paths = extract_json_path_values(json);
        assert_eq!(paths, vec!["C:\\Users\\shawn\\Dropbox"]);
    }

    #[test]
    fn extracts_multiple_dropbox_paths() {
        let json = r#"{"personal":{"path":"C:\\Dropbox"},"business":{"path":"D:\\DropboxWork"}}"#;
        let paths = extract_json_path_values(json);
        assert_eq!(paths.len(), 2);
        assert!(paths.contains(&"C:\\Dropbox".to_string()));
        assert!(paths.contains(&"D:\\DropboxWork".to_string()));
    }

    #[test]
    fn dropbox_extraction_handles_empty_json() {
        assert!(extract_json_path_values("{}").is_empty());
        assert!(extract_json_path_values("").is_empty());
    }
}
