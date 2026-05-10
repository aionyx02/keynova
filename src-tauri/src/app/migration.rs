use std::path::{Path, PathBuf};

use crate::platform_dirs::{keynova_config_dir, keynova_data_dir};

const MARKER_FILE: &str = ".migration_v1_complete";

/// Run once on startup: silently copy data from legacy cwd-based paths to the
/// correct OS data directories (introduced in 5.3.A/B).
///
/// The old code fell back to `PathBuf::from(".")` when APPDATA was unset or
/// on Linux/macOS where APPDATA doesn't exist, so data could have landed next
/// to the binary or in whatever the working directory happened to be.
///
/// After migration the marker file is written so this never runs again.
pub fn run_legacy_migration() {
    let new_data = keynova_data_dir();
    let marker = new_data.join(MARKER_FILE);
    if marker.exists() {
        return;
    }

    let candidates = legacy_candidates();
    for legacy_root in &candidates {
        if !legacy_root.exists() || legacy_root == &new_data {
            continue;
        }
        migrate_from(legacy_root);
    }

    // Always write the marker so we don't repeat the scan on every launch.
    let _ = std::fs::create_dir_all(&new_data);
    if let Err(e) = std::fs::write(&marker, "") {
        eprintln!("[keynova] migration: could not write marker: {e}");
    }
}

fn legacy_candidates() -> Vec<PathBuf> {
    let mut candidates = Vec::new();

    // 1. cwd-relative Keynova/ — the most common fallback path.
    if let Ok(cwd) = std::env::current_dir() {
        candidates.push(cwd.join("Keynova"));
    }

    // 2. Executable-sibling Keynova/ — covers portable / installed layouts.
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            candidates.push(dir.join("Keynova"));
        }
    }

    candidates
}

fn migrate_from(legacy_root: &Path) {
    eprintln!(
        "[keynova] migration: found legacy data at {} — migrating",
        legacy_root.display()
    );

    let new_config = keynova_config_dir();
    let new_data = keynova_data_dir();

    // config.toml
    copy_file_once(
        &legacy_root.join("config.toml"),
        &new_config.join("config.toml"),
    );

    // notes/
    copy_dir_once(&legacy_root.join("notes"), &new_data.join("notes"));

    // knowledge.db
    copy_file_once(
        &legacy_root.join("knowledge.db"),
        &new_data.join("knowledge.db"),
    );

    // Tantivy index is intentionally skipped — it is rebuilt automatically
    // by the background file indexer on startup.
}

/// Copy a single file only when the destination does not already exist.
fn copy_file_once(src: &Path, dst: &Path) {
    if !src.exists() || dst.exists() {
        return;
    }
    if let Some(parent) = dst.parent() {
        if let Err(e) = std::fs::create_dir_all(parent) {
            eprintln!("[keynova] migration: mkdir {}: {e}", parent.display());
            return;
        }
    }
    match std::fs::copy(src, dst) {
        Ok(_) => eprintln!("[keynova] migration: copied {}", src.display()),
        Err(e) => eprintln!("[keynova] migration: copy {} failed: {e}", src.display()),
    }
}

/// Recursively copy a directory, skipping files that already exist at dst.
fn copy_dir_once(src: &Path, dst: &Path) {
    if !src.exists() {
        return;
    }
    let Ok(entries) = std::fs::read_dir(src) else {
        return;
    };
    if let Err(e) = std::fs::create_dir_all(dst) {
        eprintln!("[keynova] migration: mkdir {}: {e}", dst.display());
        return;
    }
    for entry in entries.flatten() {
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if src_path.is_dir() {
            copy_dir_once(&src_path, &dst_path);
        } else {
            copy_file_once(&src_path, &dst_path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_crash_when_no_legacy_data() {
        // Should complete without panicking even when legacy paths don't exist.
        let candidates = legacy_candidates();
        for c in &candidates {
            // Verify we get PathBuf values, not nonsense.
            assert!(c.ends_with("Keynova"));
        }
    }

    #[test]
    fn copy_file_once_skips_existing_dst() {
        let tmp = std::env::temp_dir().join(format!(
            "keynova-migration-test-{}",
            uuid::Uuid::new_v4()
        ));
        std::fs::create_dir_all(&tmp).unwrap();
        let src = tmp.join("src.txt");
        let dst = tmp.join("dst.txt");
        std::fs::write(&src, "source").unwrap();
        std::fs::write(&dst, "original").unwrap();

        copy_file_once(&src, &dst);
        assert_eq!(std::fs::read_to_string(&dst).unwrap(), "original");
        let _ = std::fs::remove_dir_all(tmp);
    }

    #[test]
    fn copy_file_once_copies_when_dst_absent() {
        let tmp = std::env::temp_dir().join(format!(
            "keynova-migration-test-{}",
            uuid::Uuid::new_v4()
        ));
        std::fs::create_dir_all(&tmp).unwrap();
        let src = tmp.join("src.txt");
        let dst = tmp.join("dst.txt");
        std::fs::write(&src, "hello").unwrap();

        copy_file_once(&src, &dst);
        assert_eq!(std::fs::read_to_string(&dst).unwrap(), "hello");
        let _ = std::fs::remove_dir_all(tmp);
    }
}