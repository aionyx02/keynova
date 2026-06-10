use serde::Serialize;

/// Primitive setting type used to generate UI fields and command suggestions.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SettingValueType {
    String,
    Integer,
    Boolean,
    Hotkey,
    Secret,
}

/// Schema metadata for one flat configuration key.
#[derive(Debug, Clone, Serialize)]
pub struct SettingSchema {
    pub key: &'static str,
    pub section: &'static str,
    pub label: &'static str,
    pub value_type: SettingValueType,
    pub default_value: &'static str,
    pub sensitive: bool,
    pub options: &'static [&'static str],
}

impl SettingSchema {
    pub const fn new(
        key: &'static str,
        section: &'static str,
        label: &'static str,
        value_type: SettingValueType,
        default_value: &'static str,
        sensitive: bool,
        options: &'static [&'static str],
    ) -> Self {
        Self {
            key,
            section,
            label,
            value_type,
            default_value,
            sensitive,
            options,
        }
    }
}

/// Returns the built-in setting schema in stable display order.
pub fn builtin_setting_schema() -> Vec<SettingSchema> {
    use SettingValueType::{Boolean, Hotkey, Integer, Secret, String};

    vec![
        SettingSchema::new(
            "hotkeys.app_launcher",
            "hotkeys",
            "App launcher",
            Hotkey,
            "Ctrl+K",
            false,
            &[],
        ),
        SettingSchema::new(
            "hotkeys.workspace_1",
            "hotkeys",
            "Workspace 1",
            Hotkey,
            "Ctrl+Alt+1",
            false,
            &[],
        ),
        SettingSchema::new(
            "hotkeys.workspace_2",
            "hotkeys",
            "Workspace 2",
            Hotkey,
            "Ctrl+Alt+2",
            false,
            &[],
        ),
        SettingSchema::new(
            "hotkeys.workspace_3",
            "hotkeys",
            "Workspace 3",
            Hotkey,
            "Ctrl+Alt+3",
            false,
            &[],
        ),
        SettingSchema::new(
            "hotkeys.workspace_cycle",
            "hotkeys",
            "Workspace cycle (next)",
            Hotkey,
            "Ctrl+Alt+0",
            false,
            &[],
        ),
        SettingSchema::new(
            "launcher.max_results",
            "launcher",
            "Max results",
            Integer,
            "10",
            false,
            &[],
        ),
        SettingSchema::new(
            "launcher.cache_ttl_secs",
            "launcher",
            "Cache TTL",
            Integer,
            "300",
            false,
            &[],
        ),
        SettingSchema::new(
            "launcher.auto_start_on_login",
            "launcher",
            "Launch on startup",
            Boolean,
            "true",
            false,
            &["true", "false"],
        ),
        SettingSchema::new(
            "search.backend",
            "search",
            "Search backend",
            String,
            "auto",
            false,
            &["auto", "everything", "app_cache", "tantivy"],
        ),
        SettingSchema::new(
            "search.index_dir",
            "search",
            "Search index directory",
            String,
            "",
            false,
            &[],
        ),
        SettingSchema::new(
            "search.exclude_dirs",
            "search",
            "Search excluded directories",
            String,
            "AppData,node_modules,.git,target,dist",
            false,
            &[],
        ),
        SettingSchema::new(
            "search.max_file_results",
            "search",
            "Max file results",
            Integer,
            "50",
            false,
            &[],
        ),
        SettingSchema::new(
            "search.preview_enabled",
            "search",
            "Preview pane enabled",
            Boolean,
            "true",
            false,
            &["true", "false"],
        ),
        SettingSchema::new(
            "search.show_rank_breakdown",
            "search",
            "Show rank breakdown tooltip",
            Boolean,
            "true",
            false,
            &["true", "false"],
        ),
        SettingSchema::new(
            "terminal.default_shell",
            "terminal",
            "Default shell",
            String,
            "",
            false,
            &[],
        ),
        SettingSchema::new(
            "terminal.font_size",
            "terminal",
            "Font size",
            Integer,
            "14",
            false,
            &[],
        ),
        SettingSchema::new(
            "terminal.scrollback_lines",
            "terminal",
            "Scrollback lines",
            Integer,
            "1000",
            false,
            &[],
        ),
        SettingSchema::new(
            "features.ai",
            "features",
            "AI Assistant (inline)",
            Boolean,
            "false",
            false,
            &["true", "false"],
        ),
        SettingSchema::new(
            "features.translation",
            "features",
            "Translation",
            Boolean,
            "true",
            false,
            &["true", "false"],
        ),
        SettingSchema::new(
            "features.notes",
            "features",
            "Notes",
            Boolean,
            "true",
            false,
            &["true", "false"],
        ),
        SettingSchema::new(
            "features.history",
            "features",
            "History",
            Boolean,
            "true",
            false,
            &["true", "false"],
        ),
        SettingSchema::new(
            "features.calculator",
            "features",
            "Calculator",
            Boolean,
            "true",
            false,
            &["true", "false"],
        ),
        SettingSchema::new(
            "features.system",
            "features",
            "System",
            Boolean,
            "true",
            false,
            &["true", "false"],
        ),
        SettingSchema::new(
            "ai.provider",
            "ai",
            "Provider",
            String,
            "claude",
            false,
            &["claude", "ollama", "openai"],
        ),
        SettingSchema::new("ai.api_key", "ai", "Claude API key", Secret, "", true, &[]),
        SettingSchema::new(
            "ai.model",
            "ai",
            "Claude model",
            String,
            "claude-sonnet-4-6",
            false,
            &[],
        ),
        SettingSchema::new(
            "ai.claude_model",
            "ai",
            "Claude model alias",
            String,
            "claude-sonnet-4-6",
            false,
            &[],
        ),
        SettingSchema::new(
            "ai.ollama_url",
            "ai",
            "Ollama URL",
            String,
            "http://localhost:11434",
            false,
            &[],
        ),
        SettingSchema::new(
            "ai.ollama_timeout_secs",
            "ai",
            "Ollama timeout",
            Integer,
            "120",
            false,
            &[],
        ),
        SettingSchema::new(
            "ai.openai_api_key",
            "ai",
            "OpenAI-compatible API key",
            Secret,
            "",
            true,
            &[],
        ),
        SettingSchema::new(
            "ai.openai_base_url",
            "ai",
            "OpenAI-compatible base URL",
            String,
            "https://api.openai.com/v1",
            false,
            &[],
        ),
        SettingSchema::new(
            "ai.openai_model",
            "ai",
            "OpenAI-compatible model",
            String,
            "gpt-4o-mini",
            false,
            &[],
        ),
        SettingSchema::new(
            "ai.max_tokens",
            "ai",
            "Max tokens",
            Integer,
            "4096",
            false,
            &[],
        ),
        SettingSchema::new(
            "ai.timeout_secs",
            "ai",
            "AI timeout",
            Integer,
            "30",
            false,
            &[],
        ),
        SettingSchema::new(
            "ai.stream_enabled",
            "ai",
            "Token streaming",
            Boolean,
            "true",
            false,
            &["true", "false"],
        ),
        SettingSchema::new(
            "translation.default_src",
            "translation",
            "Default source",
            String,
            "auto",
            false,
            &[],
        ),
        SettingSchema::new(
            "translation.default_dst",
            "translation",
            "Default target",
            String,
            "zh-TW",
            false,
            &[],
        ),
        SettingSchema::new(
            "translation.provider",
            "translation",
            "Provider",
            String,
            "google_cloud_v2",
            false,
            &["google_cloud_v2"],
        ),
        SettingSchema::new(
            "translation.api_key",
            "translation",
            "Google Cloud API key",
            Secret,
            "",
            true,
            &[],
        ),
        SettingSchema::new(
            "translation.timeout_secs",
            "translation",
            "Translation timeout",
            Integer,
            "30",
            false,
            &[],
        ),
        SettingSchema::new(
            "notes.storage_dir",
            "notes",
            "Storage directory",
            String,
            "",
            false,
            &[],
        ),
        SettingSchema::new(
            "notes.default_extension",
            "notes",
            "Default extension",
            String,
            "md",
            false,
            &[],
        ),
        SettingSchema::new(
            "history.max_items",
            "history",
            "Max items",
            Integer,
            "200",
            false,
            &[],
        ),
        SettingSchema::new(
            "history.ignore_duplicates",
            "history",
            "Ignore duplicates",
            Boolean,
            "true",
            false,
            &["true", "false"],
        ),
        SettingSchema::new(
            "system.enable_volume_control",
            "system",
            "Volume control",
            Boolean,
            "true",
            false,
            &["true", "false"],
        ),
        SettingSchema::new(
            "system.enable_brightness_control",
            "system",
            "Brightness control",
            Boolean,
            "true",
            false,
            &["true", "false"],
        ),
        SettingSchema::new(
            "system.enable_wifi_info",
            "system",
            "Wi-Fi info",
            Boolean,
            "true",
            false,
            &["true", "false"],
        ),
        SettingSchema::new(
            "performance.low_memory_mode",
            "performance",
            "Low memory mode",
            Boolean,
            "false",
            false,
            &["true", "false"],
        ),
        SettingSchema::new(
            "ai.ollama_keep_alive",
            "ai",
            "Ollama keep-alive",
            String,
            "5m",
            false,
            &[],
        ),
        SettingSchema::new(
            "security.network_allowlist",
            "security",
            "Outbound network allowlist",
            String,
            "api.anthropic.com,api.openai.com,translation.googleapis.com,translate.googleapis.com,api.tavily.com,duckduckgo.com,github.com",
            false,
            &[],
        ),
        SettingSchema::new(
            "agent.local_context.enabled",
            "agent",
            "Local context review (enabled)",
            Boolean,
            "false",
            false,
            &["true", "false"],
        ),
        SettingSchema::new(
            "agent.local_context.max_scan_files",
            "agent",
            "Local context max scan files",
            Integer,
            "500",
            false,
            &[],
        ),
        SettingSchema::new(
            "agent.local_context.max_preview_bytes",
            "agent",
            "Local context max preview bytes",
            Integer,
            "4096",
            false,
            &[],
        ),
        SettingSchema::new(
            "agent.local_context.max_depth",
            "agent",
            "Local context scan depth",
            Integer,
            "4",
            false,
            &[],
        ),
        SettingSchema::new(
            "agent.local_context.extra_denylist",
            "agent",
            "Local context extra denylist (comma-separated)",
            String,
            "",
            false,
            &[],
        ),
        // REF.7.A — fixes a REF.6.B latent bug where useLauncherSettings watched
        // this key but no schema row backed it. setting.list_all therefore could
        // never return it. Default true matches the in-code default consumed by
        // CapabilityHintLine.
        SettingSchema::new(
            "launcher.show_capability_hint",
            "launcher",
            "Show capability prefix hint above empty palette",
            Boolean,
            "true",
            false,
            &["true", "false"],
        ),
    ]
}

pub fn is_sensitive_key(key: &str) -> bool {
    builtin_setting_schema()
        .iter()
        .any(|schema| schema.key == key && schema.sensitive)
}

pub fn setting_schema_for_key(key: &str) -> Option<SettingSchema> {
    builtin_setting_schema()
        .into_iter()
        .find(|schema| schema.key == key)
}

pub fn is_protected_setting_key(key: &str) -> bool {
    key.starts_with("security.")
}

pub fn validate_user_setting_value(key: &str, value: &str) -> Result<(), String> {
    if is_protected_setting_key(key) {
        return Err(format!(
            "setting '{key}' is protected and cannot be changed through user-facing settings"
        ));
    }

    let schema =
        setting_schema_for_key(key).ok_or_else(|| format!("unknown setting key '{key}'"))?;
    let trimmed = value.trim();

    match schema.value_type {
        SettingValueType::Boolean => {
            if !matches!(trimmed.to_ascii_lowercase().as_str(), "true" | "false") {
                return Err(format!("setting '{key}' expects a boolean value"));
            }
        }
        SettingValueType::Integer => {
            if trimmed.parse::<i64>().is_err() {
                return Err(format!("setting '{key}' expects an integer value"));
            }
        }
        SettingValueType::String | SettingValueType::Hotkey | SettingValueType::Secret => {}
    }

    if !schema.options.is_empty() && !schema.options.contains(&trimmed) {
        return Err(format!(
            "setting '{key}' must be one of: {}",
            schema.options.join(", ")
        ));
    }

    // Command-path settings (`terminal.default_shell`) feed the program field of
    // a backend-issued TerminalLaunchSpec. Terminal launch is already gated to
    // backend-issued
    // specs, but reject shell metacharacters / control chars here as
    // defense-in-depth so a stored setting can't smuggle extra commands into any
    // current or future launch path. Spaces and quotes (paths) remain allowed.
    if is_command_setting_key(key) && contains_unsafe_command_chars(trimmed) {
        return Err(format!(
            "setting '{key}' must be a plain command or path without shell metacharacters"
        ));
    }

    Ok(())
}

fn is_command_setting_key(key: &str) -> bool {
    matches!(key, "terminal.default_shell")
}

fn contains_unsafe_command_chars(value: &str) -> bool {
    value
        .chars()
        .any(|c| c.is_control() || matches!(c, ';' | '&' | '|' | '<' | '>' | '`' | '$'))
}

/// Mask shown for a sensitive setting that HAS a stored value, so the UI can
/// tell "set" from "unset" without leaking the secret. An unset secret stays
/// empty. Mirrors the `setting.get` mask.
pub const SECRET_MASK: &str = "********";

pub fn redact_setting_value(key: &str, value: &str) -> String {
    if is_sensitive_key(key) && !value.is_empty() {
        SECRET_MASK.to_string()
    } else {
        value.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn user_setting_validation_rejects_security_keys() {
        let error = validate_user_setting_value(
            "security.network_allowlist",
            "evil.example.com,api.openai.com",
        )
        .expect_err("security settings are protected");

        assert!(error.contains("protected"));
    }

    #[test]
    fn user_setting_validation_rejects_unknown_keys() {
        let error = validate_user_setting_value("security_typo.network_allowlist", "value")
            .expect_err("unknown keys should not be persisted");

        assert!(error.contains("unknown setting key"));
    }

    #[test]
    fn user_setting_validation_checks_types_and_options() {
        assert!(validate_user_setting_value("features.ai", "maybe").is_err());
        assert!(validate_user_setting_value("launcher.max_results", "many").is_err());
        assert!(validate_user_setting_value("ai.provider", "evil").is_err());

        assert!(validate_user_setting_value("features.ai", "false").is_ok());
        assert!(validate_user_setting_value("launcher.max_results", "20").is_ok());
        assert!(validate_user_setting_value("ai.provider", "openai").is_ok());
    }
}
