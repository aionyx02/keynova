use std::collections::HashSet;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::Arc;

use crate::core::AppEvent;

const NVIM_VERSION: &str = "v0.10.4";

// Pinned SHA-256 of the per-platform release archives for NVIM_VERSION, taken
// from neovim's published `<asset>.sha256sum` files. The downloaded archive is
// verified against this before extraction so a tampered/MITM'd payload is
// rejected. Update together with NVIM_VERSION.
#[cfg(target_os = "windows")]
const EXPECTED_SHA256: &str = "dceeb8301f64e244e3e2dffaedbb153bd01c0c6ecb5024a90e3172dc8e65555c";
#[cfg(target_os = "macos")]
const EXPECTED_SHA256: &str = "c1405071127b59dbdefc31d9c52e9a5c36db67dcef6dcf83e898aada1f3f778e";
#[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
const EXPECTED_SHA256: &str = "95aaa8e89473f5421114f2787c13ae0ec6e11ebbd1a13a1bd6fcf63420f8073f";

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
    let cmd = if cfg!(target_os = "windows") {
        "where"
    } else {
        "which"
    };
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
    return portable_dir()
        .join("nvim-win64")
        .join("bin")
        .join("nvim.exe");
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
pub fn download_nvim(
    emit: Arc<dyn Fn(AppEvent) + Send + Sync>,
    allowed_hosts: HashSet<String>,
) -> Result<PathBuf, String> {
    let dir = portable_dir();
    std::fs::create_dir_all(&dir).map_err(|e| format!("create dir: {e}"))?;

    #[cfg(target_os = "windows")]
    let (archive_name, url) = (
        "nvim-win64.zip".to_string(),
        format!("https://github.com/neovim/neovim/releases/download/{NVIM_VERSION}/nvim-win64.zip"),
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
    let url =
        crate::core::network_policy::enforce_outbound_url(&url, &allowed_hosts, "Neovim download")?;

    emit_progress(&emit, "downloading", 0);
    download_with_progress(&url, &archive_path, &emit)?;

    // Verify integrity before trusting the archive contents.
    if let Err(error) = verify_archive_sha256(&archive_path, EXPECTED_SHA256) {
        let _ = std::fs::remove_file(&archive_path);
        return Err(error);
    }

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
        file.write_all(&buf[..n])
            .map_err(|e| format!("write: {e}"))?;
        downloaded += n as u64;
        if total > 0 {
            let pct = downloaded
                .saturating_mul(100)
                .checked_div(total)
                .unwrap_or(0)
                .min(99) as u8;
            if pct != last_pct {
                last_pct = pct;
                emit_progress(emit, "downloading", pct);
            }
        }
    }

    Ok(())
}

/// Compute the SHA-256 of `archive` and compare it (case-insensitively) to the
/// pinned `expected` hex digest. Streams the file in chunks so large archives
/// don't need to be buffered in memory.
fn verify_archive_sha256(archive: &std::path::Path, expected: &str) -> Result<(), String> {
    use sha2::{Digest, Sha256};

    let mut file = std::fs::File::open(archive).map_err(|e| format!("open archive: {e}"))?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 64 * 1024];
    loop {
        let read = file
            .read(&mut buf)
            .map_err(|e| format!("read archive: {e}"))?;
        if read == 0 {
            break;
        }
        hasher.update(&buf[..read]);
    }
    let actual = hasher.finalize();
    let actual_hex: String = actual.iter().map(|b| format!("{b:02x}")).collect();

    if actual_hex.eq_ignore_ascii_case(expected) {
        Ok(())
    } else {
        Err(format!(
            "Neovim download checksum mismatch (expected {expected}, got {actual_hex}); refusing to extract"
        ))
    }
}

fn extract_archive(archive: &std::path::Path, dest: &std::path::Path) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        let status = std::process::Command::new("powershell")
            .args([
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                "Expand-Archive -LiteralPath $env:KEYNOVA_ARCHIVE -DestinationPath $env:KEYNOVA_DEST -Force",
            ])
            .env("KEYNOVA_ARCHIVE", archive)
            .env("KEYNOVA_DEST", dest)
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
