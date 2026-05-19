//! UTIL.2 dev utility primitives — pure functions consumed by the
//! `handlers/dev_utils_cmd.rs` BuiltinCommand wrappers.
//!
//! All functions are stateless. Errors are surfaced as `String` for direct
//! display in the inline command result.

use base64::Engine;
use chrono::{Local, TimeZone, Utc};
use md5::{Digest as Md5Digest, Md5};
use rand::seq::SliceRandom;
use rand::Rng;
use sha1::Sha1;
use sha2::{Sha256, Sha512};
use std::str::FromStr;
use uuid::Uuid;

// ─── uuid / nanoid ───────────────────────────────────────────────────────────

pub fn uuid_v4() -> String {
    Uuid::new_v4().to_string()
}

/// Generate a nanoid-style ID using URL-safe alphabet (`A-Za-z0-9_-`).
/// Length defaults to 21 when input is 0 or invalid.
pub fn nanoid(length: usize) -> String {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789_-";
    let len = if length == 0 || length > 256 { 21 } else { length };
    let mut rng = rand::thread_rng();
    (0..len)
        .map(|_| ALPHABET[rng.gen_range(0..ALPHABET.len())] as char)
        .collect()
}

// ─── password ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PasswordMode {
    All,
    Alnum,
    Alpha,
}

impl FromStr for PasswordMode {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_lowercase().as_str() {
            "" | "sym" | "all" => Ok(Self::All),
            "alnum" => Ok(Self::Alnum),
            "alpha" => Ok(Self::Alpha),
            other => Err(format!("unknown password mode '{other}' (sym|alnum|alpha)")),
        }
    }
}

pub fn generate_password(length: usize, mode: PasswordMode) -> String {
    let len = if length == 0 || length > 1024 { 16 } else { length };
    let alphabet: &[u8] = match mode {
        PasswordMode::All => b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789!@#$%^&*()-_=+[]{}<>?",
        PasswordMode::Alnum => b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789",
        PasswordMode::Alpha => b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz",
    };
    let mut rng = rand::thread_rng();
    let mut chars: Vec<u8> = (0..len)
        .map(|_| *alphabet.choose(&mut rng).expect("alphabet non-empty"))
        .collect();
    // Shuffle once more for paranoia (alphabet may have grouped char classes).
    chars.shuffle(&mut rng);
    String::from_utf8(chars).expect("ASCII-only alphabet")
}

// ─── hash ────────────────────────────────────────────────────────────────────

pub fn hash_text(algo: &str, text: &str) -> Result<String, String> {
    match algo.to_lowercase().as_str() {
        "md5" => {
            let mut h = Md5::new();
            h.update(text.as_bytes());
            Ok(format!("{:x}", h.finalize()))
        }
        "sha1" => {
            let mut h = Sha1::new();
            h.update(text.as_bytes());
            Ok(format!("{:x}", h.finalize()))
        }
        "sha256" => {
            let mut h = Sha256::new();
            h.update(text.as_bytes());
            Ok(format!("{:x}", h.finalize()))
        }
        "sha512" => {
            let mut h = Sha512::new();
            h.update(text.as_bytes());
            Ok(format!("{:x}", h.finalize()))
        }
        other => Err(format!("unsupported hash algorithm '{other}' (md5|sha1|sha256|sha512)")),
    }
}

// ─── base64 / url encoding ───────────────────────────────────────────────────

pub fn b64_encode(input: &str) -> String {
    base64::engine::general_purpose::STANDARD.encode(input.as_bytes())
}

pub fn b64_decode(input: &str) -> Result<String, String> {
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(input.trim().as_bytes())
        .map_err(|e| format!("invalid base64: {e}"))?;
    String::from_utf8(bytes).map_err(|_| "decoded bytes are not valid UTF-8".into())
}

pub fn url_encode(input: &str) -> String {
    urlencoding::encode(input).into_owned()
}

pub fn url_decode(input: &str) -> Result<String, String> {
    urlencoding::decode(input)
        .map(|cow| cow.into_owned())
        .map_err(|e| format!("invalid percent-encoding: {e}"))
}

// ─── JSON pretty / minify ────────────────────────────────────────────────────

pub fn json_pretty(input: &str) -> Result<String, String> {
    let value: serde_json::Value =
        serde_json::from_str(input).map_err(|e| format!("invalid JSON: {e}"))?;
    serde_json::to_string_pretty(&value).map_err(|e| e.to_string())
}

pub fn json_minify(input: &str) -> Result<String, String> {
    let value: serde_json::Value =
        serde_json::from_str(input).map_err(|e| format!("invalid JSON: {e}"))?;
    serde_json::to_string(&value).map_err(|e| e.to_string())
}

// ─── regex test ──────────────────────────────────────────────────────────────

/// Returns a human-readable report listing each match's index, range, and
/// captured groups (up to first 20 matches).
pub fn regex_test(pattern: &str, text: &str) -> Result<String, String> {
    let re = regex::Regex::new(pattern).map_err(|e| format!("invalid regex: {e}"))?;
    let mut out = String::new();
    let mut count = 0usize;
    for cap in re.captures_iter(text).take(20) {
        let whole = cap.get(0).expect("capture 0 always present");
        out.push_str(&format!(
            "[{}..{}] {}",
            whole.start(),
            whole.end(),
            whole.as_str()
        ));
        for (i, group) in cap.iter().enumerate().skip(1) {
            if let Some(m) = group {
                out.push_str(&format!("\n  ${} = {}", i, m.as_str()));
            }
        }
        out.push('\n');
        count += 1;
    }
    if count == 0 {
        Ok("(no matches)".into())
    } else {
        out.push_str(&format!("\n{} match(es)", count));
        Ok(out)
    }
}

// ─── JWT decode (no signature verification) ──────────────────────────────────

pub fn jwt_decode(token: &str) -> Result<String, String> {
    let parts: Vec<&str> = token.trim().split('.').collect();
    if parts.len() != 3 {
        return Err("invalid JWT: expected 3 parts separated by '.'".into());
    }
    let header = decode_jwt_part(parts[0]).map_err(|e| format!("header: {e}"))?;
    let payload = decode_jwt_part(parts[1]).map_err(|e| format!("payload: {e}"))?;

    let mut out = String::new();
    out.push_str("Header:\n");
    out.push_str(&header);
    out.push_str("\n\nPayload:\n");
    out.push_str(&payload);

    // Expiry hint if `exp` claim present.
    if let Ok(payload_value) = serde_json::from_str::<serde_json::Value>(&payload) {
        if let Some(exp) = payload_value.get("exp").and_then(|v| v.as_i64()) {
            let now = chrono::Utc::now().timestamp();
            let exp_dt = Utc.timestamp_opt(exp, 0).single();
            out.push_str("\n\n");
            if exp < now {
                out.push_str(&format!(
                    "⚠️ Expired {} seconds ago",
                    now.saturating_sub(exp)
                ));
            } else {
                out.push_str(&format!(
                    "Expires in {} seconds",
                    exp.saturating_sub(now)
                ));
            }
            if let Some(dt) = exp_dt {
                out.push_str(&format!(" (at {})", dt.format("%Y-%m-%d %H:%M:%S UTC")));
            }
        }
    }
    Ok(out)
}

fn decode_jwt_part(part: &str) -> Result<String, String> {
    // JWT uses URL-safe base64 without padding.
    let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(part)
        .map_err(|e| format!("base64 decode failed: {e}"))?;
    let text = String::from_utf8(bytes).map_err(|_| "not UTF-8".to_string())?;
    // Pretty-print if valid JSON, else return as-is.
    match serde_json::from_str::<serde_json::Value>(&text) {
        Ok(v) => serde_json::to_string_pretty(&v).map_err(|e| e.to_string()),
        Err(_) => Ok(text),
    }
}

// ─── color converter ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

/// Parse `#rrggbb`, `#rgb`, `rgb(r,g,b)`, or `hsl(h,s%,l%)`. Returns an RGB
/// representation plus all three formats as a multi-line summary string.
pub fn color_convert(input: &str) -> Result<String, String> {
    let trimmed = input.trim();
    let rgb = parse_color(trimmed)?;
    let (h, s, l) = rgb_to_hsl(rgb);
    Ok(format!(
        "hex   #{:02X}{:02X}{:02X}\nrgb   rgb({}, {}, {})\nhsl   hsl({:.0}, {:.0}%, {:.0}%)",
        rgb.r,
        rgb.g,
        rgb.b,
        rgb.r,
        rgb.g,
        rgb.b,
        h,
        s * 100.0,
        l * 100.0,
    ))
}

fn parse_color(input: &str) -> Result<Rgb, String> {
    let s = input.trim();
    if let Some(rest) = s.strip_prefix('#') {
        return parse_hex(rest);
    }
    if let Some(rest) = s
        .strip_prefix("rgb(")
        .or_else(|| s.strip_prefix("RGB("))
        .and_then(|t| t.strip_suffix(')'))
    {
        let nums: Vec<u8> = rest
            .split(',')
            .map(|p| p.trim().parse::<u8>().map_err(|e| e.to_string()))
            .collect::<Result<_, _>>()?;
        if nums.len() != 3 {
            return Err("rgb() expects 3 components".into());
        }
        return Ok(Rgb {
            r: nums[0],
            g: nums[1],
            b: nums[2],
        });
    }
    if let Some(rest) = s
        .strip_prefix("hsl(")
        .or_else(|| s.strip_prefix("HSL("))
        .and_then(|t| t.strip_suffix(')'))
    {
        let parts: Vec<&str> = rest.split(',').map(str::trim).collect();
        if parts.len() != 3 {
            return Err("hsl() expects 3 components".into());
        }
        let h: f64 = parts[0].parse().map_err(|e: std::num::ParseFloatError| e.to_string())?;
        let s_pct: f64 = parts[1]
            .trim_end_matches('%')
            .parse()
            .map_err(|e: std::num::ParseFloatError| e.to_string())?;
        let l_pct: f64 = parts[2]
            .trim_end_matches('%')
            .parse()
            .map_err(|e: std::num::ParseFloatError| e.to_string())?;
        return Ok(hsl_to_rgb(h, s_pct / 100.0, l_pct / 100.0));
    }
    Err(format!("unrecognised color literal '{s}'"))
}

fn parse_hex(s: &str) -> Result<Rgb, String> {
    let expand = match s.len() {
        3 => {
            let mut buf = String::with_capacity(6);
            for ch in s.chars() {
                buf.push(ch);
                buf.push(ch);
            }
            buf
        }
        6 => s.to_string(),
        n => return Err(format!("hex expected 3 or 6 chars, got {n}")),
    };
    let r = u8::from_str_radix(&expand[0..2], 16).map_err(|e| e.to_string())?;
    let g = u8::from_str_radix(&expand[2..4], 16).map_err(|e| e.to_string())?;
    let b = u8::from_str_radix(&expand[4..6], 16).map_err(|e| e.to_string())?;
    Ok(Rgb { r, g, b })
}

fn rgb_to_hsl(rgb: Rgb) -> (f64, f64, f64) {
    let r = rgb.r as f64 / 255.0;
    let g = rgb.g as f64 / 255.0;
    let b = rgb.b as f64 / 255.0;
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let l = (max + min) / 2.0;
    if (max - min).abs() < 1e-9 {
        return (0.0, 0.0, l);
    }
    let d = max - min;
    let s = if l > 0.5 { d / (2.0 - max - min) } else { d / (max + min) };
    let h = if (max - r).abs() < 1e-9 {
        ((g - b) / d) + if g < b { 6.0 } else { 0.0 }
    } else if (max - g).abs() < 1e-9 {
        (b - r) / d + 2.0
    } else {
        (r - g) / d + 4.0
    };
    (h * 60.0, s, l)
}

fn hsl_to_rgb(h: f64, s: f64, l: f64) -> Rgb {
    if s.abs() < 1e-9 {
        let v = (l * 255.0).round().clamp(0.0, 255.0) as u8;
        return Rgb { r: v, g: v, b: v };
    }
    let q = if l < 0.5 { l * (1.0 + s) } else { l + s - l * s };
    let p = 2.0 * l - q;
    let hk = ((h % 360.0) + 360.0) % 360.0 / 360.0;
    let r = hue_to_rgb(p, q, hk + 1.0 / 3.0);
    let g = hue_to_rgb(p, q, hk);
    let b = hue_to_rgb(p, q, hk - 1.0 / 3.0);
    Rgb {
        r: (r * 255.0).round().clamp(0.0, 255.0) as u8,
        g: (g * 255.0).round().clamp(0.0, 255.0) as u8,
        b: (b * 255.0).round().clamp(0.0, 255.0) as u8,
    }
}

fn hue_to_rgb(p: f64, q: f64, t: f64) -> f64 {
    let t = if t < 0.0 { t + 1.0 } else if t > 1.0 { t - 1.0 } else { t };
    if t < 1.0 / 6.0 {
        p + (q - p) * 6.0 * t
    } else if t < 1.0 / 2.0 {
        q
    } else if t < 2.0 / 3.0 {
        p + (q - p) * (2.0 / 3.0 - t) * 6.0
    } else {
        p
    }
}

// ─── cron explainer ──────────────────────────────────────────────────────────

/// Parse a 5/6/7-field cron expression and return a human-readable description
/// plus the next 5 fire times in local time.
pub fn cron_explain(expression: &str) -> Result<String, String> {
    use cron::Schedule;
    use std::str::FromStr;

    // The `cron` crate expects 6 or 7 fields (with seconds). Auto-prefix '0'
    // when user provides 5-field POSIX form for ergonomic input.
    let trimmed = expression.trim();
    let normalized = match trimmed.split_whitespace().count() {
        5 => format!("0 {trimmed}"),
        6 | 7 => trimmed.to_string(),
        n => return Err(format!("cron expected 5/6/7 fields, got {n}")),
    };
    let schedule =
        Schedule::from_str(&normalized).map_err(|e| format!("invalid cron: {e}"))?;
    let mut out = format!("schedule: {}\n\nnext fires (local):", normalized);
    for dt in schedule.upcoming(Local).take(5) {
        out.push_str(&format!("\n  {}", dt.format("%Y-%m-%d %H:%M:%S")));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── uuid / nanoid ──────────────────────────────────────────────────────

    #[test]
    fn uuid_v4_has_correct_shape() {
        let s = uuid_v4();
        assert_eq!(s.len(), 36);
        assert_eq!(s.chars().filter(|c| *c == '-').count(), 4);
    }

    #[test]
    fn nanoid_respects_length() {
        assert_eq!(nanoid(10).len(), 10);
        assert_eq!(nanoid(0).len(), 21);
        assert_eq!(nanoid(500).len(), 21);
    }

    // ── password ───────────────────────────────────────────────────────────

    #[test]
    fn generate_password_alphabet_constraints() {
        let pw_alpha = generate_password(20, PasswordMode::Alpha);
        assert!(pw_alpha.chars().all(|c| c.is_ascii_alphabetic()));
        let pw_alnum = generate_password(20, PasswordMode::Alnum);
        assert!(pw_alnum.chars().all(|c| c.is_ascii_alphanumeric()));
        let pw_all = generate_password(30, PasswordMode::All);
        assert_eq!(pw_all.len(), 30);
    }

    #[test]
    fn password_mode_from_str() {
        assert_eq!(PasswordMode::from_str("alpha").unwrap(), PasswordMode::Alpha);
        assert_eq!(PasswordMode::from_str("ALNUM").unwrap(), PasswordMode::Alnum);
        assert_eq!(PasswordMode::from_str("").unwrap(), PasswordMode::All);
        assert!(PasswordMode::from_str("rocket").is_err());
    }

    // ── hash ───────────────────────────────────────────────────────────────

    #[test]
    fn hash_known_vectors() {
        assert_eq!(
            hash_text("md5", "abc").unwrap(),
            "900150983cd24fb0d6963f7d28e17f72"
        );
        assert_eq!(
            hash_text("sha1", "abc").unwrap(),
            "a9993e364706816aba3e25717850c26c9cd0d89d"
        );
        assert_eq!(
            hash_text("sha256", "abc").unwrap(),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert!(hash_text("sha512", "abc").unwrap().starts_with("ddaf35a193617aba"));
    }

    #[test]
    fn hash_unknown_algo_errors() {
        assert!(hash_text("rot13", "abc").is_err());
    }

    // ── base64 / urlenc ────────────────────────────────────────────────────

    #[test]
    fn b64_roundtrip() {
        let enc = b64_encode("Hello, world!");
        assert_eq!(enc, "SGVsbG8sIHdvcmxkIQ==");
        assert_eq!(b64_decode(&enc).unwrap(), "Hello, world!");
    }

    #[test]
    fn b64_decode_invalid_errors() {
        assert!(b64_decode("not-base64-!@#").is_err());
    }

    #[test]
    fn urlenc_roundtrip() {
        let enc = url_encode("hello world/?&=");
        assert!(enc.contains("%20") || enc.contains('+'));
        assert_eq!(url_decode(&enc).unwrap(), "hello world/?&=");
    }

    // ── json ────────────────────────────────────────────────────────────────

    #[test]
    fn json_pretty_formats() {
        let pretty = json_pretty(r#"{"a":1,"b":[1,2]}"#).unwrap();
        assert!(pretty.contains("\n"));
        assert!(pretty.contains("\"a\""));
    }

    #[test]
    fn json_minify_strips_whitespace() {
        let mini = json_minify("{\n  \"a\": 1\n}").unwrap();
        assert_eq!(mini, "{\"a\":1}");
    }

    #[test]
    fn json_invalid_errors() {
        assert!(json_pretty("{not json}").is_err());
    }

    // ── regex ──────────────────────────────────────────────────────────────

    #[test]
    fn regex_finds_matches() {
        let out = regex_test(r"\d+", "abc 123 def 456").unwrap();
        assert!(out.contains("123"));
        assert!(out.contains("456"));
        assert!(out.contains("2 match"));
    }

    #[test]
    fn regex_no_match_returns_hint() {
        let out = regex_test(r"\d+", "abc").unwrap();
        assert_eq!(out, "(no matches)");
    }

    #[test]
    fn regex_invalid_pattern_errors() {
        assert!(regex_test(r"[unclosed", "abc").is_err());
    }

    // ── jwt ─────────────────────────────────────────────────────────────────

    #[test]
    fn jwt_decode_header_and_payload() {
        // {"alg":"HS256","typ":"JWT"}.{"sub":"123","name":"Alice"}.<sig>
        let token = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjMiLCJuYW1lIjoiQWxpY2UifQ.signature";
        let out = jwt_decode(token).unwrap();
        assert!(out.contains("\"alg\""));
        assert!(out.contains("HS256"));
        assert!(out.contains("Alice"));
    }

    #[test]
    fn jwt_decode_rejects_malformed() {
        assert!(jwt_decode("just-one-part").is_err());
    }

    // ── color ───────────────────────────────────────────────────────────────

    #[test]
    fn color_hex_to_all_formats() {
        let out = color_convert("#FF0000").unwrap();
        assert!(out.contains("#FF0000"));
        assert!(out.contains("rgb(255, 0, 0)"));
        assert!(out.contains("hsl(0,"));
    }

    #[test]
    fn color_rgb_short_hex() {
        let out = color_convert("#0f0").unwrap();
        assert!(out.contains("#00FF00"));
    }

    #[test]
    fn color_invalid_input_errors() {
        assert!(color_convert("not a color").is_err());
    }

    // ── cron ────────────────────────────────────────────────────────────────

    #[test]
    fn cron_5_field_normalises_with_seconds() {
        let out = cron_explain("*/15 * * * *").unwrap();
        assert!(out.contains("schedule"));
        assert!(out.contains("next fires"));
    }

    #[test]
    fn cron_invalid_errors() {
        assert!(cron_explain("not a cron").is_err());
    }
}
