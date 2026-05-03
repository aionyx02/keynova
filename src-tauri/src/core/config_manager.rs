use std::collections::HashMap;
use std::path::PathBuf;

/// 讀寫 %APPDATA%\Keynova\config.toml，回退至 default_config.toml。
/// 值以字串形式暴露給外部，但 persist() 時會偵測型別寫出正確 TOML 格式。
pub struct ConfigManager {
    data: HashMap<String, String>,
    config_path: PathBuf,
}

impl ConfigManager {
    pub fn new() -> Self {
        let config_path = Self::user_config_path();
        let data = Self::load(&config_path);
        Self { data, config_path }
    }

    fn user_config_path() -> PathBuf {
        let base = std::env::var("APPDATA").unwrap_or_else(|_| ".".into());
        PathBuf::from(base).join("Keynova").join("config.toml")
    }

    fn load(path: &PathBuf) -> HashMap<String, String> {
        if let Ok(content) = std::fs::read_to_string(path) {
            if let Ok(table) = content.parse::<toml::Table>() {
                return flatten_table(&table, "");
            }
        }
        let default_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("default_config.toml");
        if let Ok(content) = std::fs::read_to_string(&default_path) {
            if let Ok(table) = content.parse::<toml::Table>() {
                return flatten_table(&table, "");
            }
        }
        HashMap::new()
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
        let mut pairs: Vec<_> = self.data.iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        pairs.sort_by(|a, b| a.0.cmp(&b.0));
        pairs
    }

    /// 以 TOML section 格式寫回磁碟，保留數字/bool 的正確型別。
    fn persist(&self) -> Result<(), String> {
        let dir = self.config_path.parent()
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
                sections.entry(section).or_default().push((field, v.clone()));
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
            if !out.is_empty() { out.push('\n'); }
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
            toml::Value::String(s) => { map.insert(full_key, s.clone()); }
            toml::Value::Integer(i) => { map.insert(full_key, i.to_string()); }
            toml::Value::Float(f) => { map.insert(full_key, f.to_string()); }
            toml::Value::Boolean(b) => { map.insert(full_key, b.to_string()); }
            _ => {}
        }
    }
    map
}