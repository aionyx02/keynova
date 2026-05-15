use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use crate::core::{prepare_observation, AgentObservationPolicy};
use crate::core::config_manager::ConfigManager;
use crate::models::learning_material::{MaterialCandidate, MaterialClass, ReviewReport, ScanStats};

const DEFAULT_MAX_SCAN_FILES: usize = 500;
const DEFAULT_MAX_PREVIEW_BYTES: usize = 4096;
const DEFAULT_MAX_DEPTH: usize = 4;

const DEFAULT_DENYLIST: &[&str] = &[
    ".git",
    "node_modules",
    ".DS_Store",
    "target",
    "__pycache__",
    ".venv",
    "venv",
    ".env",
    "*.key",
    "*.pem",
    "*.pfx",
    "*.p12",
    "*.ppk",
    "id_rsa",
    "id_ed25519",
    "*.asc",
];

/// Configuration-driven scanner for local learning materials.
///
/// Built from `ConfigManager` values at call time; holds no locks.
/// All path access enforces `canonicalize()` + root-prefix symlink-escape checks.
pub struct LearningMaterialManager {
    pub enabled: bool,
    pub max_scan_files: usize,
    pub max_preview_bytes: usize,
    pub max_depth: usize,
    pub denylist: Vec<String>,
}

impl LearningMaterialManager {
    pub fn from_config(config: &ConfigManager) -> Self {
        let enabled = config
            .get("agent.local_context.enabled")
            .map(|v| v.eq_ignore_ascii_case("true"))
            .unwrap_or(false);
        let max_scan_files = config
            .get("agent.local_context.max_scan_files")
            .and_then(|v| v.parse().ok())
            .unwrap_or(DEFAULT_MAX_SCAN_FILES);
        let max_preview_bytes = config
            .get("agent.local_context.max_preview_bytes")
            .and_then(|v| v.parse().ok())
            .unwrap_or(DEFAULT_MAX_PREVIEW_BYTES);
        let max_depth = config
            .get("agent.local_context.max_depth")
            .and_then(|v| v.parse().ok())
            .unwrap_or(DEFAULT_MAX_DEPTH);
        // Denylist: config value is comma-separated; merged with hardcoded defaults.
        let mut denylist: Vec<String> = DEFAULT_DENYLIST.iter().map(|s| s.to_string()).collect();
        if let Some(extra) = config.get("agent.local_context.extra_denylist") {
            for pat in extra.split(',').map(str::trim).filter(|s| !s.is_empty()) {
                denylist.push(pat.to_string());
            }
        }
        Self {
            enabled,
            max_scan_files,
            max_preview_bytes,
            max_depth,
            denylist,
        }
    }

    /// Metadata-first scan of `roots`. Returns an error if `enabled = false`.
    ///
    /// No file content is read during the scan; only filesystem metadata is accessed.
    pub fn scan(&self, roots: &[PathBuf]) -> Result<ReviewReport, String> {
        if !self.enabled {
            return Err(
                "Learning material review is disabled. \
                 Enable agent.local_context.enabled in Settings → Agent."
                    .into(),
            );
        }

        let mut candidates: Vec<MaterialCandidate> = Vec::new();
        let mut stats = ScanStats::default();
        let mut all_roots: Vec<String> = Vec::new();

        for root in roots {
            let canonical_root = root
                .canonicalize()
                .map_err(|e| format!("invalid root '{}': {e}", root.display()))?;
            all_roots.push(canonical_root.display().to_string());
            self.walk(
                &canonical_root,
                &canonical_root,
                0,
                &mut candidates,
                &mut stats,
            );
        }

        stats.candidate_count = candidates.len();
        Ok(ReviewReport {
            roots: all_roots,
            candidates,
            stats,
        })
    }

    /// Produce a bounded, redacted text preview of a file.
    ///
    /// Returns an explanatory label string when the file is denied, binary, or unreadable.
    pub fn preview_file(&self, path: &Path) -> String {
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("");

        if is_denied(name, &self.denylist) {
            return "[denylist: preview blocked]".to_string();
        }

        let size = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
        if size > (self.max_preview_bytes * 8) as u64 {
            return format!("[{size} bytes: too large for preview]");
        }

        let mut buf = vec![0u8; self.max_preview_bytes];
        let n = match std::fs::File::open(path).and_then(|mut f| f.read(&mut buf)) {
            Ok(n) => n,
            Err(e) => return format!("[read error: {e}]"),
        };

        match std::str::from_utf8(&buf[..n]) {
            Ok(text) => prepare_observation(
                text,
                &AgentObservationPolicy {
                    max_chars: self.max_preview_bytes,
                    max_lines: 80,
                    preserve_head_lines: 40,
                    preserve_tail_lines: 20,
                    redact_secrets: true,
                },
            )
            .content,
            Err(_) => "[binary: preview not available]".to_string(),
        }
    }

    // ── Private scan helpers ──────────────────────────────────────────────────

    fn walk(
        &self,
        root: &Path,
        dir: &Path,
        depth: usize,
        candidates: &mut Vec<MaterialCandidate>,
        stats: &mut ScanStats,
    ) {
        if candidates.len() >= self.max_scan_files || depth > self.max_depth {
            return;
        }

        let entries = match std::fs::read_dir(dir) {
            Ok(it) => it,
            Err(_) => {
                stats.denied_count += 1;
                return;
            }
        };

        for entry in entries.flatten() {
            if candidates.len() >= self.max_scan_files {
                break;
            }

            let path = entry.path();

            // Symlink escape prevention: canonicalize and verify prefix.
            let canonical = match path.canonicalize() {
                Ok(c) => c,
                Err(_) => {
                    stats.denied_count += 1;
                    continue;
                }
            };
            if !canonical.starts_with(root) {
                stats.denied_count += 1;
                continue;
            }

            let name = canonical
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_string();

            if is_denied(&name, &self.denylist) {
                stats.filtered_count += 1;
                continue;
            }

            stats.scanned_count += 1;

            if canonical.is_dir() {
                // Project root detection (classify the directory itself).
                if is_project_root(&canonical) {
                    let modified_secs = modified_epoch(&entry.metadata().ok());
                    candidates.push(MaterialCandidate {
                        path: canonical.display().to_string(),
                        name: name.clone(),
                        class: MaterialClass::Project,
                        size_bytes: 0,
                        modified_secs,
                    });
                }
                // Always recurse into directories (within depth limit).
                self.walk(root, &canonical, depth + 1, candidates, stats);
            } else {
                let class = classify_by_extension(&canonical);
                if class == MaterialClass::Unknown {
                    continue;
                }
                let meta = entry.metadata().ok();
                candidates.push(MaterialCandidate {
                    path: canonical.display().to_string(),
                    name,
                    class,
                    size_bytes: meta.as_ref().map(|m| m.len()).unwrap_or(0),
                    modified_secs: modified_epoch(&meta),
                });
            }
        }
    }
}

// ── Pure helper functions ─────────────────────────────────────────────────────

fn is_denied(name: &str, denylist: &[String]) -> bool {
    let lower = name.to_lowercase();
    for pattern in denylist {
        let p = pattern.to_lowercase();
        if let Some(ext) = p.strip_prefix('*') {
            // Glob extension: "*.key" → match files ending in ".key"
            if lower.ends_with(ext) {
                return true;
            }
        } else if lower == p {
            return true;
        }
    }
    false
}

fn is_project_root(dir: &Path) -> bool {
    const MARKERS: &[&str] = &[
        "Cargo.toml",
        "package.json",
        "go.mod",
        "pyproject.toml",
        "build.gradle",
        "CMakeLists.txt",
        ".git",
    ];
    MARKERS.iter().any(|m| dir.join(m).exists())
}

pub fn classify_by_extension(path: &Path) -> MaterialClass {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_lowercase)
        .unwrap_or_default();

    match ext.as_str() {
        "md" | "txt" | "org" | "rst" | "wiki" | "adoc" => MaterialClass::Note,
        "pdf" | "docx" | "doc" | "odt" | "rtf" | "pages" | "epub" => MaterialClass::Report,
        "pptx" | "ppt" | "key" | "odp" | "gslides" => MaterialClass::Presentation,
        "cer" | "crt" | "pem" | "p12" | "pfx" | "p7b" | "der" => MaterialClass::Certificate,
        _ => MaterialClass::Unknown,
    }
}

fn modified_epoch(meta: &Option<std::fs::Metadata>) -> u64 {
    meta.as_ref()
        .and_then(|m| m.modified().ok())
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use uuid::Uuid;

    fn mk_tmp(suffix: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("keynova-lm-test-{suffix}-{}", Uuid::new_v4()));
        fs::create_dir_all(&dir).expect("create tmp dir");
        dir.canonicalize().expect("canonicalize tmp dir")
    }

    fn enabled_manager() -> LearningMaterialManager {
        LearningMaterialManager {
            enabled: true,
            max_scan_files: 500,
            max_preview_bytes: 4096,
            max_depth: 4,
            denylist: DEFAULT_DENYLIST.iter().map(|s| s.to_string()).collect(),
        }
    }

    fn disabled_manager() -> LearningMaterialManager {
        LearningMaterialManager { enabled: false, ..enabled_manager() }
    }

    #[test]
    fn scan_returns_error_when_disabled() {
        let mgr = disabled_manager();
        let err = mgr.scan(&[std::env::temp_dir()]).unwrap_err();
        assert!(err.contains("disabled"), "expected 'disabled' in: {err}");
    }

    #[test]
    fn scan_rejects_nonexistent_root() {
        let mgr = enabled_manager();
        assert!(mgr.scan(&[PathBuf::from("C:\\nonexistent\\keynova_lm_xyz")]).is_err()
            || mgr.scan(&[PathBuf::from("/nonexistent/keynova_lm_xyz")]).is_err());
    }

    #[test]
    fn classifier_identifies_note_extensions() {
        for (name, expected) in [("file.md", MaterialClass::Note), ("readme.txt", MaterialClass::Note), ("doc.org", MaterialClass::Note)] {
            assert_eq!(classify_by_extension(Path::new(name)), expected, "failed for {name}");
        }
    }

    #[test]
    fn classifier_identifies_report_extensions() {
        assert_eq!(classify_by_extension(Path::new("report.pdf")), MaterialClass::Report);
        assert_eq!(classify_by_extension(Path::new("thesis.docx")), MaterialClass::Report);
    }

    #[test]
    fn classifier_identifies_presentation_extensions() {
        assert_eq!(classify_by_extension(Path::new("slides.pptx")), MaterialClass::Presentation);
    }

    #[test]
    fn classifier_identifies_certificate_extensions() {
        assert_eq!(classify_by_extension(Path::new("cert.pem")), MaterialClass::Certificate);
        assert_eq!(classify_by_extension(Path::new("root.cer")), MaterialClass::Certificate);
    }

    #[test]
    fn classifier_returns_unknown_for_binary() {
        assert_eq!(classify_by_extension(Path::new("program.exe")), MaterialClass::Unknown);
        assert_eq!(classify_by_extension(Path::new("archive.zip")), MaterialClass::Unknown);
    }

    #[test]
    fn denylist_blocks_git_and_node_modules() {
        let dl: Vec<String> = DEFAULT_DENYLIST.iter().map(|s| s.to_string()).collect();
        assert!(is_denied(".git", &dl));
        assert!(is_denied("node_modules", &dl));
    }

    #[test]
    fn denylist_blocks_secret_extensions() {
        let dl: Vec<String> = DEFAULT_DENYLIST.iter().map(|s| s.to_string()).collect();
        assert!(is_denied("server.pem", &dl));
        assert!(is_denied("client.key", &dl));
        assert!(is_denied("identity.p12", &dl));
    }

    #[test]
    fn denylist_allows_benign_files() {
        let dl: Vec<String> = DEFAULT_DENYLIST.iter().map(|s| s.to_string()).collect();
        assert!(!is_denied("notes.md", &dl));
        assert!(!is_denied("thesis.pdf", &dl));
    }

    #[test]
    fn scan_counts_stats_correctly() {
        let root = mk_tmp("stats");
        fs::write(root.join("note.md"), "content").expect("write note");
        fs::write(root.join("report.pdf"), "content").expect("write report");
        fs::write(root.join("unknown.exe"), "content").expect("write exe");
        fs::create_dir(root.join("node_modules")).expect("mkdir node_modules");

        let mgr = enabled_manager();
        let report = mgr.scan(&[root]).expect("scan ok");

        assert_eq!(report.candidates.len(), 2, "note + report should be candidates");
        assert!(report.stats.filtered_count >= 1, "node_modules should be filtered");
    }

    #[test]
    fn scan_grounded_output_covers_expected_classes() {
        let root = mk_tmp("classes");
        fs::write(root.join("note.md"), "hello").expect("write");
        fs::write(root.join("report.pdf"), "pdf").expect("write");
        fs::write(root.join("slides.pptx"), "pptx").expect("write");

        let mgr = enabled_manager();
        let report = mgr.scan(&[root]).expect("scan");

        let classes: Vec<MaterialClass> = report.candidates.iter().map(|c| c.class).collect();
        assert!(classes.contains(&MaterialClass::Note));
        assert!(classes.contains(&MaterialClass::Report));
        assert!(classes.contains(&MaterialClass::Presentation));
    }

    #[test]
    fn review_report_to_markdown_includes_all_sections() {
        use crate::models::learning_material::{MaterialCandidate, ReviewReport, ScanStats};
        let report = ReviewReport {
            roots: vec!["/home/user/docs".into()],
            candidates: vec![
                MaterialCandidate { path: "/home/user/docs/note.md".into(), name: "note.md".into(), class: MaterialClass::Note, size_bytes: 1024, modified_secs: 0 },
                MaterialCandidate { path: "/home/user/docs/slides.pptx".into(), name: "slides.pptx".into(), class: MaterialClass::Presentation, size_bytes: 2048, modified_secs: 0 },
            ],
            stats: ScanStats { scanned_count: 5, candidate_count: 2, filtered_count: 2, denied_count: 1 },
        };
        let md = report.to_markdown();
        assert!(md.contains("# Learning Material Review"));
        assert!(md.contains("## Note"));
        assert!(md.contains("## Presentation"));
        assert!(md.contains("note.md"));
        assert!(md.contains("slides.pptx"));
    }
}