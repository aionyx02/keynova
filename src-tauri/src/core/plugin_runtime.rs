#![allow(dead_code)]

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::models::plugin::{PluginAuditEntry, PluginManifest, PluginPermission};

#[derive(Debug, Clone)]
pub struct LoadedPlugin {
    pub manifest: PluginManifest,
    pub manifest_path: PathBuf,
    pub wasm_path: PathBuf,
}

pub struct PluginRuntime;

impl PluginRuntime {
    pub fn load_manifest(path: impl AsRef<Path>) -> Result<LoadedPlugin, String> {
        let path = path.as_ref();
        let source = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
        let extension = path.extension().and_then(|ext| ext.to_str()).unwrap_or("");
        let manifest = match extension {
            "json" => serde_json::from_str::<PluginManifest>(&source).map_err(|e| e.to_string())?,
            "toml" => toml::from_str::<PluginManifest>(&source).map_err(|e| e.to_string())?,
            _ => {
                return Err(format!(
                    "unsupported plugin manifest extension '{}'",
                    extension
                ))
            }
        };
        Self::validate_manifest(&manifest)?;
        let base_dir = path.parent().unwrap_or_else(|| Path::new("."));
        let wasm_path = base_dir.join(&manifest.wasm);
        Ok(LoadedPlugin {
            manifest,
            manifest_path: path.to_path_buf(),
            wasm_path,
        })
    }

    pub fn load_plugins(dir: impl AsRef<Path>) -> Result<Vec<LoadedPlugin>, String> {
        let dir = dir.as_ref();
        if !dir.exists() {
            return Ok(Vec::new());
        }

        let mut plugins = Vec::new();
        for entry in std::fs::read_dir(dir).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            let path = entry.path();
            let manifest_path = if path.is_dir() {
                ["plugin.toml", "plugin.json"]
                    .into_iter()
                    .map(|name| path.join(name))
                    .find(|candidate| candidate.exists())
            } else if is_manifest_file(&path) {
                Some(path)
            } else {
                None
            };
            if let Some(manifest_path) = manifest_path {
                plugins.push(Self::load_manifest(manifest_path)?);
            }
        }
        plugins.sort_by(|left, right| left.manifest.id.cmp(&right.manifest.id));
        Ok(plugins)
    }

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

fn is_manifest_file(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| matches!(name, "plugin.toml" | "plugin.json"))
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

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_plugin_dir(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("keynova-plugin-{name}-{}", uuid::Uuid::new_v4()))
    }

    #[test]
    fn loads_plugin_manifest_from_directory() {
        let root = temp_plugin_dir("loader");
        let plugin_dir = root.join("hello");
        std::fs::create_dir_all(&plugin_dir).unwrap();
        std::fs::write(
            plugin_dir.join("plugin.toml"),
            r#"
            id = "hello"
            name = "Hello"
            version = "0.1.0"
            wasm = "hello.wasm"
            permissions = ["log"]
            "#,
        )
        .unwrap();

        let plugins = PluginRuntime::load_plugins(&root).unwrap();
        assert_eq!(plugins.len(), 1);
        assert_eq!(plugins[0].manifest.id, "hello");
        assert_eq!(plugins[0].wasm_path, plugin_dir.join("hello.wasm"));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn rejects_manifest_with_dangerous_default_permission() {
        let root = temp_plugin_dir("deny");
        std::fs::create_dir_all(&root).unwrap();
        let manifest_path = root.join("plugin.toml");
        std::fs::write(
            &manifest_path,
            r#"
            id = "danger"
            name = "Danger"
            version = "0.1.0"
            wasm = "danger.wasm"
            permissions = ["shell"]
            "#,
        )
        .unwrap();

        let error = PluginRuntime::load_manifest(&manifest_path).unwrap_err();
        assert!(error.contains("denied by default"));
        let _ = std::fs::remove_dir_all(root);
    }
}
