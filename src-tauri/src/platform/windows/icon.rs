//! Shell icon extraction (PowerShell + SHGetFileInfo) with in-memory + on-disk
//! PNG caching.
//!
//! Focused Windows icon helpers re-exported by the platform facade.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

/// 以 ShellExecute 語義開啟捷徑或可執行檔。
static SEARCH_ICON_CACHE: OnceLock<Mutex<HashMap<String, Option<String>>>> = OnceLock::new();

const SEARCH_ICON_SCRIPT: &str = r#"
Add-Type -AssemblyName System.Drawing
if (-not ("KeynovaShellIcon" -as [type])) {
Add-Type @"
using System;
using System.Runtime.InteropServices;

public static class KeynovaShellIcon {
    [StructLayout(LayoutKind.Sequential, CharSet = CharSet.Unicode)]
    public struct SHFILEINFO {
        public IntPtr hIcon;
        public int iIcon;
        public uint dwAttributes;
        [MarshalAs(UnmanagedType.ByValTStr, SizeConst = 260)]
        public string szDisplayName;
        [MarshalAs(UnmanagedType.ByValTStr, SizeConst = 80)]
        public string szTypeName;
    }

    [DllImport("Shell32.dll", CharSet = CharSet.Unicode)]
    public static extern IntPtr SHGetFileInfo(
        string pszPath,
        uint dwFileAttributes,
        out SHFILEINFO psfi,
        uint cbFileInfo,
        uint uFlags
    );

    [DllImport("User32.dll", SetLastError = true)]
    public static extern bool DestroyIcon(IntPtr hIcon);
}
"@
}

$path = $env:KEYNOVA_ICON_PATH
$kind = $env:KEYNOVA_ICON_KIND
if ([string]::IsNullOrWhiteSpace($path)) {
    exit 1
}

$attrs = 0
if ($kind -eq "folder") {
    $attrs = 0x10
} elseif ($kind -eq "file") {
    $attrs = 0x80
}

$flags = 0x100
if ($env:KEYNOVA_ICON_USE_ATTRS -eq "1") {
    $flags = $flags -bor 0x10
}
$info = New-Object KeynovaShellIcon+SHFILEINFO
[void][KeynovaShellIcon]::SHGetFileInfo(
    $path,
    [uint32]$attrs,
    [ref]$info,
    [uint32][System.Runtime.InteropServices.Marshal]::SizeOf([type][KeynovaShellIcon+SHFILEINFO]),
    [uint32]$flags
)

if ($info.hIcon -eq [IntPtr]::Zero) {
    exit 1
}

try {
    $icon = [System.Drawing.Icon]::FromHandle($info.hIcon)
    $bitmap = $icon.ToBitmap()
    $stream = New-Object System.IO.MemoryStream
    $bitmap.Save($stream, [System.Drawing.Imaging.ImageFormat]::Png)
    [Convert]::ToBase64String($stream.ToArray())
} finally {
    [KeynovaShellIcon]::DestroyIcon($info.hIcon) | Out-Null
}
"#;

pub fn search_icon_data_url(icon_key: &str, kind: &str, path: &str) -> Option<String> {
    if path.trim().is_empty() {
        return None;
    }
    if kind != "app" && kind != "file" && kind != "folder" {
        return None;
    }

    let cache = SEARCH_ICON_CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    if let Ok(guard) = cache.lock() {
        if let Some(cached) = guard.get(icon_key) {
            return cached.clone();
        }
    }

    if let Some(base64) = read_icon_disk_cache(icon_key) {
        let data_url = Some(format!("data:image/png;base64,{base64}"));
        if let Ok(mut guard) = cache.lock() {
            guard.insert(icon_key.to_string(), data_url.clone());
        }
        return data_url;
    }

    let base64 = extract_shell_icon_base64(path, kind);
    let data_url = base64
        .as_ref()
        .map(|b| format!("data:image/png;base64,{b}"));

    // Only persist successful extractions. Missing icons stay in the in-mem
    // None bucket so we don't repeat PowerShell calls this session, but a
    // restart re-tries them in case the source app installs an icon later.
    if let Some(b) = &base64 {
        write_icon_disk_cache(icon_key, b);
    }

    if let Ok(mut guard) = cache.lock() {
        guard.insert(icon_key.to_string(), data_url.clone());
    }

    data_url
}

// ─── Icon disk cache ────────────────────────────────────────────────────────
//
// On-disk PNG cache so PowerShell + SHGetFileInfo only runs once per icon_key
// across restarts. Files live under `dirs::cache_dir()/keynova/icons/<hash>.b64`,
// holding the same base64 payload PowerShell emits — no encode/decode round-trip
// on the hot path. icon_key already encodes path/extension changes (see
// handlers::search::icon_key_for_item), so renames/upgrades skip the cache
// naturally.

fn icon_cache_dir() -> Option<std::path::PathBuf> {
    let dir = dirs::cache_dir()?.join("keynova").join("icons");
    if !dir.exists() {
        std::fs::create_dir_all(&dir).ok()?;
    }
    Some(dir)
}

fn icon_cache_file_name(icon_key: &str) -> String {
    use sha2::{Digest, Sha256};
    let hash = Sha256::digest(icon_key.as_bytes());
    let mut name = String::with_capacity(16 + 4);
    for byte in &hash[..8] {
        use std::fmt::Write;
        let _ = write!(name, "{byte:02x}");
    }
    name.push_str(".b64");
    name
}

fn read_icon_disk_cache(icon_key: &str) -> Option<String> {
    let dir = icon_cache_dir()?;
    let file = dir.join(icon_cache_file_name(icon_key));
    let raw = std::fs::read_to_string(&file).ok()?;
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    Some(trimmed.to_string())
}

fn write_icon_disk_cache(icon_key: &str, base64: &str) {
    let Some(dir) = icon_cache_dir() else {
        return;
    };
    let file = dir.join(icon_cache_file_name(icon_key));
    let _ = std::fs::write(&file, base64);
}

fn extract_shell_icon_base64(path: &str, kind: &str) -> Option<String> {
    run_icon_script(path, kind, false)
}

fn run_icon_script(path: &str, kind: &str, use_attrs: bool) -> Option<String> {
    let mut cmd = std::process::Command::new("powershell.exe");
    cmd.args([
        "-NoProfile",
        "-NonInteractive",
        "-ExecutionPolicy",
        "Bypass",
        "-Command",
        SEARCH_ICON_SCRIPT,
    ])
    .env("KEYNOVA_ICON_PATH", path)
    .env("KEYNOVA_ICON_KIND", kind);
    if use_attrs {
        cmd.env("KEYNOVA_ICON_USE_ATTRS", "1");
    }
    let output = cmd.output().ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8(output.stdout).ok()?;
    let base64 = text.trim();
    if base64.is_empty() {
        return None;
    }
    Some(base64.to_string())
}

// ─── Icon cache pre-warm ────────────────────────────────────────────────────
//
// Cold-cache first-render jank fix: after app scan completes at startup, walk
// the candidate set and write disk-cache entries before the user opens the
// palette. icon_key shape (see handlers::search::icon_key_for_item):
//   - folder → "folder"
//   - file:{ext} → shared across all files of the same extension
//   - app:{hash} → one per .lnk path
//
// File-ext warming uses SHGFI_USEFILEATTRIBUTES so we get the system default
// icon for that extension without touching a real file (env-gated; runtime
// path still queries the actual file).

const WARM_FILE_EXTENSIONS: &[&str] = &[
    "txt", "md", "pdf", "doc", "docx", "xls", "xlsx", "ppt", "pptx", "png", "jpg", "jpeg", "gif",
    "mp4", "mp3", "zip", "rar", "7z", "exe", "rs", "py", "js", "ts", "tsx", "jsx", "json", "html",
    "css", "yaml", "yml", "toml",
];

/// Pre-warm the icon disk cache for folder + common file extensions + every
/// scanned app. Skips work when the on-disk cache already has the key. Safe to
/// run repeatedly; designed to be spawned on a background thread at startup.
pub fn warm_icon_cache() {
    warm_folder_icon();
    warm_file_ext_icons();
    warm_app_icons();
}

fn warm_folder_icon() {
    use crate::handlers::search::icon_key_for_item;
    use crate::models::search_result::ResultKind;
    let probe = dirs::home_dir()
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_else(|| "C:\\".to_string());
    let key = icon_key_for_item("warmcache", &probe, &ResultKind::Folder);
    if read_icon_disk_cache(&key).is_some() {
        return;
    }
    if let Some(base64) = run_icon_script(&probe, "folder", false) {
        write_icon_disk_cache(&key, &base64);
    }
}

fn warm_file_ext_icons() {
    use crate::handlers::search::icon_key_for_item;
    use crate::models::search_result::ResultKind;
    for ext in WARM_FILE_EXTENSIONS {
        let dummy = format!("probe.{ext}");
        let key = icon_key_for_item("warmcache", &dummy, &ResultKind::File);
        if read_icon_disk_cache(&key).is_some() {
            continue;
        }
        if let Some(base64) = run_icon_script(&dummy, "file", true) {
            write_icon_disk_cache(&key, &base64);
        }
    }
}

fn warm_app_icons() {
    use crate::handlers::search::icon_key_for_item;
    use crate::models::search_result::ResultKind;
    for app in super::apps::scan_applications() {
        let key = icon_key_for_item("warmcache", &app.path, &ResultKind::App);
        if read_icon_disk_cache(&key).is_some() {
            continue;
        }
        if let Some(base64) = run_icon_script(&app.path, "app", false) {
            write_icon_disk_cache(&key, &base64);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn icon_cache_file_name_is_stable_per_key() {
        let a = icon_cache_file_name("app:abc123");
        let b = icon_cache_file_name("app:abc123");
        assert_eq!(a, b);
    }

    #[test]
    fn icon_cache_file_name_differs_per_key() {
        let a = icon_cache_file_name("app:abc123");
        let b = icon_cache_file_name("app:abc124");
        assert_ne!(a, b);
    }

    #[test]
    fn icon_cache_file_name_has_b64_extension_and_hex_stem() {
        let name = icon_cache_file_name("file:rs");
        assert!(name.ends_with(".b64"), "expected .b64 suffix, got {name}");
        let stem = name.trim_end_matches(".b64");
        assert_eq!(stem.len(), 16, "expected 16-hex stem, got {stem}");
        assert!(
            stem.chars().all(|c| c.is_ascii_hexdigit()),
            "expected lowercase hex stem, got {stem}"
        );
    }

    #[test]
    fn warm_file_extensions_are_lowercase_unique_and_dotless() {
        use std::collections::HashSet;
        let set: HashSet<&&str> = WARM_FILE_EXTENSIONS.iter().collect();
        assert_eq!(
            set.len(),
            WARM_FILE_EXTENSIONS.len(),
            "duplicate extension in WARM_FILE_EXTENSIONS"
        );
        for ext in WARM_FILE_EXTENSIONS {
            assert!(!ext.is_empty(), "empty extension entry");
            assert!(
                !ext.starts_with('.'),
                "extension must not start with dot: {ext}"
            );
            assert_eq!(
                *ext,
                ext.to_ascii_lowercase(),
                "extension must be lowercase: {ext}"
            );
        }
    }

    #[test]
    fn warm_ext_keys_match_search_handler_format() {
        use crate::handlers::search::icon_key_for_item;
        use crate::models::search_result::ResultKind;
        for ext in WARM_FILE_EXTENSIONS {
            let dummy = format!("probe.{ext}");
            let key = icon_key_for_item("warmcache", &dummy, &ResultKind::File);
            assert_eq!(key, format!("file:{ext}"));
        }
    }
}
