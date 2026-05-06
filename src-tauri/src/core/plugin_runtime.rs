#![allow(dead_code)]

use std::collections::HashSet;

use crate::models::plugin::{PluginAuditEntry, PluginManifest, PluginPermission};

pub struct PluginRuntime;

impl PluginRuntime {
    pub fn validate_manifest(manifest: &PluginManifest) -> Result<(), String> {
        if manifest.id.trim().is_empty() {
            return Err("plugin id is required".to_string());
        }
        if manifest.wasm.trim().is_empty() {
            return Err("plugin wasm entrypoint is required".to_string());
        }
        deny_dangerous_defaults(&manifest.permissions)
    }

    pub fn check_permission(
        manifest: &PluginManifest,
        permission: PluginPermission,
        target: Option<String>,
    ) -> PluginAuditEntry {
        let allowed = manifest.permissions.iter().any(|p| p == &permission);
        PluginAuditEntry {
            plugin_id: manifest.id.clone(),
            host_call: format!("{permission:?}"),
            allowed,
            target,
            error: (!allowed).then(|| "permission denied".to_string()),
        }
    }
}

fn deny_dangerous_defaults(permissions: &[PluginPermission]) -> Result<(), String> {
    let requested: HashSet<_> = permissions.iter().collect();
    for denied in [
        PluginPermission::Filesystem,
        PluginPermission::Shell,
        PluginPermission::AiKey,
        PluginPermission::SystemControl,
    ] {
        if requested.contains(&denied) {
            return Err(format!(
                "{denied:?} is denied by default and requires a future explicit grant flow"
            ));
        }
    }
    Ok(())
}
