//! Icon key derivation and SVG fallback rendering for search results.
//!
//! Focused icon helpers used by the search handler facade.

use crate::models::search_result::ResultKind;

pub(crate) fn icon_key_for_item(source: &str, path: &str, kind: &ResultKind) -> String {
    match kind {
        ResultKind::App => format!("app:{}", stable_hash(path)),
        ResultKind::File => {
            let ext = std::path::Path::new(path)
                .extension()
                .and_then(|ext| ext.to_str())
                .unwrap_or("file")
                .to_ascii_lowercase();
            format!("file:{ext}")
        }
        ResultKind::Folder => "folder".into(),
        _ => source.to_string(),
    }
}

pub(super) fn icon_label(icon_key: &str, kind: &str, path: &str) -> String {
    if let Some(ext) = icon_key.strip_prefix("file:") {
        return ext.chars().take(3).collect::<String>().to_uppercase();
    }
    match kind {
        "app" => "APP".into(),
        "folder" => "DIR".into(),
        "command" => "CMD".into(),
        "note" => "MD".into(),
        "history" => "HIS".into(),
        "model" => "AI".into(),
        _ => std::path::Path::new(path)
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.chars().take(3).collect::<String>().to_uppercase())
            .unwrap_or_else(|| "KEY".into()),
    }
}

pub(super) fn icon_color(icon_key: &str, kind: &str) -> &'static str {
    if icon_key.starts_with("file:") {
        return "#0ea5e9";
    }
    match kind {
        "app" => "#8b5cf6",
        "folder" => "#f59e0b",
        "command" => "#10b981",
        "note" => "#14b8a6",
        "history" => "#71717a",
        "model" => "#d946ef",
        _ => "#64748b",
    }
}

pub(super) fn svg_data_url(label: &str, color: &str) -> String {
    let safe_label = escape_xml(label);
    let svg = format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="32" height="32" viewBox="0 0 32 32"><rect width="32" height="32" rx="7" fill="{color}"/><text x="16" y="20" text-anchor="middle" font-family="Segoe UI,Arial,sans-serif" font-size="10" font-weight="700" fill="#fff">{safe_label}</text></svg>"##
    );
    format!("data:image/svg+xml;utf8,{}", percent_encode(&svg))
}

fn escape_xml(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn percent_encode(value: &str) -> String {
    value
        .bytes()
        .map(|byte| match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                (byte as char).to_string()
            }
            _ => format!("%{byte:02X}"),
        })
        .collect()
}

fn stable_hash(value: &str) -> u64 {
    let mut h: u64 = 14695981039346656037;
    for b in value.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(1099511628211);
    }
    h
}
