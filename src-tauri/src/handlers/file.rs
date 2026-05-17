use std::path::Path;

use serde_json::{json, Value};

use crate::core::{CommandHandler, CommandResult};

/// Handles `file.*` IPC commands invoked from the launcher's secondary action menu.
///
/// Slice 2 (LAUNCH.1.A backend): `reveal` and `open_with`, both backed by
/// `tauri-plugin-opener`'s desktop-shell APIs. Destructive ops (`rename`/`move`/`delete`/`hash`)
/// arrive in Slice 3 and must include `"confirm": true` in their payload.
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

#[cfg(test)]
mod tests {
    use super::*;

    fn handler() -> FileHandler {
        FileHandler::new()
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
        assert!(err.contains("unknown file command"), "unexpected error: {err}");
        assert!(err.contains("teleport"), "error must mention command name");
    }

    #[test]
    fn require_path_trims_whitespace() {
        let payload = json!({ "path": "  /tmp/foo  " });
        let path = require_path(&payload).expect("path present");
        assert_eq!(path, "/tmp/foo");
    }
}
