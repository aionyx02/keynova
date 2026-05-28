use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use crate::core::preview::{classify_path, guess_image_mime, read_text_preview, PreviewKind};
use crate::core::{CommandHandler, CommandResult};
use crate::models::ipc_requests::{
    FileDeleteRequest, FileHashRequest, FileMoveRequest, FileOpenAsTextRequest, FilePreviewRequest,
    FileRenameRequest,
};

/// Handles `file.*` IPC commands invoked from the launcher's secondary action menu.
///
/// `reveal` and `open_with` open via `tauri-plugin-opener`. Destructive ops
/// (`rename` / `move` / `delete`) use a two-phase confirm gate: callers must
/// include `"confirm": true` to actually mutate; otherwise the handler returns
/// a preview JSON describing what *would* happen.
pub struct FileHandler;

impl FileHandler {
    pub fn new() -> Self {
        Self
    }
}

impl Default for FileHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl CommandHandler for FileHandler {
    fn namespace(&self) -> &'static str {
        "file"
    }

    fn execute(&self, command: &str, payload: Value) -> CommandResult {
        match command {
            "reveal" => {
                let path = require_path(&payload)?;
                tauri_plugin_opener::reveal_item_in_dir(Path::new(&path))
                    .map_err(|e| format!("reveal failed: {e}"))?;
                Ok(json!({ "ok": true, "path": path }))
            }
            "open_with" => {
                let path = require_path(&payload)?;
                let with = payload
                    .get("with")
                    .and_then(Value::as_str)
                    .map(|s| s.trim())
                    .filter(|s| !s.is_empty())
                    .map(str::to_string);
                tauri_plugin_opener::open_path(Path::new(&path), with.as_deref())
                    .map_err(|e| format!("open_with failed: {e}"))?;
                Ok(json!({ "ok": true, "path": path, "with": with }))
            }
            "open_as_text" => {
                let req: FileOpenAsTextRequest = serde_json::from_value(payload)
                    .map_err(|e| format!("invalid file.open_as_text request: {e}"))?;
                let path = trim_path(&req.path)?;
                ensure_path_exists(&path)?;
                let editor = text_editor_for_platform();
                tauri_plugin_opener::open_path(Path::new(&path), editor)
                    .map_err(|e| format!("open_as_text failed: {e}"))?;
                Ok(json!({ "ok": true, "path": path, "with": editor }))
            }
            "rename" => {
                let req: FileRenameRequest = serde_json::from_value(payload)
                    .map_err(|e| format!("invalid file.rename request: {e}"))?;
                let source = trim_path(&req.path)?;
                let source_path = PathBuf::from(&source);
                ensure_path_exists(&source)?;
                let new_name = validate_new_name(&req.new_name)?;
                let parent = source_path
                    .parent()
                    .ok_or_else(|| "cannot rename root path".to_string())?;
                let target = parent.join(&new_name);
                if target.exists() {
                    return Err(format!("target already exists: {}", target.display()));
                }
                if !req.confirm {
                    return Ok(json!({
                        "preview": true,
                        "source": source,
                        "target": target.to_string_lossy(),
                    }));
                }
                fs::rename(&source_path, &target).map_err(|e| format!("rename failed: {e}"))?;
                verify_rename_landed(&source_path, &target)?;
                Ok(json!({
                    "ok": true,
                    "source": source,
                    "target": target.to_string_lossy(),
                }))
            }
            "move" => {
                let req: FileMoveRequest = serde_json::from_value(payload)
                    .map_err(|e| format!("invalid file.move request: {e}"))?;
                let source = trim_path(&req.path)?;
                let source_path = PathBuf::from(&source);
                ensure_path_exists(&source)?;
                let target_dir = trim_path(&req.target_dir)?;
                let target_dir_path = PathBuf::from(&target_dir);
                if !target_dir_path.is_dir() {
                    return Err(format!("target_dir is not a directory: {target_dir}"));
                }
                let file_name = source_path
                    .file_name()
                    .ok_or_else(|| "cannot move root path".to_string())?;
                let target = target_dir_path.join(file_name);
                let target_exists = target.exists();
                if target_exists && !req.overwrite {
                    return Err(format!("target already exists: {}", target.display()));
                }
                if !req.confirm {
                    return Ok(json!({
                        "preview": true,
                        "source": source,
                        "target": target.to_string_lossy(),
                        "overwrite": target_exists,
                    }));
                }
                fs::rename(&source_path, &target).map_err(|e| format!("move failed: {e}"))?;
                verify_rename_landed(&source_path, &target)?;
                Ok(json!({
                    "ok": true,
                    "source": source,
                    "target": target.to_string_lossy(),
                }))
            }
            "delete" => {
                let req: FileDeleteRequest = serde_json::from_value(payload)
                    .map_err(|e| format!("invalid file.delete request: {e}"))?;
                let path = trim_path(&req.path)?;
                let target = PathBuf::from(&path);
                trace_delete(
                    "request",
                    &path,
                    &format!("confirm={} raw_path={:?}", req.confirm, req.path),
                );
                let metadata = target.metadata().map_err(|e| {
                    trace_delete("metadata_err", &path, &format!("{e}"));
                    format!("cannot inspect path: {e}")
                })?;
                let kind = if metadata.is_dir() { "folder" } else { "file" };
                let size: Option<u64> = if metadata.is_file() {
                    Some(metadata.len())
                } else {
                    None
                };
                trace_delete(
                    "metadata_ok",
                    &path,
                    &format!(
                        "kind={kind} size={size:?} attrs={} symlink={}",
                        file_attributes_string(&metadata),
                        symlink_meta_summary(&target),
                    ),
                );
                if !req.confirm {
                    trace_delete("preview", &path, "returning preview JSON");
                    return Ok(json!({
                        "preview": true,
                        "path": path,
                        "size": size,
                        "kind": kind,
                        "destination": "recycle_bin",
                    }));
                }
                trace_delete("trash_invoke", &path, "calling trash::delete");
                trash::delete(&target).map_err(|e| {
                    trace_delete("trash_err", &path, &format!("{e}"));
                    format!("delete failed: {e}")
                })?;
                trace_delete("trash_ok", &path, "trash::delete returned Ok");
                // `trash` v5 on Windows wraps SHFileOperationW which is known to
                // return Ok() without actually moving the file in some edge
                // cases (file in use, Recycle Bin disabled on volume, network
                // drives, permission quirks). Verify the path is gone before
                // reporting success — otherwise the launcher hides the row
                // (kill set) while the file silently remains on disk.
                verify_path_removed(&target).map_err(|reason| {
                    trace_delete("verify_err", &path, &reason);
                    format!(
                        "trash returned ok but {}. Possible causes: file is open in another app, \
                         Recycle Bin disabled on this volume, insufficient permissions, \
                         or the path is on a network share without trash support. \
                         Path: {}",
                        reason, path
                    )
                })?;
                trace_delete("verify_ok", &path, "post-trash symlink_metadata=NotFound");
                Ok(json!({
                    "ok": true,
                    "path": path,
                    "kind": kind,
                    "destination": "recycle_bin",
                }))
            }
            "hash" => {
                let req: FileHashRequest = serde_json::from_value(payload)
                    .map_err(|e| format!("invalid file.hash request: {e}"))?;
                let algorithm = req.algorithm.to_ascii_lowercase();
                if algorithm != "sha256" {
                    return Err(format!("unsupported algorithm: {algorithm}"));
                }
                let path = trim_path(&req.path)?;
                let (hex, bytes) = stream_sha256(Path::new(&path))?;
                Ok(json!({
                    "algorithm": "sha256",
                    "path": path,
                    "hex": hex,
                    "bytes": bytes,
                }))
            }
            "preview" => {
                let req: FilePreviewRequest = serde_json::from_value(payload)
                    .map_err(|e| format!("invalid file.preview request: {e}"))?;
                let path = trim_path(&req.path)?;
                ensure_path_exists(&path)?;
                let p = Path::new(&path);
                let meta = fs::metadata(p).map_err(|e| format!("metadata failed: {e}"))?;
                let size_bytes = meta.len();
                let modified_ms = meta
                    .modified()
                    .ok()
                    .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                    .map(|d| d.as_millis() as i64);
                let kind = classify_path(p, &meta);
                match kind {
                    PreviewKind::Image => Ok(json!({
                        "path": path,
                        "kind": "image",
                        "size_bytes": size_bytes,
                        "modified_ms": modified_ms,
                        "mime": guess_image_mime(p),
                        "truncated": false,
                    })),
                    PreviewKind::Text => {
                        let max_bytes = req.max_bytes.unwrap_or(4096).min(64 * 1024);
                        let max_lines = req.max_lines.unwrap_or(500).min(2000);
                        let preview = read_text_preview(p, max_bytes, max_lines, true)?;
                        Ok(json!({
                            "path": path,
                            "kind": "text",
                            "size_bytes": size_bytes,
                            "modified_ms": modified_ms,
                            "content": preview.content,
                            "truncated": preview.truncated,
                            "line_count": preview.line_count,
                        }))
                    }
                    PreviewKind::Binary => Ok(json!({
                        "path": path,
                        "kind": "binary",
                        "size_bytes": size_bytes,
                        "modified_ms": modified_ms,
                        "truncated": false,
                    })),
                }
            }
            _ => Err(format!("unknown file command '{command}'")),
        }
    }
}

fn require_path(payload: &Value) -> Result<String, String> {
    payload
        .get("path")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .ok_or_else(|| "missing or empty 'path'".to_string())
}

fn trim_path(raw: &str) -> Result<String, String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        Err("missing or empty 'path'".to_string())
    } else {
        Ok(trimmed.to_string())
    }
}

fn ensure_path_exists(path: &str) -> Result<(), String> {
    if Path::new(path).exists() {
        Ok(())
    } else {
        Err(format!("path does not exist: {path}"))
    }
}

fn validate_new_name(raw: &str) -> Result<String, String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err("new_name is empty".to_string());
    }
    if trimmed == "." || trimmed == ".." {
        return Err("new_name cannot be '.' or '..'".to_string());
    }
    if trimmed.contains('/') || trimmed.contains('\\') {
        return Err("new_name cannot contain path separators".to_string());
    }
    Ok(trimmed.to_string())
}

fn text_editor_for_platform() -> Option<&'static str> {
    if cfg!(target_os = "windows") {
        Some("notepad.exe")
    } else if cfg!(target_os = "macos") {
        Some("TextEdit")
    } else {
        None
    }
}

/// Debug-only delete-flow tracer. Bug B 2026-05-19 — adds step-by-step
/// `eprintln!` to stderr so a future user repro can be diagnosed without
/// guessing. Format intentionally machine-parseable (`key=value`) so a user
/// can paste a block and we can pattern-match. No-op in release builds.
fn trace_delete(stage: &str, path: &str, detail: &str) {
    #[cfg(debug_assertions)]
    {
        eprintln!("[keynova][file.delete] stage={stage} path={path:?} {detail}");
    }
    #[cfg(not(debug_assertions))]
    {
        let _ = (stage, path, detail);
    }
}

/// Decode Windows `FILE_ATTRIBUTE_*` bits into a readable flag list. On
/// non-Windows we fall back to a permissions debug string. Used purely for
/// debug tracing — see `trace_delete`.
#[cfg(target_os = "windows")]
fn file_attributes_string(metadata: &fs::Metadata) -> String {
    use std::os::windows::fs::MetadataExt;
    let attrs = metadata.file_attributes();
    let mut flags: Vec<&'static str> = Vec::new();
    if attrs & 0x0000_0001 != 0 {
        flags.push("READONLY");
    }
    if attrs & 0x0000_0002 != 0 {
        flags.push("HIDDEN");
    }
    if attrs & 0x0000_0004 != 0 {
        flags.push("SYSTEM");
    }
    if attrs & 0x0000_0010 != 0 {
        flags.push("DIRECTORY");
    }
    if attrs & 0x0000_0020 != 0 {
        flags.push("ARCHIVE");
    }
    if attrs & 0x0000_0400 != 0 {
        flags.push("REPARSE_POINT");
    }
    if attrs & 0x0000_1000 != 0 {
        flags.push("OFFLINE");
    }
    if attrs & 0x0008_0000 != 0 {
        flags.push("PINNED");
    }
    if attrs & 0x0010_0000 != 0 {
        flags.push("UNPINNED");
    }
    if attrs & 0x0040_0000 != 0 {
        flags.push("RECALL_ON_DATA_ACCESS");
    }
    format!("0x{attrs:X}[{}]", flags.join("|"))
}

#[cfg(not(target_os = "windows"))]
fn file_attributes_string(metadata: &fs::Metadata) -> String {
    format!("perms={:?}", metadata.permissions())
}

/// Short summary of `symlink_metadata` for the trace log (vs `metadata`,
/// follows-links). Helps distinguish "the path is a symlink/junction" cases.
fn symlink_meta_summary(target: &Path) -> String {
    match fs::symlink_metadata(target) {
        Ok(m) => {
            let t = m.file_type();
            format!(
                "ok(file={} dir={} symlink={})",
                t.is_file(),
                t.is_dir(),
                t.is_symlink(),
            )
        }
        Err(e) => format!("err({e})"),
    }
}

/// Post-mutation verifier for destructive ops: ensures the original path is
/// gone after `trash::delete` (which can return Ok without actually moving
/// the file on Windows). Returns `Ok(())` when the path is truly removed,
/// `Err(reason)` when the file/folder still exists.
fn verify_path_removed(target: &Path) -> Result<(), String> {
    // `Path::exists` follows symlinks; for trash purposes we want to know
    // whether ANY entry (file/dir/symlink) is still resolvable at this path.
    // `symlink_metadata` reports the entry without following, so a broken
    // symlink would still be considered "present" — which matches the user's
    // expectation (they asked to delete this entry).
    match fs::symlink_metadata(target) {
        Ok(_) => Err(format!("file still exists at {}", target.display())),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(format!(
            "could not verify removal of {}: {e}",
            target.display()
        )),
    }
}

/// Post-rename verifier: source must no longer exist and target must.
fn verify_rename_landed(source: &Path, target: &Path) -> Result<(), String> {
    if source.exists() {
        return Err(format!(
            "rename returned ok but source still exists at {}",
            source.display()
        ));
    }
    if !target.exists() {
        return Err(format!(
            "rename returned ok but target was not created at {}",
            target.display()
        ));
    }
    Ok(())
}

fn stream_sha256(path: &Path) -> Result<(String, u64), String> {
    let mut file = fs::File::open(path).map_err(|e| format!("open failed: {e}"))?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 64 * 1024];
    let mut total: u64 = 0;
    loop {
        let n = file
            .read(&mut buf)
            .map_err(|e| format!("read failed: {e}"))?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
        total += n as u64;
    }
    let hex = format!("{:x}", hasher.finalize());
    Ok((hex, total))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::TempDir;

    fn handler() -> FileHandler {
        FileHandler::new()
    }

    fn tmp_file(dir: &TempDir, name: &str, contents: &[u8]) -> PathBuf {
        let path = dir.path().join(name);
        let mut f = File::create(&path).expect("create temp file");
        f.write_all(contents).expect("write temp file");
        path
    }

    #[test]
    fn namespace_is_file() {
        assert_eq!(handler().namespace(), "file");
    }

    #[test]
    fn reveal_requires_path() {
        let err = handler()
            .execute("reveal", json!({}))
            .expect_err("missing path should error");
        assert!(err.contains("path"), "unexpected error: {err}");
    }

    #[test]
    fn reveal_rejects_empty_path() {
        let err = handler()
            .execute("reveal", json!({ "path": "   " }))
            .expect_err("empty path should error");
        assert!(err.contains("path"), "unexpected error: {err}");
    }

    #[test]
    fn open_with_requires_path() {
        let err = handler()
            .execute("open_with", json!({ "with": "notepad" }))
            .expect_err("missing path should error");
        assert!(err.contains("path"), "unexpected error: {err}");
    }

    #[test]
    fn unknown_command_returns_explicit_error() {
        let err = handler()
            .execute("teleport", json!({}))
            .expect_err("unknown command should error");
        assert!(
            err.contains("unknown file command"),
            "unexpected error: {err}"
        );
        assert!(err.contains("teleport"), "error must mention command name");
    }

    #[test]
    fn require_path_trims_whitespace() {
        let payload = json!({ "path": "  /tmp/foo  " });
        let path = require_path(&payload).expect("path present");
        assert_eq!(path, "/tmp/foo");
    }

    // ── rename ───────────────────────────────────────────────────────────────

    #[test]
    fn rename_preview_when_confirm_false() {
        let dir = TempDir::new().unwrap();
        let path = tmp_file(&dir, "a.txt", b"hi");
        let res = handler()
            .execute(
                "rename",
                json!({ "path": path.to_string_lossy(), "new_name": "b.txt" }),
            )
            .expect("preview ok");
        assert_eq!(res["preview"], json!(true));
        assert!(path.exists(), "preview must not move the file");
        assert!(res["target"].as_str().unwrap().ends_with("b.txt"));
    }

    #[test]
    fn rename_executes_when_confirm_true() {
        let dir = TempDir::new().unwrap();
        let path = tmp_file(&dir, "a.txt", b"hi");
        let res = handler()
            .execute(
                "rename",
                json!({ "path": path.to_string_lossy(), "new_name": "b.txt", "confirm": true }),
            )
            .expect("rename ok");
        assert_eq!(res["ok"], json!(true));
        assert!(!path.exists());
        assert!(dir.path().join("b.txt").exists());
    }

    #[test]
    fn rename_rejects_existing_target() {
        let dir = TempDir::new().unwrap();
        let path = tmp_file(&dir, "a.txt", b"hi");
        tmp_file(&dir, "b.txt", b"there");
        let err = handler()
            .execute(
                "rename",
                json!({ "path": path.to_string_lossy(), "new_name": "b.txt", "confirm": true }),
            )
            .expect_err("should reject existing target");
        assert!(err.contains("already exists"), "unexpected: {err}");
    }

    #[test]
    fn rename_rejects_path_separator_in_new_name() {
        let dir = TempDir::new().unwrap();
        let path = tmp_file(&dir, "a.txt", b"hi");
        let err = handler()
            .execute(
                "rename",
                json!({ "path": path.to_string_lossy(), "new_name": "x/y.txt" }),
            )
            .expect_err("should reject separator");
        assert!(err.contains("separator"), "unexpected: {err}");
    }

    // ── move ─────────────────────────────────────────────────────────────────

    #[test]
    fn move_preview_returns_source_and_target() {
        let dir = TempDir::new().unwrap();
        let path = tmp_file(&dir, "a.txt", b"hi");
        let dest = dir.path().join("sub");
        fs::create_dir(&dest).unwrap();
        let res = handler()
            .execute(
                "move",
                json!({
                    "path": path.to_string_lossy(),
                    "target_dir": dest.to_string_lossy(),
                }),
            )
            .expect("preview ok");
        assert_eq!(res["preview"], json!(true));
        assert!(path.exists(), "preview must not move the file");
        assert!(res["target"].as_str().unwrap().ends_with("a.txt"));
    }

    #[test]
    fn move_rejects_non_directory_target() {
        let dir = TempDir::new().unwrap();
        let path = tmp_file(&dir, "a.txt", b"hi");
        let not_a_dir = tmp_file(&dir, "b.txt", b"x");
        let err = handler()
            .execute(
                "move",
                json!({
                    "path": path.to_string_lossy(),
                    "target_dir": not_a_dir.to_string_lossy(),
                }),
            )
            .expect_err("should reject non-directory");
        assert!(err.contains("not a directory"), "unexpected: {err}");
    }

    // ── delete ───────────────────────────────────────────────────────────────

    #[test]
    fn delete_preview_reports_destination_recycle_bin() {
        let dir = TempDir::new().unwrap();
        let path = tmp_file(&dir, "a.txt", b"hello");
        let res = handler()
            .execute("delete", json!({ "path": path.to_string_lossy() }))
            .expect("preview ok");
        assert_eq!(res["preview"], json!(true));
        assert_eq!(res["destination"], json!("recycle_bin"));
        assert_eq!(res["kind"], json!("file"));
        assert_eq!(res["size"], json!(5));
        assert!(path.exists(), "preview must not delete the file");
    }

    #[test]
    #[ignore = "requires desktop session for trash API; run manually"]
    fn delete_moves_to_trash_when_confirmed() {
        let dir = TempDir::new().unwrap();
        let path = tmp_file(&dir, "a.txt", b"hi");
        let res = handler()
            .execute(
                "delete",
                json!({ "path": path.to_string_lossy(), "confirm": true }),
            )
            .expect("trash ok");
        assert_eq!(res["ok"], json!(true));
        assert!(!path.exists());
    }

    /// Bug B 2026-05-19 repro — exercises `delete` against a path supplied via
    /// `KEYNOVA_REPRO_DELETE_PATH` env var. Used to test paths the standard
    /// tempdir-based test can't reach: OneDrive-synced Desktop, network drives,
    /// junction points, etc. Test prints the FileHandler reply and the post-
    /// delete `symlink_metadata` result so a regression can be diagnosed
    /// without needing the GUI. Always-ignored — run manually:
    ///
    ///   $env:KEYNOVA_REPRO_DELETE_PATH = "C:\Users\shawn\OneDrive\Desktop\foo.txt"
    ///   cargo test --manifest-path src-tauri/Cargo.toml -- --ignored --nocapture repro_delete_path_from_env
    #[test]
    #[ignore = "manual repro driver, set KEYNOVA_REPRO_DELETE_PATH"]
    fn repro_delete_path_from_env() {
        let path = std::env::var("KEYNOVA_REPRO_DELETE_PATH")
            .expect("set KEYNOVA_REPRO_DELETE_PATH to absolute path of a file you can lose");
        let p = PathBuf::from(&path);
        println!("[repro] target: {}", path);
        println!("[repro] pre  Path::exists = {}", p.exists());
        let pre_meta = fs::symlink_metadata(&p);
        println!("[repro] pre  symlink_metadata: {:?}", pre_meta);
        let res = handler().execute("delete", json!({ "path": path, "confirm": true }));
        println!("[repro] FileHandler reply: {:?}", res);
        let post_meta = fs::symlink_metadata(&p);
        println!("[repro] post symlink_metadata: {:?}", post_meta);
        println!("[repro] Path::exists post-call = {}", p.exists());
        // Don't assert — we want raw observation. The user can inspect output.
    }

    // ── hash ─────────────────────────────────────────────────────────────────

    #[test]
    fn hash_sha256_matches_known_vector() {
        let dir = TempDir::new().unwrap();
        let path = tmp_file(&dir, "abc.txt", b"abc");
        let res = handler()
            .execute("hash", json!({ "path": path.to_string_lossy() }))
            .expect("hash ok");
        assert_eq!(res["algorithm"], json!("sha256"));
        assert_eq!(res["bytes"], json!(3));
        assert_eq!(
            res["hex"].as_str().unwrap(),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn hash_rejects_unknown_algorithm() {
        let dir = TempDir::new().unwrap();
        let path = tmp_file(&dir, "a.txt", b"hi");
        let err = handler()
            .execute(
                "hash",
                json!({ "path": path.to_string_lossy(), "algorithm": "md5" }),
            )
            .expect_err("should reject md5");
        assert!(err.contains("unsupported"), "unexpected: {err}");
    }

    // ── open_as_text ─────────────────────────────────────────────────────────

    #[test]
    fn open_as_text_requires_path() {
        let err = handler()
            .execute("open_as_text", json!({}))
            .expect_err("missing path should error");
        assert!(
            err.contains("missing field") || err.contains("path"),
            "unexpected: {err}"
        );
    }

    // ── verify_path_removed / verify_rename_landed (bug-fix 2026-05-18) ─────

    #[test]
    fn verify_path_removed_reports_present_file() {
        let dir = TempDir::new().unwrap();
        let p = tmp_file(&dir, "still-here.txt", b"x");
        let err = verify_path_removed(&p).expect_err("file present should error");
        assert!(err.contains("still exists"), "unexpected: {err}");
    }

    #[test]
    fn verify_path_removed_ok_when_missing() {
        let dir = TempDir::new().unwrap();
        let p = dir.path().join("never-existed.txt");
        verify_path_removed(&p).expect("missing path should be Ok");
    }

    #[test]
    fn verify_rename_landed_detects_no_move() {
        let dir = TempDir::new().unwrap();
        let src = tmp_file(&dir, "src.txt", b"x");
        let dst = dir.path().join("dst.txt");
        // src still exists, dst missing → both checks should fail. The function
        // surfaces the source-still-exists branch first.
        let err = verify_rename_landed(&src, &dst).expect_err("should error");
        assert!(err.contains("source still exists"), "unexpected: {err}");
    }

    #[test]
    fn verify_rename_landed_ok_when_moved() {
        let dir = TempDir::new().unwrap();
        let src_real = tmp_file(&dir, "real.txt", b"x");
        let src_phantom = dir.path().join("phantom.txt"); // never created
                                                          // Pretend `src_phantom` was renamed to `src_real`: phantom missing,
                                                          // real present — verify is happy.
        verify_rename_landed(&src_phantom, &src_real).expect("ok when src gone, dst exists");
    }

    // ── preview (LAUNCH.1.C) ─────────────────────────────────────────────────

    #[test]
    fn preview_text_file_returns_content_with_size_and_kind() {
        let dir = TempDir::new().unwrap();
        let path = tmp_file(&dir, "note.md", b"# heading\nbody line\n");
        let res = handler()
            .execute("preview", json!({ "path": path.to_string_lossy() }))
            .expect("preview ok");
        assert_eq!(res["kind"], json!("text"));
        assert!(res["content"].as_str().unwrap().contains("heading"));
        assert_eq!(res["size_bytes"], json!(20));
        assert_eq!(res["truncated"], json!(false));
        assert!(res["line_count"].as_i64().unwrap() >= 1);
    }

    #[test]
    fn preview_image_returns_kind_image_no_content() {
        let dir = TempDir::new().unwrap();
        let path = tmp_file(&dir, "a.png", b"\x89PNG\r\n\x1a\n");
        let res = handler()
            .execute("preview", json!({ "path": path.to_string_lossy() }))
            .expect("preview ok");
        assert_eq!(res["kind"], json!("image"));
        assert_eq!(res["mime"], json!("image/png"));
        assert!(
            res.get("content").is_none(),
            "image preview must not include content"
        );
    }

    #[test]
    fn preview_binary_returns_kind_binary_no_content() {
        let dir = TempDir::new().unwrap();
        // High-non-printable content with no extension → sniff classifies as binary.
        let content: Vec<u8> = (0..=255u16).map(|i| i as u8).collect();
        let path = tmp_file(&dir, "blob", &content);
        let res = handler()
            .execute("preview", json!({ "path": path.to_string_lossy() }))
            .expect("preview ok");
        assert_eq!(res["kind"], json!("binary"));
        assert!(res.get("content").is_none());
    }

    #[test]
    fn preview_truncates_when_file_exceeds_cap() {
        let dir = TempDir::new().unwrap();
        let body: String = (0..2000).map(|i| format!("line {i}\n")).collect();
        let path = tmp_file(&dir, "big.txt", body.as_bytes());
        let res = handler()
            .execute(
                "preview",
                json!({ "path": path.to_string_lossy(), "max_bytes": 256, "max_lines": 50 }),
            )
            .expect("preview ok");
        assert_eq!(res["kind"], json!("text"));
        assert_eq!(res["truncated"], json!(true));
    }

    #[test]
    fn preview_missing_path_errors() {
        let err = handler()
            .execute("preview", json!({}))
            .expect_err("missing path should error");
        assert!(
            err.contains("missing field") || err.contains("path"),
            "unexpected: {err}"
        );
    }

    #[test]
    fn preview_clamps_oversized_max_bytes() {
        let dir = TempDir::new().unwrap();
        let body = vec![b'a'; 200_000];
        let path = tmp_file(&dir, "big.txt", &body);
        let res = handler()
            .execute(
                "preview",
                json!({ "path": path.to_string_lossy(), "max_bytes": 1_000_000 }),
            )
            .expect("preview ok");
        // Even though the request asked for 1 MB, the handler caps to 64 KiB.
        let content_len = res["content"].as_str().unwrap().len();
        assert!(
            content_len <= 64 * 1024,
            "content should be capped at 64 KiB, got {content_len}"
        );
    }
}
