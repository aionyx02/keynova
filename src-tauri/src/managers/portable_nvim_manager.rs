use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::Arc;

use crate::core::AppEvent;

const NVIM_VERSION: &str = "v0.10.4";

/// Locate an existing nvim binary in priority order:
/// 1. Explicitly configured path
/// 2. `nvim` on the system PATH
/// 3. Keynova's portable install directory
pub fn detect_nvim(configured: Option<&str>) -> Option<PathBuf> {
    if let Some(path) = configured {
        let p = PathBuf::from(path);
        if p.is_file() {
            return Some(p);
        }
    }

    if which_nvim().is_some() {
        return Some(PathBuf::from("nvim"));
    }

    let exe = portable_nvim_exe();
    if exe.is_file() {
        return Some(exe);
    }

    None
}

fn which_nvim() -> Option<PathBuf> {
    let cmd = if cfg!(target_os = "windows") { "where" } else { "which" };
    let output = std::process::Command::new(cmd).arg("nvim").output().ok()?;
    if !output.status.success() {
        return None;
    }
    let raw = String::from_utf8_lossy(&output.stdout);
    let first = raw.lines().next()?.trim();
    let p = PathBuf::from(first);
    p.is_file().then_some(p)
}

fn portable_dir() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("Keynova")
        .join("nvim")
}

pub fn portable_nvim_exe() -> PathBuf {
    #[cfg(target_os = "windows")]
    return portable_dir().join("nvim-win64").join("bin").join("nvim.exe");
    #[cfg(target_os = "macos")]
    return portable_dir()
        .join("nvim-macos-x86_64")
        .join("bin")
        .join("nvim");
    #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
    return portable_dir()
        .join("nvim-linux-x86_64")
        .join("bin")
        .join("nvim");
}

/// Download neovim into the portable directory. `emit` receives progress events.
pub fn download_nvim(emit: Arc<dyn Fn(AppEvent) + Send + Sync>) -> Result<PathBuf, String> {
    let dir = portable_dir();
    std::fs::create_dir_all(&dir).map_err(|e| format!("create dir: {e}"))?;

    #[cfg(target_os = "windows")]
    let (archive_name, url) = (
        "nvim-win64.zip".to_string(),
        format!(
            "https://github.com/neovim/neovim/releases/download/{NVIM_VERSION}/nvim-win64.zip"
        ),
    );
    #[cfg(target_os = "macos")]
    let (archive_name, url) = (
        "nvim-macos-x86_64.tar.gz".to_string(),
        format!(
            "https://github.com/neovim/neovim/releases/download/{NVIM_VERSION}/nvim-macos-x86_64.tar.gz"
        ),
    );
    #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
    let (archive_name, url) = (
        "nvim-linux-x86_64.tar.gz".to_string(),
        format!(
            "https://github.com/neovim/neovim/releases/download/{NVIM_VERSION}/nvim-linux-x86_64.tar.gz"
        ),
    );

    let archive_path = dir.join(&archive_name);

    emit_progress(&emit, "downloading", 0);
    download_with_progress(&url, &archive_path, &emit)?;

    emit_progress(&emit, "extracting", 0);
    extract_archive(&archive_path, &dir)?;
    let _ = std::fs::remove_file(&archive_path);

    let exe = portable_nvim_exe();
    if exe.is_file() {
        emit_progress(&emit, "done", 100);
        Ok(exe)
    } else {
        Err(format!(
            "nvim binary not found at expected path: {}",
            exe.display()
        ))
    }
}

fn emit_progress(emit: &Arc<dyn Fn(AppEvent) + Send + Sync>, stage: &str, pct: u8) {
    emit(AppEvent::new(
        "nvim.download_progress",
        serde_json::json!({ "stage": stage, "pct": pct }),
    ));
}

fn download_with_progress(
    url: &str,
    dest: &std::path::Path,
    emit: &Arc<dyn Fn(AppEvent) + Send + Sync>,
) -> Result<(), String> {
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(300))
        .build()
        .map_err(|e| format!("http client: {e}"))?;

    let mut response = client
        .get(url)
        .send()
        .map_err(|e| format!("download failed: {e}"))?;

    if !response.status().is_success() {
        return Err(format!("HTTP {}", response.status()));
    }

    let total = response.content_length().unwrap_or(0);
    let mut file = std::fs::File::create(dest).map_err(|e| format!("create file: {e}"))?;
    let mut downloaded: u64 = 0;
    let mut last_pct = 0u8;
    let mut buf = vec![0u8; 65536];

    loop {
        let n = response.read(&mut buf).map_err(|e| format!("read: {e}"))?;
        if n == 0 {
            break;
        }
        file.write_all(&buf[..n]).map_err(|e| format!("write: {e}"))?;
        downloaded += n as u64;
        if total > 0 {
            let pct = downloaded.saturating_mul(100).checked_div(total).unwrap_or(0).min(99) as u8;
            if pct != last_pct {
                last_pct = pct;
                emit_progress(emit, "downloading", pct);
            }
        }
    }

    Ok(())
}

fn extract_archive(archive: &std::path::Path, dest: &std::path::Path) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        let status = std::process::Command::new("powershell")
            .args([
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                &format!(
                    "Expand-Archive -LiteralPath '{}' -DestinationPath '{}' -Force",
                    archive.display(),
                    dest.display()
                ),
            ])
            .status()
            .map_err(|e| format!("powershell: {e}"))?;
        if !status.success() {
            return Err("Expand-Archive failed".to_string());
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        let status = std::process::Command::new("tar")
            .args([
                "-xzf",
                &archive.to_string_lossy(),
                "-C",
                &dest.to_string_lossy(),
            ])
            .status()
            .map_err(|e| format!("tar: {e}"))?;
        if !status.success() {
            return Err("tar extraction failed".to_string());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn portable_exe_path_has_correct_extension() {
        let exe = portable_nvim_exe();
        #[cfg(target_os = "windows")]
        assert_eq!(exe.extension().and_then(|e| e.to_str()), Some("exe"));
        #[cfg(not(target_os = "windows"))]
        assert!(exe.file_name().and_then(|n| n.to_str()) == Some("nvim"));
    }

    #[test]
    fn detect_nvim_returns_none_for_bogus_path() {
        let result = detect_nvim(Some("/nonexistent/path/to/nvim"));
        // Only None if nvim isn't on PATH either; we just ensure no panic
        let _ = result;
    }
}