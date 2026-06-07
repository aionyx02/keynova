//! Backend crash-log panic hook (STAB.1 / ADR-0051).
//!
//! The frontend already has an ErrorBoundary plus global `error` /
//! `unhandledrejection` handlers, but the Rust side installs no panic hook, so a
//! panic in an async task or command handler can drop the process with nothing on
//! disk to diagnose (the "閃退 / window vanished" class in `bug-followup.md`).
//!
//! [`install_panic_hook`] registers a process-wide hook that appends one
//! **redacted**, single line per panic to `crash.log` in the data dir, then
//! chains to the previous hook so console/stderr behavior is unchanged. The write
//! is best-effort and must never panic, block, or grow without bound.
//!
//! The pure helpers ([`format_entry`], [`append_bounded`]) carry the redaction
//! and rotation guarantees and are unit-tested; the I/O wrappers stay thin.

use std::panic::PanicHookInfo;
use std::path::{Path, PathBuf};
use std::sync::Once;

use chrono::Utc;

/// Cap the whole log so it can never grow unbounded; oldest lines drop first.
const MAX_LOG_BYTES: usize = 64 * 1024;
/// Cap a single panic message so a giant payload can't bloat the log or `/diag`.
const MAX_MSG_CHARS: usize = 500;

static INSTALL_ONCE: Once = Once::new();

/// Home-directory stand-in token, matching the `/diag` bundle convention.
fn home_token() -> &'static str {
    if cfg!(windows) {
        "%USERPROFILE%"
    } else {
        "~"
    }
}

/// Replace **every** occurrence of the user's home path with `token`. Unlike
/// `diagnostics::collapse_home` (leading-prefix only, for path display), a panic
/// message can embed a home path mid-string, so redact all occurrences.
fn redact_home_all(s: &str, home: &str, token: &str) -> String {
    if home.is_empty() {
        return s.to_string();
    }
    let trimmed = home.trim_end_matches(['/', '\\']);
    if trimmed.is_empty() {
        return s.to_string();
    }
    s.replace(trimmed, token)
}

/// Build one redacted, single-line crash entry. Pure: the caller supplies the
/// timestamp, location, raw message, and home dir so the format/redaction can be
/// tested without installing a hook or touching the clock.
pub fn format_entry(timestamp: &str, location: &str, message: &str, home: &str) -> String {
    let token = home_token();
    let location = redact_home_all(location, home, token).replace(['\n', '\r', '\t'], " ");
    let mut msg = redact_home_all(message, home, token).replace(['\n', '\r', '\t'], " ");
    if msg.chars().count() > MAX_MSG_CHARS {
        msg = msg.chars().take(MAX_MSG_CHARS).collect::<String>();
        msg.push('…');
    }
    format!("{timestamp}\t{location}\t{msg}")
}

/// Append `entry` to `existing` log contents, dropping oldest whole lines so the
/// result stays within `max_bytes`. Pure.
pub fn append_bounded(existing: &str, entry: &str, max_bytes: usize) -> String {
    let mut combined = String::with_capacity(existing.len() + entry.len() + 2);
    combined.push_str(existing);
    if !existing.is_empty() && !existing.ends_with('\n') {
        combined.push('\n');
    }
    combined.push_str(entry);
    combined.push('\n');

    if combined.len() <= max_bytes {
        return combined;
    }
    // Drop oldest whole lines until under the cap (keep the newest entries).
    let mut start = 0;
    while combined.len() - start > max_bytes {
        match combined[start..].find('\n') {
            Some(idx) => start += idx + 1,
            None => break,
        }
    }
    combined[start..].to_string()
}

/// Extract the panic payload as a string for the common `&str` / `String` cases.
fn payload_message(info: &PanicHookInfo<'_>) -> String {
    if let Some(s) = info.payload().downcast_ref::<&str>() {
        (*s).to_string()
    } else if let Some(s) = info.payload().downcast_ref::<String>() {
        s.clone()
    } else {
        "<non-string panic payload>".to_string()
    }
}

/// Best-effort append of one entry to the crash log. Never panics or blocks.
fn write_entry(path: &Path, entry: &str) {
    let existing = std::fs::read_to_string(path).unwrap_or_default();
    let next = append_bounded(&existing, entry, MAX_LOG_BYTES);
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(path, next);
}

/// Install a process-wide panic hook that records a redacted entry to
/// `crash_log_path`, then chains to the previous hook. Idempotent (first call
/// wins) so repeated calls in tests or re-init are safe.
pub fn install_panic_hook(crash_log_path: PathBuf) {
    INSTALL_ONCE.call_once(move || {
        let previous = std::panic::take_hook();
        let home = dirs::home_dir()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default();
        std::panic::set_hook(Box::new(move |info| {
            let location = info
                .location()
                .map(|l| format!("{}:{}", l.file(), l.line()))
                .unwrap_or_else(|| "<unknown>".to_string());
            let entry = format_entry(
                &Utc::now().to_rfc3339(),
                &location,
                &payload_message(info),
                &home,
            );
            write_entry(&crash_log_path, &entry);
            previous(info);
        }));
    });
}

/// Most recent crash entry (already redacted on write) for the `/diag` bundle, or
/// `None` if no crash has been recorded.
pub fn read_last_crash(crash_log_path: &Path) -> Option<String> {
    let content = std::fs::read_to_string(crash_log_path).ok()?;
    content
        .lines()
        .rev()
        .find(|l| !l.trim().is_empty())
        .map(|l| l.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_entry_redacts_home_and_is_single_line() {
        let entry = format_entry(
            "2026-06-07T00:00:00Z",
            "src/core/foo.rs:12",
            "boom reading /home/alice/secret\nstacky\tdetail",
            "/home/alice",
        );
        // No raw home path, no embedded newline/tab beyond the field separators.
        assert!(!entry.contains("/home/alice"));
        assert!(entry.contains(&format!("{}/secret", home_token())));
        assert_eq!(entry.lines().count(), 1);
        // Tab-delimited: timestamp, location, message.
        let fields: Vec<&str> = entry.split('\t').collect();
        assert_eq!(fields.len(), 3);
        assert_eq!(fields[0], "2026-06-07T00:00:00Z");
    }

    #[test]
    fn format_entry_caps_long_message() {
        let long = "x".repeat(MAX_MSG_CHARS + 100);
        let entry = format_entry("t", "loc", &long, "");
        let msg = entry.split('\t').nth(2).unwrap();
        assert!(msg.chars().count() <= MAX_MSG_CHARS + 1); // +1 for the ellipsis
        assert!(msg.ends_with('…'));
    }

    #[test]
    fn append_bounded_keeps_newest_within_cap() {
        let mut log = String::new();
        for i in 0..200 {
            log = append_bounded(&log, &format!("entry-{i}"), 100);
            assert!(log.len() <= 100, "log exceeded cap at i={i}: {}", log.len());
        }
        // Oldest dropped, newest retained.
        assert!(log.contains("entry-199"));
        assert!(!log.contains("entry-0\n"));
    }

    #[test]
    fn append_bounded_appends_when_under_cap() {
        let a = append_bounded("", "first", 1024);
        let b = append_bounded(&a, "second", 1024);
        assert_eq!(b, "first\nsecond\n");
    }

    #[test]
    fn read_last_crash_returns_newest_line() {
        let base = std::env::temp_dir()
            .join(format!("keynova_crashlog_{}", std::process::id()));
        std::fs::create_dir_all(&base).expect("mkdir");
        let path = base.join("crash.log");

        assert_eq!(read_last_crash(&path), None);

        write_entry(&path, "2026-06-07T00:00:00Z\tloc1\told");
        write_entry(&path, "2026-06-07T00:00:01Z\tloc2\tnew");
        let last = read_last_crash(&path).expect("entry present");
        assert!(last.contains("new"));
        assert!(!last.contains("old"));

        std::fs::remove_dir_all(&base).ok();
    }
}