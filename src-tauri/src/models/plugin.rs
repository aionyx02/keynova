#![allow(dead_code)]

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum PluginPermission {
    Log,
    GetSetting,
    SearchWorkspace,
    ReadClipboard,
    HttpFetch,
    RunAction,
    Filesystem,
    Shell,
    AiKey,
    SystemControl,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginResourceLimits {
    pub memory_mb: u32,
    pub execution_ms: u64,
    pub host_call_count: u32,
    pub output_bytes: usize,
    pub search_result_count: usize,
}

impl Default for PluginResourceLimits {
    fn default() -> Self {
        Self {
            memory_mb: 32,
            execution_ms: 500,
            host_call_count: 64,
            output_bytes: 64 * 1024,
            search_result_count: 50,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    pub id: String,
    pub name: String,
    pub version: String,
    pub wasm: String,
    #[serde(default)]
    pub permissions: Vec<PluginPermission>,
    #[serde(default)]
    pub limits: PluginResourceLimits,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginAuditEntry {
    pub plugin_id: String,
    pub host_call: String,
    pub allowed: bool,
    pub target: Option<String>,
    pub error: Option<String>,
}
