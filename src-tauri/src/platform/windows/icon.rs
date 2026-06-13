//! Shell icon extraction (native `SHGetFileInfo` + GDI `GetDIBits`, PNG-encoded
//! via the `png` crate) with in-memory + on-disk PNG caching.
//!
//! Replaces the former per-icon `powershell.exe` shell-out (ADR-0056): no child
//! process, so no console flash and no per-row spawn latency on a cold cache.
//!
//! Focused Windows icon helpers re-exported by the platform facade.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

static SEARCH_ICON_CACHE: OnceLock<Mutex<HashMap<String, Option<String>>>> = OnceLock::new();

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
    extract_icon_base64(path, kind, false)
}

// ─── Native shell-icon extraction (ADR-0056) ────────────────────────────────
//
// SHGetFileInfoW → HICON → GetIconInfo → GetDIBits (32bpp top-down BGRA) → PNG.
// `use_attrs` sets SHGFI_USEFILEATTRIBUTES so warming can probe a file extension
// from a dummy path without touching disk; the runtime path queries the real
// file. Returns base64-encoded PNG (same payload the old PowerShell path emitted).

/// Base64-encoded PNG for the shell icon of `path`, or `None` if extraction fails.
fn extract_icon_base64(path: &str, kind: &str, use_attrs: bool) -> Option<String> {
    use base64::Engine;
    let png = native_shell_icon_png(path, kind, use_attrs)?;
    Some(base64::engine::general_purpose::STANDARD.encode(png))
}

fn native_shell_icon_png(path: &str, kind: &str, use_attrs: bool) -> Option<Vec<u8>> {
    use windows::core::PCWSTR;
    use windows::Win32::Storage::FileSystem::FILE_FLAGS_AND_ATTRIBUTES;
    use windows::Win32::UI::Shell::{
        SHGetFileInfoW, SHFILEINFOW, SHGFI_ICON, SHGFI_LARGEICON, SHGFI_USEFILEATTRIBUTES,
    };
    use windows::Win32::UI::WindowsAndMessaging::DestroyIcon;

    let wide: Vec<u16> = path.encode_utf16().chain(std::iter::once(0)).collect();
    let attrs: u32 = match kind {
        "folder" => 0x10, // FILE_ATTRIBUTE_DIRECTORY
        "file" => 0x80,   // FILE_ATTRIBUTE_NORMAL
        _ => 0,
    };
    let mut flags = SHGFI_ICON | SHGFI_LARGEICON;
    if use_attrs {
        flags |= SHGFI_USEFILEATTRIBUTES;
    }

    let mut info = SHFILEINFOW::default();
    let ret = unsafe {
        SHGetFileInfoW(
            PCWSTR(wide.as_ptr()),
            FILE_FLAGS_AND_ATTRIBUTES(attrs),
            Some(&mut info),
            std::mem::size_of::<SHFILEINFOW>() as u32,
            flags,
        )
    };
    if ret == 0 || info.hIcon.is_invalid() {
        return None;
    }
    let png = hicon_to_png(info.hIcon);
    unsafe {
        let _ = DestroyIcon(info.hIcon);
    }
    png
}

fn hicon_to_png(hicon: windows::Win32::UI::WindowsAndMessaging::HICON) -> Option<Vec<u8>> {
    use windows::Win32::Graphics::Gdi::{DeleteObject, GetObjectW, HGDIOBJ, BITMAP};
    use windows::Win32::UI::WindowsAndMessaging::{GetIconInfo, ICONINFO};

    let mut ii = ICONINFO::default();
    unsafe { GetIconInfo(hicon, &mut ii) }.ok()?;
    let hbm_color = ii.hbmColor;
    let hbm_mask = ii.hbmMask;
    let cleanup = || unsafe {
        if !hbm_color.is_invalid() {
            let _ = DeleteObject(HGDIOBJ(hbm_color.0));
        }
        if !hbm_mask.is_invalid() {
            let _ = DeleteObject(HGDIOBJ(hbm_mask.0));
        }
    };

    let mut bmp = BITMAP::default();
    let got = unsafe {
        GetObjectW(
            HGDIOBJ(hbm_color.0),
            std::mem::size_of::<BITMAP>() as i32,
            Some(&mut bmp as *mut _ as *mut core::ffi::c_void),
        )
    };
    if got == 0 || bmp.bmWidth <= 0 || bmp.bmHeight <= 0 {
        cleanup();
        return None;
    }
    let width = bmp.bmWidth as u32;
    let height = bmp.bmHeight as u32;

    let color = get_dib_bgra(hbm_color, width, height);
    let mask = get_dib_bgra(hbm_mask, width, height);
    cleanup();

    let color = color?;
    let rgba = bgra_to_rgba(width, height, &color, mask.as_deref());
    encode_png_rgba(width, height, &rgba)
}

fn get_dib_bgra(
    hbm: windows::Win32::Graphics::Gdi::HBITMAP,
    width: u32,
    height: u32,
) -> Option<Vec<u8>> {
    use windows::Win32::Graphics::Gdi::{
        GetDC, GetDIBits, ReleaseDC, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS,
    };

    if hbm.is_invalid() {
        return None;
    }
    let hdc = unsafe { GetDC(None) };
    if hdc.is_invalid() {
        return None;
    }
    let mut bmi = BITMAPINFO::default();
    bmi.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
    bmi.bmiHeader.biWidth = width as i32;
    bmi.bmiHeader.biHeight = -(height as i32); // top-down
    bmi.bmiHeader.biPlanes = 1;
    bmi.bmiHeader.biBitCount = 32;
    bmi.bmiHeader.biCompression = BI_RGB.0;

    let mut buf = vec![0u8; (width * height * 4) as usize];
    let lines = unsafe {
        GetDIBits(
            hdc,
            hbm,
            0,
            height,
            Some(buf.as_mut_ptr() as *mut core::ffi::c_void),
            &mut bmi,
            DIB_RGB_COLORS,
        )
    };
    unsafe {
        ReleaseDC(None, hdc);
    }
    if lines == 0 {
        None
    } else {
        Some(buf)
    }
}

/// Convert 32bpp top-down BGRA (as `GetDIBits` returns it) to RGBA. If the color
/// bitmap carries no alpha (all-zero alpha — older icons), recover transparency
/// from the AND mask (`mask` black = opaque, non-black = transparent).
fn bgra_to_rgba(width: u32, height: u32, color_bgra: &[u8], mask_bgra: Option<&[u8]>) -> Vec<u8> {
    let n = (width as usize) * (height as usize);
    let mut rgba = vec![0u8; n * 4];
    let has_alpha = color_bgra.chunks_exact(4).any(|p| p[3] != 0);
    for i in 0..n {
        let b = color_bgra[i * 4];
        let g = color_bgra[i * 4 + 1];
        let r = color_bgra[i * 4 + 2];
        let a = if has_alpha {
            color_bgra[i * 4 + 3]
        } else if let Some(mask) = mask_bgra {
            if mask[i * 4] == 0 {
                255
            } else {
                0
            }
        } else {
            255
        };
        rgba[i * 4] = r;
        rgba[i * 4 + 1] = g;
        rgba[i * 4 + 2] = b;
        rgba[i * 4 + 3] = a;
    }
    rgba
}

fn encode_png_rgba(width: u32, height: u32, rgba: &[u8]) -> Option<Vec<u8>> {
    let mut out = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut out, width, height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().ok()?;
        writer.write_image_data(rgba).ok()?;
    }
    Some(out)
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
    if let Some(base64) = extract_icon_base64(&probe, "folder", false) {
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
        if let Some(base64) = extract_icon_base64(&dummy, "file", true) {
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
        if let Some(base64) = extract_icon_base64(&app.path, "app", false) {
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

    #[test]
    fn bgra_to_rgba_swaps_channels_and_keeps_alpha() {
        // one pixel: B=10 G=20 R=30 A=40
        let bgra = [10u8, 20, 30, 40];
        let rgba = bgra_to_rgba(1, 1, &bgra, None);
        assert_eq!(rgba, vec![30, 20, 10, 40]);
    }

    #[test]
    fn bgra_to_rgba_recovers_alpha_from_mask_when_color_has_none() {
        // two pixels, color alpha all zero -> must use mask.
        // mask pixel 0 = black (opaque -> 255), pixel 1 = white (transparent -> 0).
        let color = [1u8, 2, 3, 0, 4, 5, 6, 0];
        let mask = [0u8, 0, 0, 0, 255, 255, 255, 255];
        let rgba = bgra_to_rgba(2, 1, &color, Some(&mask));
        assert_eq!(rgba[3], 255, "mask-black pixel should be opaque");
        assert_eq!(rgba[7], 0, "mask-white pixel should be transparent");
    }

    #[test]
    fn bgra_to_rgba_prefers_real_alpha_over_mask() {
        // color carries alpha -> mask is ignored.
        let color = [0u8, 0, 0, 128];
        let mask = [255u8, 255, 255, 255]; // would force transparent if used
        let rgba = bgra_to_rgba(1, 1, &color, Some(&mask));
        assert_eq!(rgba[3], 128);
    }

    #[test]
    fn encode_png_rgba_emits_valid_png_signature() {
        let rgba = [255u8, 0, 0, 255, 0, 255, 0, 255]; // 2x1
        let png = encode_png_rgba(2, 1, &rgba).expect("encode");
        assert_eq!(&png[..8], &[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]);
    }
}
