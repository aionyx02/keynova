//! Shared bounded file-preview helpers used by both `file.preview` (LAUNCH.1.C
//! search preview pane) and `learning_material.preview` (FEAT.11 review tool).
//!
//! All public functions are pure / take no manager state so they can be unit
//! tested in isolation. Path canonicalization, denylist, and permission checks
//! happen at the call site — this module only reads what it is told to read.

use std::fs;
use std::io::Read;
use std::path::Path;

use crate::core::{prepare_observation, AgentObservationPolicy};

/// Image extensions recognised by `classify_path`. Lowercased, no leading dot.
const IMAGE_EXTENSIONS: &[&str] = &["png", "jpg", "jpeg", "gif", "webp", "bmp", "svg", "ico"];

/// Text extensions that bypass content sniffing. Lowercased, no leading dot.
const TEXT_EXTENSIONS: &[&str] = &[
    "txt", "md", "log", "json", "toml", "yaml", "yml", "rs", "ts", "tsx", "js", "jsx", "py", "go",
    "java", "kt", "swift", "c", "cpp", "h", "hpp", "sh", "bat", "ps1", "css", "html", "xml", "csv",
    "ini", "conf", "rst", "org", "wiki", "adoc",
];

/// Coarse content kind used by the preview pane.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreviewKind {
    Text,
    Image,
    Binary,
}

impl PreviewKind {
    pub fn as_str(self) -> &'static str {
        match self {
            PreviewKind::Text => "text",
            PreviewKind::Image => "image",
            PreviewKind::Binary => "binary",
        }
    }
}

/// Classify a path by extension first, falling back to a printable-byte sniff
/// when the extension is unrecognised.
pub fn classify_path(path: &Path, meta: &fs::Metadata) -> PreviewKind {
    if !meta.is_file() {
        return PreviewKind::Binary;
    }
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase)
        .unwrap_or_default();

    if IMAGE_EXTENSIONS.contains(&ext.as_str()) {
        return PreviewKind::Image;
    }
    if TEXT_EXTENSIONS.contains(&ext.as_str()) {
        return PreviewKind::Text;
    }

    // Sniff first 512 bytes: if >5% are non-printable / non-whitespace, treat as binary.
    let mut buf = [0u8; 512];
    let n = match fs::File::open(path).and_then(|mut f| f.read(&mut buf)) {
        Ok(n) => n,
        Err(_) => return PreviewKind::Binary,
    };
    if n == 0 {
        return PreviewKind::Text;
    }
    let non_printable = buf[..n]
        .iter()
        .filter(|b| !(b.is_ascii_graphic() || b.is_ascii_whitespace()))
        .count();
    if non_printable * 20 > n {
        PreviewKind::Binary
    } else {
        PreviewKind::Text
    }
}

/// Best-effort MIME guess for image files (extension based).
pub fn guess_image_mime(path: &Path) -> &'static str {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase)
        .unwrap_or_default();
    match ext.as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "bmp" => "image/bmp",
        "svg" => "image/svg+xml",
        "ico" => "image/x-icon",
        _ => "application/octet-stream",
    }
}

/// Result of a bounded text preview read.
pub struct TextPreview {
    pub content: String,
    pub truncated: bool,
    pub line_count: usize,
}

/// Read up to `max_bytes` of `path`, redact secrets, and clamp to `max_lines`.
///
/// Returns `Err` for I/O failures. UTF-8 detection failure yields a binary-tag
/// content string (`[binary: preview not available]`) with `truncated=false` so
/// the caller can decide whether to surface it as text or downgrade to binary.
pub fn read_text_preview(
    path: &Path,
    max_bytes: usize,
    max_lines: usize,
    redact_secrets: bool,
) -> Result<TextPreview, String> {
    let mut buf = vec![0u8; max_bytes];
    let n = fs::File::open(path)
        .and_then(|mut f| f.read(&mut buf))
        .map_err(|e| format!("read failed: {e}"))?;
    let raw = match std::str::from_utf8(&buf[..n]) {
        Ok(text) => text,
        Err(_) => {
            return Ok(TextPreview {
                content: "[binary: preview not available]".to_string(),
                truncated: false,
                line_count: 0,
            });
        }
    };
    let policy = AgentObservationPolicy {
        max_chars: max_bytes,
        max_lines,
        preserve_head_lines: max_lines.saturating_sub(20).max(1),
        preserve_tail_lines: 20.min(max_lines),
        redact_secrets,
    };
    let prepared = prepare_observation(raw, &policy);
    let line_count = prepared.content.lines().count();
    // Treat as truncated when the file had more bytes than we read OR observation truncated lines/chars.
    let file_truncated = (n as u64) < fs::metadata(path).map(|m| m.len()).unwrap_or(n as u64);
    Ok(TextPreview {
        content: prepared.content,
        truncated: prepared.truncated || file_truncated,
        line_count,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    fn write_tmp(dir: &TempDir, name: &str, content: &[u8]) -> std::path::PathBuf {
        let path = dir.path().join(name);
        let mut f = fs::File::create(&path).unwrap();
        f.write_all(content).unwrap();
        path
    }

    #[test]
    fn classify_text_by_extension() {
        let dir = TempDir::new().unwrap();
        let path = write_tmp(&dir, "a.md", b"# hello");
        let meta = fs::metadata(&path).unwrap();
        assert_eq!(classify_path(&path, &meta), PreviewKind::Text);
    }

    #[test]
    fn classify_image_by_extension() {
        let dir = TempDir::new().unwrap();
        let path = write_tmp(&dir, "a.png", b"\x89PNG\r\n\x1a\n");
        let meta = fs::metadata(&path).unwrap();
        assert_eq!(classify_path(&path, &meta), PreviewKind::Image);
    }

    #[test]
    fn classify_binary_by_sniff() {
        let dir = TempDir::new().unwrap();
        // 256 bytes of random binary (high non-printable ratio) with no extension.
        let mut content = Vec::with_capacity(256);
        for i in 0..256u32 {
            content.push((i % 256) as u8);
        }
        let path = write_tmp(&dir, "blob", &content);
        let meta = fs::metadata(&path).unwrap();
        assert_eq!(classify_path(&path, &meta), PreviewKind::Binary);
    }

    #[test]
    fn classify_unknown_text_by_sniff() {
        let dir = TempDir::new().unwrap();
        let path = write_tmp(&dir, "no_ext", b"this is plain ascii content for sniffing");
        let meta = fs::metadata(&path).unwrap();
        assert_eq!(classify_path(&path, &meta), PreviewKind::Text);
    }

    #[test]
    fn guess_mime_covers_common_formats() {
        assert_eq!(guess_image_mime(Path::new("a.png")), "image/png");
        assert_eq!(guess_image_mime(Path::new("a.JPG")), "image/jpeg");
        assert_eq!(guess_image_mime(Path::new("a.svg")), "image/svg+xml");
        assert_eq!(
            guess_image_mime(Path::new("a.bin")),
            "application/octet-stream"
        );
    }

    #[test]
    fn read_text_preview_returns_content() {
        let dir = TempDir::new().unwrap();
        let path = write_tmp(&dir, "a.txt", b"line1\nline2\nline3\n");
        let preview = read_text_preview(&path, 4096, 500, true).unwrap();
        assert!(preview.content.contains("line1"));
        assert!(preview.content.contains("line3"));
        assert_eq!(preview.truncated, false);
    }

    #[test]
    fn read_text_preview_truncates_when_file_larger_than_cap() {
        let dir = TempDir::new().unwrap();
        let body: String = (0..2000).map(|i| format!("line {i}\n")).collect();
        let path = write_tmp(&dir, "big.txt", body.as_bytes());
        let preview = read_text_preview(&path, 256, 50, true).unwrap();
        assert!(preview.truncated, "expected truncation flag set");
    }

    #[test]
    fn read_text_preview_handles_binary_as_label() {
        let dir = TempDir::new().unwrap();
        let path = write_tmp(&dir, "a.bin", &[0xff, 0xfe, 0xfd, 0xfc, 0xfb]);
        let preview = read_text_preview(&path, 4096, 100, true).unwrap();
        assert!(preview.content.contains("binary"));
    }
}
