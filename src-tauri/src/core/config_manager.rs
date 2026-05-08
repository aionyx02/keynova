use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use serde::Serialize;

use crate::models::settings_schema::{
    builtin_setting_schema, is_sensitive_key, redact_setting_value, SettingSchema,
};

#[derive(Clone, Debug, Serialize)]
pub struct ConfigChange {
    pub key: String,
    pub old: Option<String>,
    pub new: Option<String>,
}

/// 讀寫 %APPDATA%\Keynova\config.toml，回退至 default_config.toml。
/// 值以字串形式暴露給外部，但 persist() 時會偵測型別寫出正確 TOML 格式。
pub struct ConfigManager {
    data: HashMap<String, String>,
    config_path: PathBuf,
}

impl ConfigManager {
    pub fn new() -> Self {
        let config_path = Self::user_config_path();
        let data = Self::load_result(&config_path).unwrap_or_else(|e| {
            eprintln!("[keynova] config load failed: {e}");
            Self::load_default_result().unwrap_or_default()
        });
        Self { data, config_path }
    }

    pub fn user_config_path() -> PathBuf {
        let base = std::env::var("APPDATA").unwrap_or_else(|_| ".".into());
        PathBuf::from(base).join("Keynova").join("config.toml")
    }

    fn load_result(path: &PathBuf) -> Result<HashMap<String, String>, String> {
        if path.exists() {
            let content = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
            let table = content
                .parse::<toml::Table>()
                .map_err(|e| format!("{}: {e}", path.display()))?;
            return Ok(flatten_table(&table, ""));
        }
        Self::load_default_result()
    }

    fn load_default_result() -> Result<HashMap<String, String>, String> {
        let default_path = Self::default_config_path();
        let content = std::fs::read_to_string(&default_path).map_err(|e| e.to_string())?;
        let table = content
            .parse::<toml::Table>()
            .map_err(|e| format!("{}: {e}", default_path.display()))?;
        Ok(flatten_table(&table, ""))
    }

    fn default_config_path() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("default_config.toml")
    }

    pub fn config_path(&self) -> PathBuf {
        self.config_path.clone()
    }

    pub fn snapshot(&self) -> HashMap<String, String> {
        self.data.clone()
    }

    pub fn diff(old: &HashMap<String, String>, new: &HashMap<String, String>) -> Vec<ConfigChange> {
        let keys: HashSet<String> = old.keys().chain(new.keys()).cloned().collect();
        let mut changes: Vec<_> = keys
            .into_iter()
            .filter_map(|key| {
                let old_value = old.get(&key).cloned();
                let new_value = new.get(&key).cloned();
                (old_value != new_value).then_some(ConfigChange {
                    key,
                    old: old_value,
                    new: new_value,
                })
            })
            .collect();
        changes.sort_by(|a, b| a.key.cmp(&b.key));
        changes
    }

    pub fn reload_from_disk(&mut self) -> Result<Vec<ConfigChange>, String> {
        let old = self.snapshot();
        let new = Self::load_result(&self.config_path)?;
        let changes = Self::diff(&old, &new);
        self.data = new;
        Ok(changes)
    }

    pub fn get(&self, key: &str) -> Option<String> {
        self.data.get(key).cloned()
    }

    /// 更新設定值並寫回磁碟（僅在失焦/儲存按鈕時呼叫）。
    pub fn set(&mut self, key: &str, value: &str) -> Result<(), String> {
        self.data.insert(key.to_string(), value.to_string());
        self.persist()
    }

    pub fn list_all(&self) -> Vec<(String, String)> {
        let mut pairs: Vec<_> = self
            .data
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        pairs.sort_by(|a, b| a.0.cmp(&b.0));
        pairs
    }

    pub fn list_all_redacted(&self) -> Vec<(String, String, bool)> {
        let mut pairs: Vec<_> = self
            .data
            .iter()
            .map(|(k, v)| (k.clone(), redact_setting_value(k, v), is_sensitive_key(k)))
            .collect();
        pairs.sort_by(|a, b| a.0.cmp(&b.0));
        pairs
    }

    pub fn schema(&self) -> Vec<SettingSchema> {
        builtin_setting_schema()
    }

    /// 以 TOML section 格式寫回磁碟，保留數字/bool 的正確型別。
    fn persist(&self) -> Result<(), String> {
        let dir = self
            .config_path
            .parent()
            .ok_or_else(|| "invalid config path".to_string())?;
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;

        // 重建 section → (key, value) 的有序結構
        let mut sections: std::collections::BTreeMap<String, Vec<(String, String)>> =
            std::collections::BTreeMap::new();
        let mut root: Vec<(String, String)> = Vec::new();

        for (k, v) in &self.data {
            if let Some(dot) = k.find('.') {
                let section = k[..dot].to_string();
                let field = k[dot + 1..].to_string();
                sections
                    .entry(section)
                    .or_default()
                    .push((field, v.clone()));
            } else {
                root.push((k.clone(), v.clone()));
            }
        }
        root.sort();
        for vals in sections.values_mut() {
            vals.sort();
        }

        let mut out = String::new();
        for (k, v) in &root {
            out.push_str(&format!("{} = {}\n", k, toml_value_str(v)));
        }
        for (section, vals) in &sections {
            if !out.is_empty() {
                out.push('\n');
            }
            out.push_str(&format!("[{}]\n", section));
            for (field, v) in vals {
                out.push_str(&format!("{} = {}\n", field, toml_value_str(v)));
            }
        }

        std::fs::write(&self.config_path, out).map_err(|e| e.to_string())
    }
}

impl Default for ConfigManager {
    fn default() -> Self {
        Self::new()
    }
}

/// 根據字串內容決定 TOML 值的表示方式（數字/bool 不加引號）。
fn toml_value_str(s: &str) -> String {
    if s == "true" || s == "false" {
        return s.to_string();
    }
    if s.parse::<i64>().is_ok() {
        return s.to_string();
    }
    if s.parse::<f64>().is_ok() {
        return s.to_string();
    }
    format!("{:?}", s) // adds surrounding double-quotes and escapes
}

fn flatten_table(table: &toml::Table, prefix: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for (key, value) in table {
        let full_key = if prefix.is_empty() {
            key.clone()
        } else {
            format!("{}.{}", prefix, key)
        };
        match value {
            toml::Value::Table(t) => map.extend(flatten_table(t, &full_key)),
            toml::Value::String(s) => {
                map.insert(full_key, s.clone());
            }
            toml::Value::Integer(i) => {
                map.insert(full_key, i.to_string());
            }
            toml::Value::Float(f) => {
                map.insert(full_key, f.to_string());
            }
            toml::Value::Boolean(b) => {
                map.insert(full_key, b.to_string());
            }
            _ => {}
        }
    }
    map
}
