//! Shared bounded file-preview helpers used by both `file.preview` and
//! `learning_material.preview`.
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

const MAX_DOCX_XML_BYTES: u64 = 2 * 1024 * 1024;

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

pub fn is_docx_path(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|ext| ext.eq_ignore_ascii_case("docx"))
        .unwrap_or(false)
}

/// Extract a bounded text preview from the main body of an Office Open XML
/// `.docx`. The file is a zip; `word/document.xml` carries paragraph text in
/// `<w:t>` nodes. We intentionally avoid a heavy XML parser here because preview
/// only needs plain text and must stay on the fast path.
pub fn read_docx_preview(
    path: &Path,
    max_bytes: usize,
    max_lines: usize,
    redact_secrets: bool,
) -> Result<TextPreview, String> {
    let file = fs::File::open(path).map_err(|e| format!("read docx failed: {e}"))?;
    let mut archive =
        zip::ZipArchive::new(file).map_err(|e| format!("invalid docx archive: {e}"))?;
    let document = archive
        .by_name("word/document.xml")
        .map_err(|e| format!("docx body missing: {e}"))?;

    let mut xml = String::new();
    let mut limited = document.take(MAX_DOCX_XML_BYTES + 1);
    limited
        .read_to_string(&mut xml)
        .map_err(|e| format!("read docx body failed: {e}"))?;
    let xml_truncated = xml.len() as u64 > MAX_DOCX_XML_BYTES;
    if xml_truncated {
        xml.truncate(MAX_DOCX_XML_BYTES as usize);
    }

    let extracted = extract_docx_plain_text(&xml, max_bytes.saturating_mul(4).max(4096), max_lines);
    let content = if extracted.content.trim().is_empty() {
        "[docx: no text found]".to_string()
    } else {
        extracted.content
    };

    let policy = AgentObservationPolicy {
        max_chars: max_bytes,
        max_lines,
        preserve_head_lines: max_lines.saturating_sub(20).max(1),
        preserve_tail_lines: 20.min(max_lines),
        redact_secrets,
    };
    let prepared = prepare_observation(&content, &policy);
    let line_count = prepared.content.lines().count();

    Ok(TextPreview {
        content: prepared.content,
        truncated: prepared.truncated || extracted.truncated || xml_truncated,
        line_count,
    })
}

struct ExtractedDocxText {
    content: String,
    truncated: bool,
}

fn extract_docx_plain_text(xml: &str, max_chars: usize, max_lines: usize) -> ExtractedDocxText {
    let mut out = String::new();
    let mut truncated = false;
    let mut lines = 0usize;

    for paragraph in xml.split("</w:p>") {
        let para = extract_docx_paragraph_text(paragraph);
        let para = para.trim();
        if para.is_empty() {
            continue;
        }
        if lines >= max_lines {
            truncated = true;
            break;
        }
        if !out.is_empty() {
            out.push('\n');
        }
        lines += 1;
        push_bounded(&mut out, para, max_chars, &mut truncated);
        if truncated {
            break;
        }
    }

    ExtractedDocxText {
        content: out,
        truncated,
    }
}

fn extract_docx_paragraph_text(xml: &str) -> String {
    let mut out = String::new();
    let mut cursor = 0usize;

    while let Some(tag_start_rel) = xml[cursor..].find('<') {
        let tag_start = cursor + tag_start_rel;
        let Some(tag_end_rel) = xml[tag_start..].find('>') else {
            break;
        };
        let tag_end = tag_start + tag_end_rel;
        let tag = &xml[tag_start + 1..tag_end];

        if tag == "w:tab/" || tag.starts_with("w:tab ") || tag.starts_with("w:tab/") {
            out.push('\t');
            cursor = tag_end + 1;
            continue;
        }
        if tag == "w:br/" || tag == "w:cr/" || tag.starts_with("w:br ") || tag.starts_with("w:cr ")
        {
            out.push('\n');
            cursor = tag_end + 1;
            continue;
        }
        if tag == "w:t" || tag.starts_with("w:t ") {
            let content_start = tag_end + 1;
            let Some(close_rel) = xml[content_start..].find("</w:t>") else {
                break;
            };
            let close = content_start + close_rel;
            out.push_str(&decode_xml_entities(&xml[content_start..close]));
            cursor = close + "</w:t>".len();
            continue;
        }

        cursor = tag_end + 1;
    }

    out
}

fn push_bounded(out: &mut String, text: &str, max_chars: usize, truncated: &mut bool) {
    let remaining = max_chars.saturating_sub(out.chars().count());
    if remaining == 0 {
        *truncated = true;
        return;
    }
    for (idx, ch) in text.chars().enumerate() {
        if idx >= remaining {
            *truncated = true;
            break;
        }
        out.push(ch);
    }
}

fn decode_xml_entities(s: &str) -> String {
    if !s.contains('&') {
        return s.to_string();
    }
    let mut out = String::with_capacity(s.len());
    let mut rest = s;

    while let Some(idx) = rest.find('&') {
        out.push_str(&rest[..idx]);
        rest = &rest[idx + 1..];
        let Some(end) = rest.find(';') else {
            out.push('&');
            out.push_str(rest);
            return out;
        };
        let entity = &rest[..end];
        match entity {
            "amp" => out.push('&'),
            "lt" => out.push('<'),
            "gt" => out.push('>'),
            "quot" => out.push('"'),
            "apos" => out.push('\''),
            _ => {
                if let Some(decoded) = decode_numeric_xml_entity(entity) {
                    out.push(decoded);
                } else {
                    out.push('&');
                    out.push_str(entity);
                    out.push(';');
                }
            }
        }
        rest = &rest[end + 1..];
    }
    out.push_str(rest);
    out
}

fn decode_numeric_xml_entity(entity: &str) -> Option<char> {
    let value = if let Some(hex) = entity
        .strip_prefix("#x")
        .or_else(|| entity.strip_prefix("#X"))
    {
        u32::from_str_radix(hex, 16).ok()?
    } else {
        let decimal = entity.strip_prefix('#')?;
        decimal.parse::<u32>().ok()?
    };
    char::from_u32(value)
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
    use zip::write::SimpleFileOptions;

    fn write_tmp(dir: &TempDir, name: &str, content: &[u8]) -> std::path::PathBuf {
        let path = dir.path().join(name);
        let mut f = fs::File::create(&path).unwrap();
        f.write_all(content).unwrap();
        path
    }

    fn write_docx(dir: &TempDir, name: &str, document_xml: &str) -> std::path::PathBuf {
        let path = dir.path().join(name);
        let file = fs::File::create(&path).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        let options =
            SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
        zip.start_file("[Content_Types].xml", options).unwrap();
        zip.write_all(br#"<?xml version="1.0"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"/>"#)
            .unwrap();
        zip.start_file("word/document.xml", options).unwrap();
        zip.write_all(document_xml.as_bytes()).unwrap();
        zip.finish().unwrap();
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
        assert!(!preview.truncated);
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

    #[test]
    fn extract_docx_plain_text_preserves_paragraphs_and_entities() {
        let xml = r#"
            <w:document>
              <w:body>
                <w:p><w:r><w:t>Hello &amp; goodbye</w:t></w:r></w:p>
                <w:p><w:r><w:t>Second</w:t><w:tab/><w:t>line &#x2713;</w:t></w:r></w:p>
              </w:body>
            </w:document>
        "#;
        let text = extract_docx_plain_text(xml, 4096, 20);
        assert_eq!(text.content, "Hello & goodbye\nSecond\tline ✓");
        assert!(!text.truncated);
    }

    #[test]
    fn read_docx_preview_returns_text_content() {
        let dir = TempDir::new().unwrap();
        let path = write_docx(
            &dir,
            "report.docx",
            r#"
            <w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
              <w:body>
                <w:p><w:r><w:t>Project brief</w:t></w:r></w:p>
                <w:p><w:r><w:t>Ship the preview fix</w:t></w:r></w:p>
              </w:body>
            </w:document>
            "#,
        );
        let preview = read_docx_preview(&path, 4096, 100, true).unwrap();
        assert!(preview.content.contains("Project brief"));
        assert!(preview.content.contains("Ship the preview fix"));
        assert_eq!(preview.line_count, 2);
        assert!(!preview.truncated);
    }
}
