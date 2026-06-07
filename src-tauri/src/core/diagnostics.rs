//! Redacted diagnostics bundle assembler for the `/diag` builtin command.
//!
//! PRODUCT.3 trusted-release support: build a copy-pasteable summary of *what is
//! installed and configured* — version, OS, feature flags, redacted config, and
//! local data/index state — that never leaks a secret. Per ADR-0047 (proposed):
//! copy-only, no network, no file contents, and user home paths are collapsed so
//! the bundle is safe to paste into a bug report.
//!
//! [`build_report`] is intentionally pure (no disk/keychain access) so the
//! redaction guarantees can be unit-tested. The caller resolves path
//! existence/size and supplies the already-redacted config rows from
//! [`crate::core::config_manager::ConfigManager::list_all_redacted`].

use std::path::{Path, PathBuf};

use serde::Serialize;

/// Mask shown for sensitive config values. Mirrors `settings_schema::SECRET_MASK`.
/// Re-applied here as defense-in-depth so a sensitive row is masked even if a
/// caller ever passes an unredacted value.
const SECRET_MASK: &str = "********";

#[derive(Debug, Clone, Serialize)]
pub struct AppFacts {
    pub version: String,
    pub os: String,
    pub arch: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct RedactedSetting {
    pub key: String,
    pub value: String,
    pub sensitive: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct PathEntry {
    pub label: String,
    /// Home-collapsed path string (never the raw absolute path with a username).
    pub path: String,
    pub exists: bool,
    pub size_bytes: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct PreflightFacts {
    pub status: String,
    pub source_mode: String,
    pub ollama_reachable: bool,
    pub local_model_count: usize,
    pub generated_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct DiagnosticsReport {
    pub app: AppFacts,
    pub flags: Vec<RedactedSetting>,
    pub config: Vec<RedactedSetting>,
    pub paths: Vec<PathEntry>,
    pub preflight: Option<PreflightFacts>,
    /// Most recent backend panic entry (already redacted), or `None`. STAB.1.
    pub last_crash: Option<String>,
}

/// A local path whose existence/size the caller already resolved. Kept separate
/// from [`build_report`] so the assembler stays disk-free and testable.
pub struct ResolvedPath {
    pub label: String,
    pub path: PathBuf,
    pub exists: bool,
    pub size_bytes: u64,
}

pub struct DiagnosticsInputs {
    pub version: String,
    pub os: String,
    pub arch: String,
    pub home_dir: Option<PathBuf>,
    /// `(key, redacted_value, sensitive)` rows from `list_all_redacted()`.
    pub redacted_config: Vec<(String, String, bool)>,
    pub paths: Vec<ResolvedPath>,
    pub preflight: Option<PreflightFacts>,
    /// Last redacted backend crash line from `crash_log::read_last_crash`. STAB.1.
    pub last_crash: Option<String>,
}

/// Platform token used to stand in for the user's home directory.
fn home_token() -> &'static str {
    if cfg!(windows) {
        "%USERPROFILE%"
    } else {
        "~"
    }
}

/// Replace a leading `home` prefix in `path` with `token` so the rendered bundle
/// does not leak the OS username. Non-home paths are returned unchanged.
pub fn collapse_home(path: &str, home: &str, token: &str) -> String {
    if home.is_empty() {
        return path.to_string();
    }
    let normalized_home = home.trim_end_matches(['/', '\\']);
    match path.strip_prefix(normalized_home) {
        // Only collapse on a real path-component boundary: the remainder must be
        // empty or start with a separator. Without this, a home of `/home/al`
        // would wrongly collapse `/home/alice/...` into `~ice/...`.
        Some(rest) if rest.is_empty() || rest.starts_with(['/', '\\']) => {
            format!("{token}{rest}")
        }
        _ => path.to_string(),
    }
}

/// Feature-flag config keys surfaced in the dedicated flags section.
fn is_flag_key(key: &str) -> bool {
    key.starts_with("features.") || key == "ai.legacy_agent"
}

/// Assemble a [`DiagnosticsReport`] from already-redacted inputs. Pure: enforces
/// the sensitive-value mask and home-path collapse but performs no I/O.
pub fn build_report(inputs: DiagnosticsInputs) -> DiagnosticsReport {
    let token = home_token();
    let home = inputs
        .home_dir
        .as_ref()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default();

    let mut flags = Vec::new();
    let mut config = Vec::new();
    for (key, value, sensitive) in inputs.redacted_config {
        // Defense-in-depth: never emit a non-empty value for a sensitive key.
        let value = if sensitive && !value.is_empty() {
            SECRET_MASK.to_string()
        } else {
            value
        };
        let setting = RedactedSetting {
            key,
            value,
            sensitive,
        };
        if is_flag_key(&setting.key) {
            flags.push(setting.clone());
        }
        config.push(setting);
    }

    let paths = inputs
        .paths
        .into_iter()
        .map(|p| PathEntry {
            label: p.label,
            path: collapse_home(&p.path.to_string_lossy(), &home, token),
            exists: p.exists,
            size_bytes: p.size_bytes,
        })
        .collect();

    DiagnosticsReport {
        app: AppFacts {
            version: inputs.version,
            os: inputs.os,
            arch: inputs.arch,
        },
        flags,
        config,
        paths,
        preflight: inputs.preflight,
        last_crash: inputs.last_crash,
    }
}

/// Render a deterministic, copy-pasteable plain-text bundle for the palette's
/// inline result region.
pub fn render_text(report: &DiagnosticsReport) -> String {
    let mut out = String::new();
    out.push_str("Keynova Diagnostics\n");
    out.push_str("===================\n\n");

    out.push_str("App\n");
    out.push_str(&format!("  version : {}\n", report.app.version));
    out.push_str(&format!("  os      : {}\n", report.app.os));
    out.push_str(&format!("  arch    : {}\n\n", report.app.arch));

    out.push_str("Feature Flags\n");
    if report.flags.is_empty() {
        out.push_str("  (none)\n");
    } else {
        for f in &report.flags {
            out.push_str(&format!("  {} = {}\n", f.key, f.value));
        }
    }
    out.push('\n');

    if let Some(pf) = &report.preflight {
        out.push_str("Startup Preflight\n");
        out.push_str(&format!("  status           : {}\n", pf.status));
        out.push_str(&format!("  source_mode      : {}\n", pf.source_mode));
        out.push_str(&format!("  ollama_reachable : {}\n", pf.ollama_reachable));
        out.push_str(&format!("  local_models     : {}\n", pf.local_model_count));
        out.push_str(&format!("  generated_at     : {}\n\n", pf.generated_at));
    }

    out.push_str("Local Data\n");
    for p in &report.paths {
        let state = if p.exists {
            format!("present, {} bytes", p.size_bytes)
        } else {
            "absent".to_string()
        };
        out.push_str(&format!("  {} : {} — {}\n", p.label, state, p.path));
    }
    out.push('\n');

    out.push_str("Last Crash\n");
    match &report.last_crash {
        Some(line) => out.push_str(&format!("  {line}\n")),
        None => out.push_str("  (none)\n"),
    }
    out.push('\n');

    out.push_str("Config (redacted)\n");
    for c in &report.config {
        let value = if c.value.is_empty() {
            "(empty)"
        } else {
            c.value.as_str()
        };
        out.push_str(&format!("  {} = {}\n", c.key, value));
    }

    out
}

/// Max directory depth and total entries [`path_size`] will visit. Past either
/// cap it returns a best-effort partial sum rather than stalling — this also
/// bounds pathological inputs like a symlink cycle under the index dir.
const PATH_SIZE_MAX_DEPTH: usize = 16;
const PATH_SIZE_MAX_ENTRIES: usize = 50_000;

/// Recursively sum file sizes under `path` (file → its size, dir → sum of
/// contained files). Best-effort: unreadable entries contribute 0, and traversal
/// stops past depth/entry caps so `/diag` can never hang on a huge or cyclic
/// tree. Reads metadata only, never file contents.
pub fn path_size(path: &Path) -> u64 {
    let mut budget = PATH_SIZE_MAX_ENTRIES;
    path_size_bounded(path, 0, &mut budget)
}

fn path_size_bounded(path: &Path, depth: usize, budget: &mut usize) -> u64 {
    if *budget == 0 {
        return 0;
    }
    *budget -= 1;
    let Ok(meta) = std::fs::metadata(path) else {
        return 0;
    };
    if meta.is_file() {
        return meta.len();
    }
    // Non-dir (special files) or at the depth cap: stop descending.
    if !meta.is_dir() || depth >= PATH_SIZE_MAX_DEPTH {
        return 0;
    }
    match std::fs::read_dir(path) {
        Ok(entries) => entries
            .flatten()
            .map(|entry| path_size_bounded(&entry.path(), depth + 1, budget))
            .sum(),
        Err(_) => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn inputs_with_config(rows: Vec<(&str, &str, bool)>) -> DiagnosticsInputs {
        DiagnosticsInputs {
            version: "9.9.9".to_string(),
            os: "testos".to_string(),
            arch: "testarch".to_string(),
            home_dir: None,
            redacted_config: rows
                .into_iter()
                .map(|(k, v, s)| (k.to_string(), v.to_string(), s))
                .collect(),
            paths: Vec::new(),
            preflight: None,
            last_crash: None,
        }
    }

    #[test]
    fn collapse_home_rewrites_home_prefix_and_leaves_others() {
        assert_eq!(
            collapse_home("/home/alice/Keynova/config.toml", "/home/alice", "~"),
            "~/Keynova/config.toml"
        );
        // Trailing separator on home is tolerated.
        assert_eq!(
            collapse_home("/home/alice/x", "/home/alice/", "~"),
            "~/x"
        );
        // Non-home path is untouched.
        assert_eq!(collapse_home("/etc/hosts", "/home/alice", "~"), "/etc/hosts");
        // Empty home disables collapsing.
        assert_eq!(collapse_home("/home/alice/x", "", "~"), "/home/alice/x");
    }

    #[test]
    fn collapse_home_requires_component_boundary() {
        // A home that is a string-prefix of a *different* user must not collapse:
        // `/home/al` is not a path-component prefix of `/home/alice`.
        assert_eq!(
            collapse_home("/home/alice/x", "/home/al", "~"),
            "/home/alice/x"
        );
        // Exact match collapses to the bare token.
        assert_eq!(collapse_home("/home/al", "/home/al", "~"), "~");
        // Windows separators are honored too.
        assert_eq!(
            collapse_home(r"C:\Users\bob\Keynova", r"C:\Users\bob", "%USERPROFILE%"),
            r"%USERPROFILE%\Keynova"
        );
        assert_eq!(
            collapse_home(r"C:\Users\bobby\x", r"C:\Users\bob", "%USERPROFILE%"),
            r"C:\Users\bobby\x"
        );
    }

    #[test]
    fn build_report_masks_sensitive_value_even_if_caller_passes_raw() {
        // Defense-in-depth: a sensitive row carrying a raw secret must still be
        // masked in the assembled report.
        let report = build_report(inputs_with_config(vec![
            ("translation.api_key", "sk-LEAKED-SECRET", true),
            ("launcher.theme", "dark", false),
        ]));

        let secret_row = report
            .config
            .iter()
            .find(|c| c.key == "translation.api_key")
            .expect("secret row present");
        assert_eq!(secret_row.value, SECRET_MASK);

        let rendered = render_text(&report);
        assert!(!rendered.contains("sk-LEAKED-SECRET"));
        assert!(rendered.contains("translation.api_key = ********"));
        // Non-sensitive value is preserved.
        assert!(rendered.contains("launcher.theme = dark"));
    }

    #[test]
    fn flags_section_extracts_feature_and_legacy_keys() {
        let report = build_report(inputs_with_config(vec![
            ("features.ai", "false", false),
            ("ai.legacy_agent", "false", false),
            ("launcher.theme", "dark", false),
        ]));
        let flag_keys: Vec<&str> = report.flags.iter().map(|f| f.key.as_str()).collect();
        assert!(flag_keys.contains(&"features.ai"));
        assert!(flag_keys.contains(&"ai.legacy_agent"));
        assert!(!flag_keys.contains(&"launcher.theme"));
    }

    #[test]
    fn render_text_is_deterministic_and_has_sections() {
        let report = build_report(inputs_with_config(vec![("features.ai", "false", false)]));
        let a = render_text(&report);
        let b = render_text(&report);
        assert_eq!(a, b);
        assert!(a.contains("Keynova Diagnostics"));
        assert!(a.contains("App\n"));
        assert!(a.contains("version : 9.9.9"));
        assert!(a.contains("os      : testos"));
        assert!(a.contains("Feature Flags"));
        assert!(a.contains("Config (redacted)"));
    }

    #[test]
    fn empty_sensitive_default_stays_empty() {
        let report = build_report(inputs_with_config(vec![("translation.api_key", "", true)]));
        let row = report
            .config
            .iter()
            .find(|c| c.key == "translation.api_key")
            .expect("row present");
        assert_eq!(row.value, "");
        assert!(render_text(&report).contains("translation.api_key = (empty)"));
    }

    #[test]
    fn path_size_sums_directory_files() {
        let base =
            std::env::temp_dir().join(format!("keynova_diag_test_{}", std::process::id()));
        let nested = base.join("nested");
        std::fs::create_dir_all(&nested).expect("create temp dirs");
        std::fs::write(base.join("a.txt"), b"hello").expect("write a"); // 5 bytes
        std::fs::write(nested.join("b.txt"), b"world!").expect("write b"); // 6 bytes

        assert_eq!(path_size(&base), 11);
        assert_eq!(path_size(&base.join("a.txt")), 5);
        assert_eq!(path_size(&base.join("missing")), 0);

        std::fs::remove_dir_all(&base).ok();
    }

    #[test]
    fn path_size_stops_at_depth_cap() {
        // A chain deeper than the cap must terminate (best-effort partial) rather
        // than recurse without bound. File past the cap is not counted.
        let base = std::env::temp_dir()
            .join(format!("keynova_diag_depth_{}", std::process::id()));
        let mut deep = base.clone();
        for i in 0..(PATH_SIZE_MAX_DEPTH + 5) {
            deep = deep.join(format!("d{i}"));
        }
        std::fs::create_dir_all(&deep).expect("create deep dirs");
        std::fs::write(deep.join("buried.txt"), b"xxxx").expect("write buried");

        // Must return (no hang/stack blowup); the buried file sits past the cap.
        assert_eq!(path_size(&base), 0);

        std::fs::remove_dir_all(&base).ok();
    }
}
