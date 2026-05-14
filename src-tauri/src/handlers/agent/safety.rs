use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use crate::core::config_manager::ConfigManager;

pub(super) fn sanitize_external_query(query: &str) -> Result<String, String> {
    let trimmed = query.trim();
    if trimmed.is_empty() {
        return Err("web.search query is empty".into());
    }
    let lower = trimmed.to_lowercase();
    for denied in [
        "claude.md",
        "tasks.md",
        "memory.md",
        "decisions.md",
        "skill.md",
        "private_architecture",
        "secret",
        "api_key",
        "api key",
        "password",
        "token",
    ] {
        if lower.contains(denied) {
            return Err(format!(
                "web.search query contains private or secret context term '{denied}'"
            ));
        }
    }
    Ok(trimmed.to_string())
}

pub(super) fn contains_any(haystack: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| haystack.contains(needle))
}

pub(super) fn long_term_memory_opt_in(config: &Arc<Mutex<ConfigManager>>) -> bool {
    config
        .lock()
        .ok()
        .and_then(|cfg| cfg.get("agent.long_term_memory_opt_in"))
        .is_some_and(|value| value == "true")
}

pub(super) fn is_allowlisted_safe_builtin(name: &str, args: &str) -> bool {
    args.trim().is_empty()
        && matches!(
            name,
            "help" | "note" | "history" | "cal" | "tr" | "ai" | "model_list"
        )
}

pub(super) fn looks_sensitive_path(path: &Path) -> bool {
    const SENSITIVE_COMPONENTS: &[&str] = &[
        ".ssh", ".gnupg", ".gpg", ".aws", ".azure", ".gcloud",
        "id_rsa", "id_ed25519", "id_ecdsa", "id_dsa",
        ".env", ".env.local", ".env.production", ".env.secret",
        "credentials", "secrets", "secret.json", "secret.toml",
        "keystore", "truststore", ".netrc", ".pgpass",
    ];
    path.components().any(|comp| {
        let s = comp.as_os_str().to_string_lossy();
        SENSITIVE_COMPONENTS
            .iter()
            .any(|pat| s.eq_ignore_ascii_case(pat))
    })
}

/// Resolve a path string to an absolute, canonicalized path that is:
/// - reachable (exists on disk)
/// - within one of the supplied workspace roots
/// - not a sensitive credential path
pub(super) fn resolve_readable_path(path_str: &str, roots: &[PathBuf]) -> Result<PathBuf, String> {
    let raw = PathBuf::from(path_str);

    // Resolve to an absolute candidate (prefer workspace-relative when not absolute)
    let candidate = if raw.is_absolute() {
        raw.clone()
    } else {
        roots
            .iter()
            .map(|r| r.join(path_str))
            .find(|p| p.exists())
            .ok_or_else(|| format!("filesystem.read: '{path_str}' not found in workspace roots"))?
    };

    // Canonicalize to resolve symlinks / `..` traversal
    let canonical = candidate
        .canonicalize()
        .map_err(|e| format!("filesystem.read: cannot resolve '{path_str}': {e}"))?;

    // Verify the resolved path lies within at least one workspace root
    let in_workspace = roots.iter().any(|root| {
        root.canonicalize()
            .map(|cr| canonical.starts_with(&cr))
            .unwrap_or(false)
    });
    if !in_workspace {
        return Err(format!(
            "filesystem.read: '{path_str}' resolves outside workspace roots"
        ));
    }

    // Block known sensitive paths regardless of workspace membership
    if looks_sensitive_path(&canonical) {
        return Err(format!(
            "filesystem.read: '{path_str}' is a sensitive path and cannot be read by the agent"
        ));
    }

    Ok(canonical)
}

#[cfg(test)]
mod tests {
    use super::*;

    // ─── P0.A / P0.B: sanitize_external_query ────────────────────────────────

    #[test]
    fn sanitize_external_query_passes_safe_query() {
        assert_eq!(
            sanitize_external_query("Rust async programming tutorial"),
            Ok("Rust async programming tutorial".into())
        );
    }

    #[test]
    fn sanitize_external_query_rejects_empty_query() {
        assert!(sanitize_external_query("   ").is_err());
    }

    #[test]
    fn sanitize_external_query_blocks_private_doc_names() {
        for term in ["CLAUDE.md", "tasks.md", "memory.md", "decisions.md"] {
            let result = sanitize_external_query(&format!("contents of {term}"));
            assert!(
                result.is_err(),
                "expected error for query containing '{term}'"
            );
        }
    }

    #[test]
    fn sanitize_external_query_blocks_secret_credential_terms() {
        for term in ["api_key", "password", "token", "api key", "secret"] {
            let result = sanitize_external_query(&format!("my {term} value"));
            assert!(
                result.is_err(),
                "expected error for query containing '{term}'"
            );
        }
    }

    #[test]
    fn sanitize_external_query_blocks_private_architecture_term() {
        let result = sanitize_external_query("explain private_architecture design");
        assert!(result.is_err());
    }

    // ─── P0.B: looks_sensitive_path ──────────────────────────────────────────

    #[test]
    fn looks_sensitive_path_detects_ssh_and_env_paths() {
        use std::path::Path;

        assert!(looks_sensitive_path(Path::new("/home/user/.ssh/id_rsa")));
        assert!(looks_sensitive_path(Path::new("/home/user/.env")));
        assert!(looks_sensitive_path(Path::new("C:/Users/user/.aws/credentials")));
        assert!(!looks_sensitive_path(Path::new("/home/user/projects/keynova/src/main.rs")));
    }
}
