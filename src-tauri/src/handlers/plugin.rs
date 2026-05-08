use serde_json::{json, Value};

use crate::core::plugin_runtime::PluginRuntime;
use crate::core::{CommandHandler, CommandResult};
use crate::models::plugin::{PluginManifest, PluginPermission};

/// Handles plugin manifest validation and permission checks.
pub struct PluginHandler;

impl CommandHandler for PluginHandler {
    fn namespace(&self) -> &'static str {
        "plugin"
    }

    fn execute(&self, command: &str, payload: Value) -> CommandResult {
        match command {
            "validate_manifest" => {
                let manifest: PluginManifest =
                    serde_json::from_value(payload).map_err(|e| e.to_string())?;
                PluginRuntime::validate_manifest(&manifest)?;
                Ok(json!({ "ok": true }))
            }
            "check_permission" => {
                let manifest_value = payload
                    .get("manifest")
                    .cloned()
                    .ok_or_else(|| "missing 'manifest'".to_string())?;
                let manifest: PluginManifest =
                    serde_json::from_value(manifest_value).map_err(|e| e.to_string())?;
                let permission_value = payload
                    .get("permission")
                    .cloned()
                    .ok_or_else(|| "missing 'permission'".to_string())?;
                let permission: PluginPermission =
                    serde_json::from_value(permission_value).map_err(|e| e.to_string())?;
                let target = payload
                    .get("target")
                    .and_then(Value::as_str)
                    .map(ToOwned::to_owned);
                Ok(json!(PluginRuntime::check_permission(
                    &manifest, permission, target
                )))
            }
            _ => Err(format!("unknown plugin command '{command}'")),
        }
    }
}
